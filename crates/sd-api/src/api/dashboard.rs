use serde::{Deserialize, Serialize};

pub(crate) const DASHBOARD_CONFIG_PATH: &str = "data/dashboard.json";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DashboardWidget {
    pub id: String,
    #[serde(rename = "type")]
    pub widget_type: String,
    pub title: String,
    #[serde(rename = "colSpan")]
    pub col_span: u32,
    #[serde(rename = "rowSpan")]
    pub row_span: u32,
    #[serde(default)]
    pub settings: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DashboardLayout {
    pub widgets: Vec<DashboardWidget>,
    pub columns: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_css: Option<String>,
}

impl Default for DashboardLayout {
    fn default() -> Self {
        Self {
            widgets: Vec::new(),
            columns: 3,
            custom_css: None,
        }
    }
}

pub fn load_dashboard_config() -> DashboardLayout {
    std::fs::read_to_string(DASHBOARD_CONFIG_PATH)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_default()
}

pub(crate) fn save_dashboard_config(layout: &DashboardLayout) -> bool {
    if let Some(parent) = std::path::Path::new(DASHBOARD_CONFIG_PATH).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    match serde_json::to_string_pretty(layout) {
        Ok(json) => std::fs::write(DASHBOARD_CONFIG_PATH, json).is_ok(),
        Err(_) => false,
    }
}
