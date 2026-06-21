use crate::app_state::AppState;
use crate::notes::normalize_keyword;
use crate::secrets::save_admin_token;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use tauri::{AppHandle, State};
use reqwest::Client;
use reqwest::multipart;
use tauri_plugin_opener::OpenerExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    pub base_url: String,
    pub admin_token: String,
    pub slug: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterLinkRequest {
    pub default_url: String,
    pub owner_id: String,
    pub tags: HashMap<String, String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerCurrentResponse {
    pub ok: bool,
    pub ignored: Option<bool>,
    pub reason: Option<String>,
    pub current_seq: Option<u64>,
}

#[tauri::command]
pub async fn create_or_update_master_link(
    state: State<'_, AppState>,
    default_url: String,
    owner_id: String,
    tags: HashMap<String, String>,
) -> Result<(), String> {
    let worker = get_worker_config_from_state(&state).await?;

    if !is_http_url(&default_url) {
        return Err("Default URL must start with http:// or https://".to_string());
    }

    let normalized_tags = normalize_tags(tags);

    let url = format!(
        "{}/api/links/{}",
        worker.base_url.trim_end_matches('/'),
        worker.slug
    );

    let response = Client::new()
        .put(url)
        .bearer_auth(&worker.admin_token)
        .json(&json!({
            "defaultUrl": default_url,
            "ownerId": owner_id,
            "tags": normalized_tags,
        }))
        .send()
        .await
        .map_err(|err| format!("Failed to reach Worker: {err}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        return Err(format!("Worker rejected master link update: {status} {body}"));
    }

    Ok(())
}

#[tauri::command]
pub async fn update_link_tags(
    state: State<'_, AppState>,
    tags: HashMap<String, String>,
) -> Result<(), String> {
    let worker = get_worker_config_from_state(&state).await?;
    update_tags_with_config(&worker, tags).await
}

pub async fn update_tags_with_config(
    worker: &WorkerConfig,
    tags: HashMap<String, String>,
) -> Result<(), String> {
    let normalized_tags = normalize_tags(tags);

    let url = format!(
        "{}/api/links/{}/tags",
        worker.base_url.trim_end_matches('/'),
        worker.slug
    );

    let response = Client::new()
        .patch(url)
        .bearer_auth(&worker.admin_token)
        .json(&json!({
            "tags": normalized_tags,
        }))
        .send()
        .await
        .map_err(|err| format!("Failed to reach Worker: {err}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        return Err(format!("Worker rejected tag update: {status} {body}"));
    }

    Ok(())
}

pub async fn set_current_tag(worker: &WorkerConfig, tag: &str, seq: u64) -> Result<(), String> {
    let normalized_tag = normalize_keyword(tag);

    if normalized_tag.is_empty() {
        return Err("Tag cannot be empty".to_string());
    }

    let url = format!(
        "{}/api/links/{}/current",
        worker.base_url.trim_end_matches('/'),
        worker.slug
    );

    let response = Client::new()
        .patch(url)
        .bearer_auth(&worker.admin_token)
        .json(&json!({
            "tag": normalized_tag,
            "seq": seq,
        }))
        .send()
        .await
        .map_err(|err| format!("Failed to reach Worker: {err}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        return Err(format!("Worker rejected current tag update: {status} {body}"));
    }

    Ok(())
}

#[allow(dead_code)]
pub async fn set_current_url(worker: &WorkerConfig, url_to_set: &str, seq: u64) -> Result<(), String> {
    if !is_http_url(url_to_set) {
        return Err("Redirect URL must start with http:// or https://".to_string());
    }

    let url = format!(
        "{}/api/links/{}/current",
        worker.base_url.trim_end_matches('/'),
        worker.slug
    );

    let response = Client::new()
        .patch(url)
        .bearer_auth(&worker.admin_token)
        .json(&json!({
            "url": url_to_set,
            "seq": seq,
        }))
        .send()
        .await
        .map_err(|err| format!("Failed to reach Worker: {err}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();

        return Err(format!("Worker rejected current URL update: {status} {body}"));
    }

    Ok(())
}

#[tauri::command]
pub async fn open_current_shortlink(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let worker = get_worker_config_from_state(&state).await?;

    let shortlink = format!(
        "{}/{}",
        worker.base_url.trim_end_matches('/'),
        worker.slug
    );

    app.opener()
        .open_url(shortlink, None::<&str>)
        .map_err(|err| format!("Failed to open shortlink: {err}"))?;

    Ok(())
}

pub async fn get_worker_config_from_state(
    state: &State<'_, AppState>,
) -> Result<WorkerConfig, String> {
    let guard = state.worker.read().await;

    guard
        .clone()
        .ok_or_else(|| "Worker is not configured".to_string())
}

fn normalize_tags(tags: HashMap<String, String>) -> HashMap<String, String> {
    tags.into_iter()
        .map(|(key, value)| (normalize_keyword(&key), value.trim().to_string()))
        .filter(|(key, value)| !key.is_empty() && is_http_url(value))
        .collect()
}

pub fn is_http_url(value: &str) -> bool {
    value.starts_with("https://") || value.starts_with("http://")
}

#[tauri::command]
pub async fn check_slug_exists(
    base_url: String,
    admin_token: String,
    slug: String,
) -> Result<bool, String> {
    let url = format!(
        "{}/api/links/{}",
        base_url.trim_end_matches('/'),
        slug.trim()
    );

    let response = Client::new()
        .get(url)
        .bearer_auth(&admin_token)
        .send()
        .await
        .map_err(|err| format!("Failed to reach Worker: {err}"))?;

    if response.status() == 200 {
        Ok(true)
    } else if response.status() == 404 {
        Ok(false)
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("Worker returned status {status}: {body}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteSlugConfig {
    #[serde(rename = "defaultUrl")]
    pub default_url: String,
    #[serde(rename = "ownerId")]
    pub owner_id: String,
}

#[tauri::command]
pub async fn download_slug_config(
    base_url: String,
    admin_token: String,
    slug: String,
) -> Result<Option<RemoteSlugConfig>, String> {
    let url = format!(
        "{}/api/links/{}",
        base_url.trim_end_matches('/'),
        slug.trim()
    );

    let response = Client::new()
        .get(url)
        .bearer_auth(&admin_token)
        .send()
        .await
        .map_err(|err| format!("Failed to reach Worker: {err}"))?;

    if response.status() == 200 {
        let config: RemoteSlugConfig = response.json().await
            .map_err(|err| format!("Failed to parse slug config: {err}"))?;
        Ok(Some(config))
    } else if response.status() == 404 {
        Ok(None)
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        Err(format!("Worker returned status {status}: {body}"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudflareDeployResult {
    pub base_url: String,
    pub admin_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneInfo {
    pub id: String,
    pub name: String,
}

#[tauri::command]
pub async fn list_cloudflare_zones(
    api_token: String,
) -> Result<Vec<ZoneInfo>, String> {
    let client = Client::new();
    let url = "https://api.cloudflare.com/client/v4/zones";
    let resp = client.get(url)
        .bearer_auth(&api_token)
        .send()
        .await
        .map_err(|err| format!("Failed to fetch zones: {err}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Cloudflare API returned error listing zones: {status} {body}"));
    }

    let result_json: serde_json::Value = resp.json().await
        .map_err(|err| format!("Failed to parse zone list response: {err}"))?;

    let zones = result_json.pointer("/result").and_then(|v| v.as_array())
        .ok_or_else(|| "No zones found in Cloudflare account".to_string())?;

    let mut list = Vec::new();
    for z in zones {
        let id = z.pointer("/id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let name = z.pointer("/name").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        if !id.is_empty() && !name.is_empty() {
            list.push(ZoneInfo { id, name });
        }
    }

    Ok(list)
}

fn generate_admin_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let start = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
    format!("tappy_adm_{:x}", start)
}

async fn setup_worker_routing(
    client: &Client,
    api_token: &str,
    account_id: &str,
    zone_id: &str,
    zone_name: &str,
    subdomain: Option<String>,
    path: Option<String>,
) -> Result<String, String> {
    if let Some(sub) = subdomain {
        let clean_sub = sub.trim().trim_matches('.');
        let hostname = format!("{}.{}", clean_sub, zone_name);
        
        let url = format!(
            "https://api.cloudflare.com/client/v4/accounts/{}/workers/domains",
            account_id
        );
        
        let resp = client.put(&url)
            .bearer_auth(api_token)
            .json(&serde_json::json!({
                "zone_id": zone_id,
                "zone_name": zone_name,
                "hostname": hostname,
                "service": "tappy-redirects",
                "environment": "production"
            }))
            .send()
            .await
            .map_err(|err| format!("Failed to connect to Cloudflare Worker Custom Domain API: {err}"))?;
            
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Cloudflare custom domain mapping failed: {status} {body}"));
        }
        
        Ok(format!("https://{}", hostname))
    } else if let Some(p) = path {
        let clean_path = p.trim().trim_matches('/');
        let pattern = format!("{}/{}/*", zone_name, clean_path);
        
        let url = format!(
            "https://api.cloudflare.com/client/v4/zones/{}/workers/routes",
            zone_id
        );
        
        let resp = client.post(&url)
            .bearer_auth(api_token)
            .json(&serde_json::json!({
                "pattern": pattern,
                "script": "tappy-redirects"
            }))
            .send()
            .await
            .map_err(|err| format!("Failed to connect to Cloudflare Worker Routes API: {err}"))?;
            
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Cloudflare route mapping failed: {status} {body}"));
        }
        
        Ok(format!("https://{}/{}", zone_name, clean_path))
    } else {
        Err("Either subdomain or path routing parameter must be configured".to_string())
    }
}

#[tauri::command]
pub async fn deploy_cloudflare_worker(
    account_id: String,
    api_token: String,
    zone_id: String,
    zone_name: String,
    subdomain: Option<String>,
    path: Option<String>,
) -> Result<CloudflareDeployResult, String> {
    let client = Client::new();
    let admin_token = generate_admin_token();

    // 1. Fetch KV Namespaces to check if tappy-redirects-kv already exists
    let list_url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{}/storage/kv/namespaces",
        account_id
    );

    let list_resp = client
        .get(&list_url)
        .bearer_auth(&api_token)
        .send()
        .await
        .map_err(|err| format!("Failed to connect to Cloudflare: {err}"))?;

    if !list_resp.status().is_success() {
        let status = list_resp.status();
        let body = list_resp.text().await.unwrap_or_default();
        return Err(format!("Cloudflare API returned error listing KV: {status} {body}"));
    }

    let list_json: serde_json::Value = list_resp
        .json()
        .await
        .map_err(|err| format!("Failed to parse KV namespace list response: {err}"))?;

    let mut kv_namespace_id = None;
    if let Some(namespaces) = list_json.pointer("/result").and_then(|v| v.as_array()) {
        for ns in namespaces {
            if ns.pointer("/title").and_then(|v| v.as_str()) == Some("tappy-redirects-kv") {
                kv_namespace_id = ns.pointer("/id").and_then(|v| v.as_str()).map(|s| s.to_string());
                break;
            }
        }
    }

    // 2. If it does not exist, create it
    let kv_namespace_id = match kv_namespace_id {
        Some(id) => id,
        None => {
            let create_resp = client
                .post(&list_url)
                .bearer_auth(&api_token)
                .json(&serde_json::json!({
                    "title": "tappy-redirects-kv"
                }))
                .send()
                .await
                .map_err(|err| format!("Failed to create KV namespace: {err}"))?;

            if !create_resp.status().is_success() {
                let status = create_resp.status();
                let body = create_resp.text().await.unwrap_or_default();
                return Err(format!("Cloudflare API rejected KV namespace creation: {status} {body}"));
            }

            let create_json: serde_json::Value = create_resp
                .json()
                .await
                .map_err(|err| format!("Failed to parse KV creation response: {err}"))?;

            create_json
                .pointer("/result/id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| "Failed to retrieve KV namespace ID from response".to_string())?
        }
    };

    // 3. Deploy the Worker script with KV binding
    let upload_url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{}/workers/scripts/tappy-redirects",
        account_id
    );

    let metadata = serde_json::json!({
        "main_module": "worker.js",
        "compatibility_date": "2024-01-01",
        "bindings": [
            {
                "type": "kv_namespace",
                "name": "REDIRECTS_KV",
                "namespace_id": kv_namespace_id
            }
        ]
    });

    let worker_code = include_str!("worker.js");

    let metadata_part = multipart::Part::text(metadata.to_string())
        .mime_str("application/json")
        .map_err(|err| err.to_string())?;

    let script_part = multipart::Part::text(worker_code)
        .mime_str("application/javascript+module")
        .map_err(|err| err.to_string())?
        .file_name("worker.js");

    let form = multipart::Form::new()
        .part("metadata", metadata_part)
        .part("worker.js", script_part);

    let upload_resp = client
        .put(&upload_url)
        .bearer_auth(&api_token)
        .multipart(form)
        .send()
        .await
        .map_err(|err| format!("Failed to deploy Worker: {err}"))?;

    if !upload_resp.status().is_success() {
        let status = upload_resp.status();
        let body = upload_resp.text().await.unwrap_or_default();
        return Err(format!("Cloudflare API rejected Worker deployment: {status} {body}"));
    }

    // 3.5 Upload ADMIN_TOKEN as a Secret (keeps it hidden in Cloudflare Dashboard)
    let secret_url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{}/workers/scripts/tappy-redirects/secrets",
        account_id
    );

    let secret_resp = client
        .put(&secret_url)
        .bearer_auth(&api_token)
        .json(&serde_json::json!({
            "name": "ADMIN_TOKEN",
            "text": admin_token,
            "type": "secret_text"
        }))
        .send()
        .await
        .map_err(|err| format!("Failed to connect to Cloudflare Secrets API: {err}"))?;

    if !secret_resp.status().is_success() {
        let status = secret_resp.status();
        let body = secret_resp.text().await.unwrap_or_default();
        return Err(format!("Cloudflare API rejected Admin Token Secret setup: {status} {body}"));
    }

    // 4. Map worker automatically to custom domain or path route on the selected zone
    let base_url = setup_worker_routing(
        &client,
        &api_token,
        &account_id,
        &zone_id,
        &zone_name,
        subdomain,
        path,
    )
    .await?;

    Ok(CloudflareDeployResult {
        base_url,
        admin_token,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationResult {
    pub admin_token: String,
}

#[tauri::command]
pub async fn rotate_worker_token(
    state: State<'_, AppState>,
    account_id: String,
    api_token: String,
) -> Result<RotationResult, String> {
    let worker = get_worker_config_from_state(&state).await?;
    let new_admin_token = generate_admin_token();

    // 1. Upload new ADMIN_TOKEN as a Secret to Cloudflare
    let client = Client::new();
    let secret_url = format!(
        "https://api.cloudflare.com/client/v4/accounts/{}/workers/scripts/tappy-redirects/secrets",
        account_id.trim()
    );

    let resp = client
        .put(&secret_url)
        .bearer_auth(api_token.trim())
        .json(&serde_json::json!({
            "name": "ADMIN_TOKEN",
            "text": new_admin_token,
            "type": "secret_text"
        }))
        .send()
        .await
        .map_err(|err| format!("Failed to connect to Cloudflare Secrets API: {err}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Cloudflare API rejected Admin Token Secret rotation: {status} {body}"));
    }

    // 2. Persist the new token to the OS keychain
    save_admin_token(&new_admin_token)?;

    // 3. Configure the new token locally in backend state
    let new_config = WorkerConfig {
        base_url: worker.base_url.clone(),
        admin_token: new_admin_token.clone(),
        slug: worker.slug.clone(),
    };

    let mut guard = state.worker.write().await;
    *guard = Some(new_config);

    Ok(RotationResult {
        admin_token: new_admin_token,
    })
}