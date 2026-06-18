use plugin_system::{command, CommandResult, PluginContext, PluginMetadata};
use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use linux as platform;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as platform;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as platform;

#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
mod unsupported;
#[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
use unsupported as platform;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolumeState {
    pub master_volume: f32,
    pub muted: bool,
    pub default_device_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppVolume {
    pub name: String,
    pub volume: f32,
    pub muted: bool,
    pub pid: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolumeData {
    pub state: VolumeState,
    #[serde(default)]
    pub apps: Vec<AppVolume>,
    pub platform_supported: bool,
    pub per_app_supported: bool,
}

pub trait VolumeControl: Send + Sync {
    fn get_master_volume(&mut self) -> Result<VolumeState, String>;
    fn set_master_volume(&mut self, volume: f32) -> Result<(), String>;
    fn set_muted(&mut self, muted: bool) -> Result<(), String>;
    fn get_app_volumes(&mut self) -> Result<Vec<AppVolume>, String>;
    fn set_app_volume(&mut self, app_name: &str, volume: f32) -> Result<(), String>;
    fn set_app_muted(&mut self, app_name: &str, muted: bool) -> Result<(), String>;
}

pub struct VolumeMasterPlugin {
    controller: Box<dyn VolumeControl>,
    pub data: VolumeData,
}

impl Default for VolumeMasterPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl VolumeMasterPlugin {
    #[cfg(test)]
    pub(crate) fn with_controller(mut controller: Box<dyn VolumeControl>) -> Self {
        let data = controller
            .get_master_volume()
            .map(|state| VolumeData {
                state,
                apps: controller.get_app_volumes().unwrap_or_default(),
                platform_supported: true,
                per_app_supported: true,
            })
            .unwrap_or(VolumeData {
                platform_supported: false,
                per_app_supported: false,
                ..Default::default()
            });

        Self { controller, data }
    }
}

#[plugin_system::plugin_export]
impl VolumeMasterPlugin {
    pub fn new() -> Self {
        let mut controller = platform::create_controller();
        let data = controller
            .get_master_volume()
            .map(|state| VolumeData {
                state,
                apps: controller.get_app_volumes().unwrap_or_default(),
                platform_supported: true,
                per_app_supported: platform::per_app_supported(),
            })
            .unwrap_or(VolumeData {
                platform_supported: false,
                per_app_supported: false,
                ..Default::default()
            });

        Self { controller, data }
    }

    fn metadata(&self) -> PluginMetadata {
        plugin_system::plugin_metadata! {
            name: "volume-master",
            version: "0.1.0",
            authors: ["StreamDeck Core"],
            dependencies: []
        }
    }

    fn on_load(&mut self, _ctx: &PluginContext) {
        log::info!("VolumeMasterPlugin loaded");
        self.refresh_internal();
    }

    fn on_unload(&mut self) {
        log::info!("VolumeMasterPlugin unloading");
    }

    fn refresh_internal(&mut self) {
        if let Ok(state) = self.controller.get_master_volume() {
            self.data.state = state;
        }
        if let Ok(apps) = self.controller.get_app_volumes() {
            self.data.apps = apps;
        }
    }

    fn interface_data(&self) -> Option<serde_json::Value> {
        serde_json::to_value(&self.data).ok()
    }

    #[command("refresh")]
    fn vol_refresh(&mut self) -> CommandResult {
        self.refresh_internal();
        Ok(serde_json::json!({"ok": true}))
    }

    #[command("set_volume")]
    fn vol_set_volume(&mut self, volume: f64) -> CommandResult {
        let clamped = (volume as f32).clamp(0.0, 100.0);
        self.controller.set_master_volume(clamped)?;
        self.data.state.master_volume = clamped;
        Ok(serde_json::json!({"ok": true}))
    }

    #[command("set_mute")]
    fn vol_set_muted(&mut self, muted: bool) -> CommandResult {
        self.controller.set_muted(muted)?;
        self.data.state.muted = muted;
        Ok(serde_json::json!({"ok": true}))
    }

    #[command("set_app_volume")]
    fn vol_set_app_volume(&mut self, app_name: String, volume: f64) -> CommandResult {
        let clamped = (volume as f32).clamp(0.0, 100.0);
        self.controller.set_app_volume(&app_name, clamped)?;
        if let Some(app) = self.data.apps.iter_mut().find(|a| a.name == app_name) {
            app.volume = clamped;
        }
        Ok(serde_json::json!({"ok": true}))
    }

    #[command("set_app_mute")]
    fn vol_set_app_muted(&mut self, app_name: String, muted: bool) -> CommandResult {
        self.controller.set_app_muted(&app_name, muted)?;
        if let Some(app) = self.data.apps.iter_mut().find(|a| a.name == app_name) {
            app.muted = muted;
        }
        Ok(serde_json::json!({"ok": true}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use plugin_system::Plugin;

    #[derive(Default)]
    struct MockVolumeControl {
        state: VolumeState,
        apps: Vec<AppVolume>,
    }

    impl MockVolumeControl {
        fn new() -> Self {
            Self {
                state: VolumeState {
                    master_volume: 45.0,
                    muted: false,
                    default_device_name: "Default Output".to_string(),
                },
                apps: vec![AppVolume {
                    name: "Firefox".to_string(),
                    volume: 14.0,
                    muted: false,
                    pid: Some(1324084),
                }],
            }
        }
    }

    impl VolumeControl for MockVolumeControl {
        fn get_master_volume(&mut self) -> Result<VolumeState, String> {
            Ok(self.state.clone())
        }

        fn set_master_volume(&mut self, volume: f32) -> Result<(), String> {
            self.state.master_volume = volume;
            Ok(())
        }

        fn set_muted(&mut self, muted: bool) -> Result<(), String> {
            self.state.muted = muted;
            Ok(())
        }

        fn get_app_volumes(&mut self) -> Result<Vec<AppVolume>, String> {
            Ok(self.apps.clone())
        }

        fn set_app_volume(&mut self, app_name: &str, volume: f32) -> Result<(), String> {
            if let Some(app) = self.apps.iter_mut().find(|a| a.name == app_name) {
                app.volume = volume;
            } else {
                self.apps.push(AppVolume {
                    name: app_name.to_string(),
                    volume,
                    muted: false,
                    pid: None,
                });
            }
            Ok(())
        }

        fn set_app_muted(&mut self, app_name: &str, muted: bool) -> Result<(), String> {
            if let Some(app) = self.apps.iter_mut().find(|a| a.name == app_name) {
                app.muted = muted;
            } else {
                self.apps.push(AppVolume {
                    name: app_name.to_string(),
                    volume: 50.0,
                    muted,
                    pid: None,
                });
            }
            Ok(())
        }
    }

    struct FailingVolumeControl;

    impl VolumeControl for FailingVolumeControl {
        fn get_master_volume(&mut self) -> Result<VolumeState, String> {
            Err("no audio device".to_string())
        }

        fn set_master_volume(&mut self, _volume: f32) -> Result<(), String> {
            Err("cannot set master volume".to_string())
        }

        fn set_muted(&mut self, _muted: bool) -> Result<(), String> {
            Err("cannot mute".to_string())
        }

        fn get_app_volumes(&mut self) -> Result<Vec<AppVolume>, String> {
            Err("per-app volume unavailable".to_string())
        }

        fn set_app_volume(&mut self, _app_name: &str, _volume: f32) -> Result<(), String> {
            Err("cannot set app volume".to_string())
        }

        fn set_app_muted(&mut self, _app_name: &str, _muted: bool) -> Result<(), String> {
            Err("cannot mute app".to_string())
        }
    }

    #[test]
    fn metadata_and_interface_ids_are_generated() {
        let plugin = VolumeMasterPlugin::with_controller(Box::new(MockVolumeControl::new()));

        assert_eq!(plugin.metadata().name, "volume-master");
        assert_eq!(plugin.interface_ids(), vec!["VolumeMaster"]);
    }

    #[test]
    fn refresh_command_updates_interface_data() {
        let mut plugin = VolumeMasterPlugin::with_controller(Box::new(MockVolumeControl::new()));
        plugin
            .handle_command("set_volume", serde_json::json!({"volume": 75.0}))
            .unwrap();

        let refreshed = plugin
            .handle_command("refresh", serde_json::json!({}))
            .unwrap();

        assert_eq!(refreshed["ok"], true);
        let data = plugin.interface_data().unwrap();
        assert_eq!(data["state"]["master_volume"], 75.0);
        assert_eq!(data["apps"][0]["name"], "Firefox");
    }

    #[test]
    fn volume_commands_update_mock_state() {
        let mut plugin = VolumeMasterPlugin::with_controller(Box::new(MockVolumeControl::new()));

        let set_volume = plugin
            .handle_command("set_volume", serde_json::json!({"volume": 150.0}))
            .unwrap();
        let set_mute = plugin
            .handle_command("set_mute", serde_json::json!({"muted": true}))
            .unwrap();

        assert_eq!(set_volume["ok"], true);
        assert_eq!(set_mute["ok"], true);
        assert_eq!(plugin.data.state.master_volume, 100.0);
        assert!(plugin.data.state.muted);
    }

    #[test]
    fn app_volume_commands_update_mock_state() {
        let mut plugin = VolumeMasterPlugin::with_controller(Box::new(MockVolumeControl::new()));

        plugin
            .handle_command(
                "set_app_volume",
                serde_json::json!({"app_name": "Firefox", "volume": 33.0}),
            )
            .unwrap();
        plugin
            .handle_command(
                "set_app_mute",
                serde_json::json!({"app_name": "Firefox", "muted": true}),
            )
            .unwrap();

        let app = plugin.data.apps.first().unwrap();
        assert_eq!(app.volume, 33.0);
        assert!(app.muted);
    }

    #[test]
    fn command_errors_are_returned_as_json() {
        let mut plugin = VolumeMasterPlugin::with_controller(Box::new(FailingVolumeControl));

        let error = plugin
            .handle_command("set_volume", serde_json::json!({"volume": 50.0}))
            .unwrap();

        assert_eq!(error["ok"], false);
        assert_eq!(error["error"], "cannot set master volume");
    }
}
