use crate::app_state::AppState;
use crate::secrets::load_admin_token;
use crate::worker_client::WorkerConfig;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_store::StoreExt;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppSettings {
    pub propresenter_host: String,
    pub propresenter_port: u16,
    pub propresenter_endpoint: String,
    pub worker_base_url: String,
    pub worker_slug: String,
    pub minimum_kv_write_spacing_ms: u64,
    pub default_redirect_url: String,
    pub link_owner_id: String,
    pub tag_mappings: std::collections::HashMap<String, String>,
}

impl AppSettings {
    pub fn with_defaults(mut self) -> Self {
        if self.propresenter_host.trim().is_empty() {
            self.propresenter_host = "127.0.0.1".to_string();
        }

        if self.propresenter_port == 0 {
            self.propresenter_port = 1025;
        }

        if self.propresenter_endpoint.trim().is_empty() {
            self.propresenter_endpoint = "v1/status/updates".to_string();
        }

        if self.minimum_kv_write_spacing_ms == 0 {
            self.minimum_kv_write_spacing_ms = 1_000;
        }

        if self.default_redirect_url.trim().is_empty() {
            self.default_redirect_url = "https://example.com".to_string();
        }

        self
    }
}

const SETTINGS_FILE: &str = "settings.json";
const SETTINGS_KEY: &str = "app_settings";

#[tauri::command]
pub async fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> Result<(), String> {
    let store = app
        .store(SETTINGS_FILE)
        .map_err(|err| format!("Failed to open settings store: {err}"))?;

    let settings = settings.with_defaults();

    // Synchronize spacing with global redirect queue
    state.redirect_queue.set_min_spacing(settings.minimum_kv_write_spacing_ms);

    store.set(
        SETTINGS_KEY,
        serde_json::to_value(&settings).map_err(|err| err.to_string())?,
    );

    store
        .save()
        .map_err(|err| format!("Failed to save settings: {err}"))?;

    Ok(())
}

#[tauri::command]
pub async fn load_settings(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppSettings, String> {
    let store = app
        .store(SETTINGS_FILE)
        .map_err(|err| format!("Failed to open settings store: {err}"))?;

    let settings = match store.get(SETTINGS_KEY) {
        Some(value) => serde_json::from_value::<AppSettings>(value.clone())
            .map(AppSettings::with_defaults)
            .map_err(|err| format!("Failed to parse settings: {err}"))?,
        None => AppSettings::default().with_defaults(),
    };

    // Synchronize spacing with global redirect queue
    state.redirect_queue.set_min_spacing(settings.minimum_kv_write_spacing_ms);

    // Restore the admin token from the OS keychain into AppState so the app
    // is fully operational after a restart without user intervention.
    if !settings.worker_base_url.is_empty() && !settings.worker_slug.is_empty() {
        match load_admin_token() {
            Ok(Some(token)) => {
                let config = WorkerConfig {
                    base_url: settings.worker_base_url.clone(),
                    admin_token: token,
                    slug: settings.worker_slug.clone(),
                };
                let mut guard = state.worker.write().await;
                *guard = Some(config);
            }
            Ok(None) => {
                // No token stored yet — user will need to go through onboarding
                // or a password rotation to re-establish the credential.
            }
            Err(e) => {
                // Non-fatal: log to stderr, proceed without token
                eprintln!("[tappy] Warning: could not read admin token from keychain: {e}");
            }
        }
    }

    Ok(settings)
}