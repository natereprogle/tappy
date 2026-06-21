mod app_state;
mod notes;
mod propresenter;
mod redirect_queue;
mod secrets;
mod settings;
mod worker_client;

use app_state::{AppState, AppStateInner};
use propresenter::{start_propresenter_listener, stop_propresenter_listener};
use secrets::save_admin_token;
use settings::{load_settings, save_settings};
use std::sync::Arc;
use tauri::{Emitter, State};
use worker_client::{
    create_or_update_master_link, open_current_shortlink, update_link_tags, WorkerConfig, check_slug_exists,
    deploy_cloudflare_worker, list_cloudflare_zones, download_slug_config, rotate_worker_token,
};

#[tauri::command]
async fn configure_worker(
    state: State<'_, AppState>,
    base_url: String,
    admin_token: String,
    slug: String,
) -> Result<(), String> {
    // Persist to OS keychain before updating in-memory state
    save_admin_token(&admin_token)?;

    let config = WorkerConfig {
        base_url,
        admin_token,
        slug,
    };

    let mut guard = state.worker.write().await;
    *guard = Some(config);

    Ok(())
}

#[tauri::command]
async fn get_worker_config(state: State<'_, AppState>) -> Result<Option<WorkerConfig>, String> {
    let guard = state.worker.read().await;
    Ok(guard.clone())
}

#[tauri::command]
async fn parse_slide_notes(notes: String) -> Result<notes::ParsedNotes, String> {
    Ok(notes::parse_notes(&notes))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state: AppState = Arc::new(AppStateInner::default());
    let state_clone = Arc::clone(&state);

    tauri::Builder::default()
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            configure_worker,
            get_worker_config,
            parse_slide_notes,
            save_settings,
            load_settings,
            update_link_tags,
            create_or_update_master_link,
            open_current_shortlink,
            start_propresenter_listener,
            stop_propresenter_listener,
            check_slug_exists,
            deploy_cloudflare_worker,
            list_cloudflare_zones,
            download_slug_config,
            rotate_worker_token,
        ])
        .setup(move |app| {
            state_clone
                .redirect_queue
                .clone()
                .start(app.handle().clone(), Arc::clone(&state_clone));

            app.handle()
                .emit("app:ready", serde_json::json!({ "ready": true }))
                .ok();

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}