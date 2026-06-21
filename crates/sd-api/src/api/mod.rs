pub(crate) mod actions;
pub(crate) mod dashboard;
pub(crate) mod dashboard_handlers;
pub(crate) mod devices;
pub(crate) mod helpers;
pub(crate) mod hotkeys;
pub(crate) mod obs;
pub(crate) mod plugins;
pub(crate) mod profiles;
pub(crate) mod proxy;
pub(crate) mod router;
pub(crate) mod system;
pub(crate) mod volume;
pub(crate) mod websocket;

pub use dashboard::{load_dashboard_config, DashboardLayout, DashboardWidget};
