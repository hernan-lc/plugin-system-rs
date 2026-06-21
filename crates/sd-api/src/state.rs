use std::sync::Arc;

use sd_actions::ActionRegistry;
use sd_devices::DeviceManager;
use sd_events::EventBus;
use sd_plugins::SdPluginManager;
use sd_profiles::ProfileManager;
use tokio::sync::RwLock;

use crate::api::DashboardLayout;

#[derive(Clone)]
pub struct AppState {
    pub events: Arc<EventBus>,
    pub action_registry: Arc<RwLock<ActionRegistry>>,
    pub profile_manager: Arc<ProfileManager>,
    pub device_manager: Arc<DeviceManager>,
    pub plugin_manager: Arc<SdPluginManager>,
    pub dashboard_config: Arc<RwLock<DashboardLayout>>,
    pub http_client: reqwest::Client,
}
