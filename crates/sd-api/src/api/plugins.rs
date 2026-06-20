use axum::{
    extract::{Multipart, Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{response::ApiResponse, state::AppState};
use sd_plugins::PluginStatus;

#[derive(Serialize)]
pub(crate) struct PluginDataResponse {
    name: String,
    version: String,
    interfaces: Vec<String>,
    data: serde_json::Value,
}

struct UploadedPluginFile {
    filename: String,
    bytes: Vec<u8>,
}

async fn read_plugin_upload(
    mut multipart: Multipart,
) -> Result<(UploadedPluginFile, Option<bool>), String> {
    let mut file = None;
    let mut enabled = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| format!("Failed to read multipart field: {e}"))?
    {
        let name = field.name().unwrap_or_default().to_string();
        if name == "file" {
            let filename = field.file_name().unwrap_or("plugin.dll").to_string();
            let bytes = field
                .bytes()
                .await
                .map_err(|e| format!("Failed to read plugin file: {e}"))?;
            file = Some(UploadedPluginFile {
                filename,
                bytes: bytes.to_vec(),
            });
        } else if name == "enabled" {
            let value = field
                .text()
                .await
                .map_err(|e| format!("Failed to read enabled field: {e}"))?;
            enabled = Some(value != "false" && value != "0");
        }
    }

    let file = file.ok_or_else(|| "Missing multipart file field".to_string())?;
    Ok((file, enabled))
}

fn plugin_api_error(error: impl ToString) -> String {
    error.to_string()
}

pub(crate) async fn list_plugins(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<PluginStatus>>> {
    match state.plugin_manager.list_plugin_statuses().await {
        Ok(statuses) => Json(ApiResponse::success(statuses)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

pub(crate) async fn reload_plugins(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    match state.plugin_manager.reload_plugins().await {
        Ok(()) => Json(ApiResponse::success("Plugins reloaded".to_string())),
        Err(e) => Json(ApiResponse::error(e)),
    }
}

pub(crate) async fn refresh_plugins(State(state): State<AppState>) -> Json<ApiResponse<String>> {
    match state.plugin_manager.refresh_plugins_from_dir().await {
        Ok(_) => Json(ApiResponse::success("Plugins refreshed".to_string())),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

pub(crate) async fn upload_plugin(
    State(state): State<AppState>,
    multipart: Multipart,
) -> Json<ApiResponse<PluginStatus>> {
    let (file, enabled) = match read_plugin_upload(multipart).await {
        Ok(value) => value,
        Err(e) => return Json(ApiResponse::error(e)),
    };

    match state
        .plugin_manager
        .install_plugin_file(&file.bytes, &file.filename, enabled.unwrap_or(true))
        .await
    {
        Ok(status) => Json(ApiResponse::success(status)),
        Err(e) => Json(ApiResponse::error(plugin_api_error(e))),
    }
}

pub(crate) async fn update_plugin(
    State(state): State<AppState>,
    Path(plugin_name): Path<String>,
    multipart: Multipart,
) -> Json<ApiResponse<PluginStatus>> {
    let (file, enabled) = match read_plugin_upload(multipart).await {
        Ok(value) => value,
        Err(e) => return Json(ApiResponse::error(e)),
    };

    match state
        .plugin_manager
        .update_plugin_file(&plugin_name, &file.bytes, &file.filename, enabled)
        .await
    {
        Ok(status) => Json(ApiResponse::success(status)),
        Err(e) => Json(ApiResponse::error(plugin_api_error(e))),
    }
}

pub(crate) async fn get_plugin_data(
    State(state): State<AppState>,
    Path(plugin_name): Path<String>,
) -> Json<ApiResponse<PluginDataResponse>> {
    let plugin_manager = state.plugin_manager.plugin_manager();
    let manager = plugin_manager.read().await;

    match manager.get_plugin_arc(&plugin_name) {
        Ok(plugin_arc) => {
            let plugin = plugin_arc.read().expect("plugin lock poisoned");
            let meta = plugin.metadata();
            let interfaces: Vec<String> = plugin
                .interface_ids()
                .into_iter()
                .map(String::from)
                .collect();
            let data = plugin
                .interface_data()
                .unwrap_or_else(|| serde_json::json!({}));

            Json(ApiResponse::success(PluginDataResponse {
                name: meta.name,
                version: meta.version,
                interfaces,
                data,
            }))
        }
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}

#[derive(Deserialize)]
pub(crate) struct SetEnabledRequest {
    enabled: bool,
}

pub(crate) async fn set_plugin_enabled(
    State(state): State<AppState>,
    Path(plugin_name): Path<String>,
    Json(req): Json<SetEnabledRequest>,
) -> Json<ApiResponse<PluginStatus>> {
    match state
        .plugin_manager
        .set_plugin_enabled(plugin_name.clone(), req.enabled)
        .await
    {
        Ok(status) => Json(ApiResponse::success(status)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}
