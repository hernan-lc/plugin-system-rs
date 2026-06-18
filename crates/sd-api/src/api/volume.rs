use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{response::ApiResponse, state::AppState};

#[derive(Serialize, Deserialize)]
pub(crate) struct VolumeStateResponse {
    master_volume: f32,
    muted: bool,
    default_device_name: String,
    platform_supported: bool,
    per_app_supported: bool,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct AppVolumeResponse {
    name: String,
    volume: f32,
    muted: bool,
    pid: Option<u32>,
}

#[derive(Serialize)]
pub(crate) struct VolumeDataResponse {
    state: VolumeStateResponse,
    apps: Vec<AppVolumeResponse>,
}

#[derive(Deserialize)]
pub(crate) struct SetVolumeRequest {
    volume: f32,
}

#[derive(Deserialize)]
pub(crate) struct SetMuteRequest {
    muted: bool,
}

#[derive(Deserialize)]
pub(crate) struct SetAppVolumeRequest {
    app_name: String,
    volume: f32,
}

#[derive(Deserialize)]
pub(crate) struct SetAppMuteRequest {
    app_name: String,
    muted: bool,
}

fn parse_volume_data(data: serde_json::Value) -> Option<VolumeDataResponse> {
    let state = data.get("state")?;
    let apps = data
        .get("apps")
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default();

    Some(VolumeDataResponse {
        state: VolumeStateResponse {
            master_volume: state
                .get("master_volume")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32,
            muted: state
                .get("muted")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            default_device_name: state
                .get("default_device_name")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            platform_supported: data
                .get("platform_supported")
                .and_then(|v| v.as_bool())
                .or_else(|| state.get("platform_supported").and_then(|v| v.as_bool()))
                .unwrap_or(false),
            per_app_supported: data
                .get("per_app_supported")
                .and_then(|v| v.as_bool())
                .or_else(|| state.get("per_app_supported").and_then(|v| v.as_bool()))
                .unwrap_or(false),
        },
        apps: apps
            .into_iter()
            .map(|a| AppVolumeResponse {
                name: a
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                volume: a.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                muted: a.get("muted").and_then(|v| v.as_bool()).unwrap_or(false),
                pid: a.get("pid").and_then(|v| v.as_u64()).map(|p| p as u32),
            })
            .collect(),
    })
}

pub(crate) async fn get_volume_state(
    State(state): State<AppState>,
) -> Json<ApiResponse<VolumeDataResponse>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;

    // First refresh the data
    let _ = crate::api::helpers::call_plugin_ok(
        &manager,
        "volume-master",
        "refresh",
        serde_json::json!({}),
    )
    .await;

    // Then read the interface data
    if let Ok(plugin_arc) = manager.get_plugin_arc("volume-master") {
        let plugin = plugin_arc.read().expect("plugin lock poisoned");
        if let Some(data) = plugin.interface_data() {
            if let Some(resp) = parse_volume_data(data) {
                return Json(ApiResponse::success(resp));
            }
        }
    }

    Json(ApiResponse::error("Volume plugin not available"))
}

pub(crate) async fn get_app_volumes(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<AppVolumeResponse>>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;

    // First refresh the data
    let _ = crate::api::helpers::call_plugin_ok(
        &manager,
        "volume-master",
        "refresh",
        serde_json::json!({}),
    )
    .await;

    // Then read the interface data
    if let Ok(plugin_arc) = manager.get_plugin_arc("volume-master") {
        let plugin = plugin_arc.read().expect("plugin lock poisoned");
        if let Some(data) = plugin.interface_data() {
            let apps = data
                .get("apps")
                .and_then(|a| a.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|a| AppVolumeResponse {
                            name: a
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string(),
                            volume: a.get("volume").and_then(|v| v.as_f64()).unwrap_or(0.0) as f32,
                            muted: a.get("muted").and_then(|v| v.as_bool()).unwrap_or(false),
                            pid: a.get("pid").and_then(|v| v.as_u64()).map(|p| p as u32),
                        })
                        .collect()
                })
                .unwrap_or_default();
            return Json(ApiResponse::success(apps));
        }
    }

    Json(ApiResponse::error("Volume plugin not available"))
}

pub(crate) async fn set_master_volume(
    State(state): State<AppState>,
    Json(req): Json<SetVolumeRequest>,
) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    let args = serde_json::json!({"volume": req.volume});
    Json(
        crate::api::helpers::call_plugin_ok_response(
            &manager,
            "volume-master",
            "set_volume",
            args,
            "Volume set",
        )
        .await,
    )
}

pub(crate) async fn set_master_mute(
    State(state): State<AppState>,
    Json(req): Json<SetMuteRequest>,
) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    let args = serde_json::json!({"muted": req.muted});
    Json(
        crate::api::helpers::call_plugin_ok_response(
            &manager,
            "volume-master",
            "set_mute",
            args,
            "Mute set",
        )
        .await,
    )
}

pub(crate) async fn set_app_volume(
    State(state): State<AppState>,
    Json(req): Json<SetAppVolumeRequest>,
) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    let args = serde_json::json!({"app_name": req.app_name, "volume": req.volume});
    Json(
        crate::api::helpers::call_plugin_ok_response(
            &manager,
            "volume-master",
            "set_app_volume",
            args,
            "App volume set",
        )
        .await,
    )
}

pub(crate) async fn set_app_mute(
    State(state): State<AppState>,
    Json(req): Json<SetAppMuteRequest>,
) -> Json<ApiResponse<String>> {
    let pm = state.plugin_manager.plugin_manager();
    let manager = pm.read().await;
    let args = serde_json::json!({"app_name": req.app_name, "muted": req.muted});
    Json(
        crate::api::helpers::call_plugin_ok_response(
            &manager,
            "volume-master",
            "set_app_mute",
            args,
            "App mute set",
        )
        .await,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_volume_data_with_apps_and_support_flags() {
        let data = serde_json::json!({
            "state": {
                "master_volume": 42.5,
                "muted": false,
                "default_device_name": "Speakers",
                "platform_supported": true,
                "per_app_supported": true
            },
            "apps": [
                {
                    "name": "Firefox",
                    "volume": 25.0,
                    "muted": true,
                    "pid": 1234
                }
            ]
        });

        let parsed = parse_volume_data(data).unwrap();

        assert_eq!(parsed.state.master_volume, 42.5);
        assert!(!parsed.state.muted);
        assert_eq!(parsed.state.default_device_name, "Speakers");
        assert!(parsed.state.platform_supported);
        assert!(parsed.state.per_app_supported);
        assert_eq!(parsed.apps.len(), 1);
        assert_eq!(parsed.apps[0].name, "Firefox");
        assert_eq!(parsed.apps[0].volume, 25.0);
        assert!(parsed.apps[0].muted);
        assert_eq!(parsed.apps[0].pid, Some(1234));
    }

    #[test]
    fn parses_volume_data_without_apps_as_empty_list() {
        let data = serde_json::json!({
            "state": {
                "master_volume": 0.0,
                "muted": true,
                "default_device_name": "",
                "platform_supported": false,
                "per_app_supported": false
            }
        });

        let parsed = parse_volume_data(data).unwrap();

        assert!(!parsed.state.platform_supported);
        assert!(!parsed.state.per_app_supported);
        assert!(parsed.apps.is_empty());
    }
}

