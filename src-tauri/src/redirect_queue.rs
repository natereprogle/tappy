use crate::{
    app_state::AppState,
    worker_client::set_current_tag,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};
use tokio::{
    sync::{Mutex, Notify},
    time::sleep,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedirectUpdate {
    pub tag: String,
    pub seq: u64,
    pub reason: RedirectUpdateReason,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RedirectUpdateReason {
    SlideTag,
    VideoTimestamp {
        seconds: u64,
        raw_time: String,
    },
    Manual,
}

#[derive(Clone)]
pub struct KvRedirectQueue {
    queue: Arc<Mutex<VecDeque<RedirectUpdate>>>,
    notify: Arc<Notify>,
    min_spacing_ms: Arc<AtomicU64>,
}

impl KvRedirectQueue {
    pub fn new(min_spacing_ms: u64) -> Self {
        Self {
            queue: Arc::new(Mutex::new(VecDeque::new())),
            notify: Arc::new(Notify::new()),
            min_spacing_ms: Arc::new(AtomicU64::new(min_spacing_ms)),
        }
    }

    pub fn set_min_spacing(&self, ms: u64) {
        self.min_spacing_ms.store(ms, Ordering::SeqCst);
    }

    pub fn get_min_spacing(&self) -> Duration {
        Duration::from_millis(self.min_spacing_ms.load(Ordering::SeqCst))
    }

    pub async fn enqueue(&self, update: RedirectUpdate) {
        let mut queue = self.queue.lock().await;

        queue.retain(|existing| existing.seq > update.seq);
        queue.push_back(update);

        self.notify.notify_one();
    }

    #[allow(dead_code)]
    pub async fn len(&self) -> usize {
        self.queue.lock().await.len()
    }

    pub fn start(self, app: AppHandle, state: AppState) {
        tauri::async_runtime::spawn(async move {
            let mut last_write = Instant::now()
                .checked_sub(self.get_min_spacing())
                .unwrap_or_else(Instant::now);

            loop {
                let update = {
                    let mut queue = self.queue.lock().await;
                    queue.pop_front()
                };

                let Some(update) = update else {
                    self.notify.notified().await;
                    continue;
                };

                let min_spacing = self.get_min_spacing();
                let elapsed = last_write.elapsed();

                if elapsed < min_spacing {
                    sleep(min_spacing - elapsed).await;
                }

                // Retrieve the latest worker config dynamically
                let worker = {
                    let guard = state.worker.read().await;
                    guard.clone()
                };

                let Some(worker) = worker else {
                    app.emit(
                        "worker:not-configured",
                        serde_json::json!({
                            "message": "Worker is not configured when processing queued redirect",
                        }),
                    )
                    .ok();
                    continue;
                };

                app.emit("redirect:write-started", &update).ok();

                match set_current_tag(&worker, &update.tag, update.seq).await {
                    Ok(_) => {
                        last_write = Instant::now();

                        app.emit(
                            "redirect:write-succeeded",
                            serde_json::json!({
                                "tag": update.tag,
                                "seq": update.seq,
                                "reason": update.reason,
                            }),
                        )
                        .ok();
                    }
                    Err(err) => {
                        app.emit(
                            "redirect:write-failed",
                            serde_json::json!({
                                "tag": update.tag,
                                "seq": update.seq,
                                "reason": update.reason,
                                "error": err,
                            }),
                        )
                        .ok();
                    }
                }
            }
        });
    }
}