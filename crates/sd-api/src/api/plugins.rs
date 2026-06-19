use axum::{
    extract::{Path, State},
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

pub(crate) async fn list_plugins(State(state): State<AppState>) -> Json<ApiResponse<Vec<PluginStatus>>> {
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
    match state.plugin_manager.set_plugin_enabled(plugin_name.clone(), req.enabled).await {
        Ok(status) => Json(ApiResponse::success(status)),
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}
