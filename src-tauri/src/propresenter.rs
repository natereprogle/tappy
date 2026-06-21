use crate::{
    app_state::AppState,
    notes::{parse_notes, ParsedNotes},
    redirect_queue::{RedirectUpdate, RedirectUpdateReason},
    worker_client::set_current_tag,
};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tauri::{AppHandle, Emitter, State};
use tokio::time::sleep;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProPresenterConfig {
    pub host: String,
    pub port: u16,
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlideUpdate {
    pub generation: u64,
    pub notes: String,
    pub parsed: ParsedNotes,
    pub media_started: bool,
}

#[tauri::command]
pub async fn start_propresenter_listener(
    app: AppHandle,
    state: State<'_, AppState>,
    host: String,
    port: u16,
    endpoint: Option<String>,
) -> Result<(), String> {
    let endpoint = endpoint.unwrap_or_else(|| "v1/status/updates".to_string());

    let config = ProPresenterConfig {
        host,
        port,
        endpoint,
    };

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();

    {
        let mut guard = state.listener_cancel.write().await;
        if let Some(old_tx) = guard.replace(tx) {
            let _ = old_tx.send(());
        }
    }

    let state_clone = state.inner().clone();
    let app_clone = app.clone();

    tauri::async_runtime::spawn(async move {
        let res = listen_to_propresenter(app_clone.clone(), state_clone.clone(), config, rx).await;

        if let Err(err) = res {
            app_clone.emit(
                "propresenter:error",
                json!({
                    "error": err.to_string(),
                }),
            )
            .ok();
        } else {
            app_clone.emit("propresenter:disconnected", json!({ "disconnected": true })).ok();
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn stop_propresenter_listener(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut guard = state.listener_cancel.write().await;
    if let Some(cancel_tx) = guard.take() {
        let _ = cancel_tx.send(());
    }

    Ok(())
}

async fn listen_to_propresenter(
    app: AppHandle,
    state: AppState,
    config: ProPresenterConfig,
    mut rx: tokio::sync::oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3600 * 24)) // Extremely long timeout for connection streaming
        .build()?;

    let mut endpoint = config.endpoint.trim();
    if endpoint.starts_with('/') {
        endpoint = &endpoint[1..];
    }

    let url = format!(
        "http://{}:{}/{}?chunked=true",
        config.host, config.port, endpoint
    );

    let request_builder = if endpoint.contains("status/updates") {
        client.post(&url).json(&vec![
            "status/slide",
            "presentation/active",
            "presentation/current"
        ])
    } else {
        client.get(&url)
    };

    let mut response = tokio::select! {
        _ = &mut rx => {
            return Ok(());
        }
        res = request_builder.send() => {
            res.with_context(|| {
                format!(
                    "Failed to connect to ProPresenter HTTP API at http://{}:{}",
                    config.host, config.port
                )
            })?
        }
    };

    if !response.status().is_success() {
        let status = response.status();
        return Err(anyhow::anyhow!(
            "ProPresenter HTTP API returned error status: {}",
            status
        ));
    }

    app.emit(
        "propresenter:connected",
        json!({
            "host": config.host,
            "port": config.port,
            "endpoint": config.endpoint,
        }),
    )
    .ok();

    let mut buffer = Vec::new();

    loop {
        let chunk_opt = tokio::select! {
            _ = &mut rx => {
                return Ok(());
            }
            res = response.chunk() => {
                res.context("Failed reading chunk from ProPresenter")?
            }
        };
        let Some(chunk) = chunk_opt else {
            break;
        };

        buffer.extend_from_slice(&chunk);

        while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
            let mut line_bytes = buffer.drain(..=pos).collect::<Vec<u8>>();
            if line_bytes.ends_with(&[b'\n']) {
                line_bytes.pop();
            }
            if line_bytes.ends_with(&[b'\r']) {
                line_bytes.pop();
            }

            let line = String::from_utf8_lossy(&line_bytes).trim().to_string();
            if line.is_empty() {
                continue;
            }

            let update: Value = match serde_json::from_str(&line) {
                Ok(value) => value,
                Err(err) => {
                    app.emit(
                        "propresenter:invalid-json",
                        json!({
                            "line": line,
                            "error": err.to_string(),
                        }),
                    )
                    .ok();

                    continue;
                }
            };

            // If the update has a "data" wrapper (from status/updates), unpack it
            let data = if let Some(inner_data) = update.get("data") {
                inner_data
            } else {
                &update
            };

            let notes_opt = extract_notes_from_update(data);
            let media_started = extract_media_started(data);

            if let Some(notes) = notes_opt {
                let parsed = parse_notes(&notes);
                let generation = state.next_generation();

                // Cache the parsed notes of the active slide
                {
                    let mut guard = state.current_notes.write().await;
                    *guard = Some(parsed.clone());
                }

                let slide_update = SlideUpdate {
                    generation,
                    notes,
                    parsed: parsed.clone(),
                    media_started,
                };

                app.emit("propresenter:slide-update", &slide_update).ok();

                if parsed.regular_tags.is_empty() && parsed.timed_tags.is_empty() {
                    continue;
                }

                apply_parsed_tags(
                    app.clone(),
                    state.clone(),
                    generation,
                    parsed,
                )
                .await;
            } else if media_started {
                // Media started, but there are no slide notes in this update.
                // Let's check our cached notes!
                let cached_parsed = {
                    let guard = state.current_notes.read().await;
                    guard.clone()
                };

                if let Some(parsed) = cached_parsed {
                    if parsed.has_video_tags() {
                        let current_gen = state.current_generation();

                        if state
                            .media_scheduled_generation
                            .load(std::sync::atomic::Ordering::SeqCst)
                            != current_gen
                        {
                            state.media_scheduled_generation.store(
                                current_gen,
                                std::sync::atomic::Ordering::SeqCst,
                            );

                            app.emit(
                                "propresenter:media-started-belated",
                                json!({
                                    "generation": current_gen,
                                    "message": "Media started after slide change. Scheduling video tags now.",
                                }),
                            )
                            .ok();

                            schedule_video_tags(
                                app.clone(),
                                state.clone(),
                                current_gen,
                                parsed,
                            )
                            .await;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

async fn apply_parsed_tags(
    app: AppHandle,
    state: AppState,
    generation: u64,
    parsed: ParsedNotes,
) {
    let worker = {
        let guard = state.worker.read().await;
        guard.clone()
    };

    let Some(worker) = worker else {
        app.emit(
            "worker:not-configured",
            json!({
                "message": "Worker is not configured",
            }),
        )
            .ok();

        return;
    };

    if let Some(first_regular_tag) = parsed.first_regular_tag() {
        let seq = generation << 32;
        match set_current_tag(&worker, first_regular_tag, seq).await {
            Ok(_) => {
                app.emit(
                    "redirect:regular-tag-applied",
                    json!({
                        "tag": first_regular_tag,
                        "generation": generation,
                    }),
                )
                    .ok();
            }
            Err(err) => {
                app.emit(
                    "redirect:regular-tag-failed",
                    json!({
                        "tag": first_regular_tag,
                        "generation": generation,
                        "error": err,
                    }),
                )
                    .ok();
            }
        }
    }

    if parsed.has_video_tags() {
        state.media_scheduled_generation.store(generation, std::sync::atomic::Ordering::SeqCst);
        schedule_video_tags(app, state, generation, parsed).await;
    }
}

async fn schedule_video_tags(
    app: AppHandle,
    state: AppState,
    generation: u64,
    parsed: ParsedNotes,
) {
    for timed_tag in parsed.timed_tags {
        let app = app.clone();
        let state = Arc::clone(&state);

        tauri::async_runtime::spawn(async move {
            sleep(Duration::from_secs(timed_tag.seconds)).await;

            if state.current_generation() != generation {
                app.emit(
                    "redirect:timestamp-cancelled",
                    json!({
                        "tag": timed_tag.keyword,
                        "seconds": timed_tag.seconds,
                        "rawTime": timed_tag.raw_time,
                        "generation": generation,
                        "reason": "stale generation",
                    }),
                )
                    .ok();

                return;
            }

            let seq = (generation << 32) | (timed_tag.seconds & 0xFFFFFFFF);
            state
                .redirect_queue
                .enqueue(RedirectUpdate {
                    tag: timed_tag.keyword.clone(),
                    seq,
                    reason: RedirectUpdateReason::VideoTimestamp {
                        seconds: timed_tag.seconds,
                        raw_time: timed_tag.raw_time.clone(),
                    },
                })
                .await;

            app.emit(
                "redirect:timestamp-queued",
                json!({
                    "tag": timed_tag.keyword,
                    "seconds": timed_tag.seconds,
                    "rawTime": timed_tag.raw_time,
                    "generation": generation,
                }),
            )
                .ok();
        });
    }
}

fn extract_notes_from_update(update: &Value) -> Option<String> {
    update
        .pointer("/current/notes")
        .or_else(|| update.pointer("/presentation/current/notes"))
        .or_else(|| update.pointer("/presentation/notes"))
        .or_else(|| update.pointer("/slide/notes"))
        .or_else(|| update.pointer("/current/slide/notes"))
        .or_else(|| update.pointer("/notes"))
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn extract_media_started(update: &Value) -> bool {
    let media_state = update
        .pointer("/media/state")
        .or_else(|| update.pointer("/presentation/current/media/state"))
        .or_else(|| update.pointer("/current/media/state"))
        .and_then(Value::as_str)
        .unwrap_or_default();

    matches!(
        media_state.to_lowercase().as_str(),
        "playing" | "started" | "running"
    )
}