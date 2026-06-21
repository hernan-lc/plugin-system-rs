use crate::response::ApiResponse;
use axum::{response::IntoResponse, Json};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct ProxyRequest {
    pub url: String,
    pub method: Option<String>,
    pub body: Option<serde_json::Value>,
    pub headers: Option<std::collections::HashMap<String, String>>,
}

#[derive(Serialize)]
pub struct ProxyResponse {
    pub status: u16,
    pub body: serde_json::Value,
    pub headers: std::collections::HashMap<String, String>,
}

use crate::state::AppState;
use axum::extract::State;

pub async fn proxy_handler(
    State(state): State<AppState>,
    Json(req): Json<ProxyRequest>,
) -> impl IntoResponse {
    let client = &state.http_client;
    let method = match req.method.as_deref() {
        Some("POST") => reqwest::Method::POST,
        Some("PUT") => reqwest::Method::PUT,
        Some("DELETE") => reqwest::Method::DELETE,
        _ => reqwest::Method::GET,
    };

    let mut rb = client.request(method, &req.url);

    if let Some(headers) = req.headers {
        for (k, v) in headers {
            rb = rb.header(k, v);
        }
    }

    if let Some(body) = req.body {
        rb = rb.json(&body);
    }

    match rb.send().await {
        Ok(res) => {
            let status = res.status().as_u16();
            let mut headers = std::collections::HashMap::new();
            for (name, value) in res.headers() {
                if let Ok(value_str) = value.to_str() {
                    headers.insert(name.to_string(), value_str.to_string());
                }
            }

            let body = if res
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .map(|v| v.contains("application/json"))
                .unwrap_or(false)
            {
                res.json::<serde_json::Value>()
                    .await
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::String(res.text().await.unwrap_or_default())
            };

            Json(ApiResponse::success(ProxyResponse {
                status,
                body,
                headers,
            }))
        }
        Err(e) => Json(ApiResponse::error(e.to_string())),
    }
}
