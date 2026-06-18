use std::collections::HashSet;

use windows::core::{Interface, Result as WinResult};
use windows::Win32::Foundation::{BOOL, CloseHandle, RPC_E_CHANGED_MODE, S_FALSE, TRUE};
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    eMultimedia, eRender, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};

use crate::{AppVolume, VolumeControl, VolumeState};

pub struct WindowsController {
    _private: (),
}

struct ComGuard {
    initialized: bool,
}

impl ComGuard {
    fn initialize() -> Result<Self, String> {
        let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };

        if result.is_ok() {
            Ok(Self { initialized: true })
        } else if result == S_FALSE || result == RPC_E_CHANGED_MODE {
            Ok(Self { initialized: false })
        } else {
            Err(format!("CoInitializeEx: {result}"))
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.initialized {
            unsafe { CoUninitialize() };
        }
    }
}

#[derive(Debug, Clone)]
struct SessionVolume {
    display_name: String,
    process_name: Option<String>,
    process_path: Option<String>,
    pid: Option<u32>,
    volume: f32,
    muted: bool,
}

impl SessionVolume {
    fn name(&self) -> String {
        if let Some(name) = self.process_name.as_deref().filter(|name| !name.is_empty()) {
            return name.to_string();
        }

        if !self.display_name.is_empty() {
            return self.display_name.clone();
        }

        self.pid
            .map(|pid| format!("PID:{pid}"))
            .unwrap_or_else(|| "Unknown".to_string())
    }

    fn matches_query(&self, query: &str) -> bool {
        if query.trim().is_empty() {
            return false;
        }

        let query = normalize_app_name(query);
        let display_name = normalize_app_name(&self.display_name);
        let process_name = self.process_name.as_deref().map(normalize_app_name);
        let process_path = self.process_path.as_deref().map(normalize_app_name);
        let pid_name = self.pid.map(|pid| normalize_app_name(&format!("PID:{pid}")));

        display_name == query
            || process_name.as_deref() == Some(query.as_str())
            || process_path.as_deref() == Some(query.as_str())
            || pid_name.as_deref() == Some(query.as_str())
            || self
                .process_path
                .as_deref()
                .map(process_basename)
                .map(normalize_app_name)
                .as_deref()
                == Some(query.as_str())
    }
}

pub fn create_controller() -> Box<dyn VolumeControl> {
    Box::new(WindowsController { _private: () })
}

pub fn per_app_supported() -> bool {
    true
}

fn normalize_app_name(name: &str) -> String {
    name.trim().to_lowercase()
}

fn process_basename(path: &str) -> &str {
    path.rsplit(['\\', '/']).next().unwrap_or(path)
}

fn process_name_from_pid(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        if handle.is_invalid() {
            return None;
        }

        let mut buffer = vec![0u16; 32768];
        let mut length = buffer.len() as u32;
        let result = QueryFullProcessImageNameW(
            handle,
            PROCESS_NAME_WIN32,
            windows::core::PWSTR(buffer.as_mut_ptr()),
            &mut length,
        );

        let _ = CloseHandle(handle);

        if result.is_ok() && length > 0 {
            Some(String::from_utf16_lossy(&buffer[..length as usize]))
        } else {
            None
        }
    }
}

fn get_endpoint_volume() -> WinResult<IAudioEndpointVolume> {
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
        let volume = device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None)?;
        Ok(volume)
    }
}

fn get_device() -> WinResult<IMMDevice> {
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
        Ok(device)
    }
}

impl WindowsController {
    fn get_session_volumes() -> Result<Vec<SessionVolume>, String> {
        let mut sessions = Vec::new();

        unsafe {
            use windows::Win32::Media::Audio::{
                IAudioSessionControl2, IAudioSessionManager2, ISimpleAudioVolume,
            };

            let device = get_device().map_err(|e| format!("Get device: {}", e))?;

            let session_manager: IAudioSessionManager2 = device
                .Activate::<IAudioSessionManager2>(CLSCTX_ALL, None)
                .map_err(|e| format!("Activate IAudioSessionManager2: {}", e))?;

            let session_list = session_manager
                .GetSessionEnumerator()
                .map_err(|e| format!("GetSessionEnumerator: {}", e))?;

            let count = session_list
                .GetCount()
                .map_err(|e| format!("GetCount: {}", e))?;

            for i in 0..count {
                let Ok(session_control) = session_list.GetSession(i) else {
                    continue;
                };
                let Ok(session2) = session_control.cast::<IAudioSessionControl2>() else {
                    continue;
                };

                let display_name = session2
                    .GetDisplayName()
                    .ok()
                    .and_then(|name| name.to_string().ok())
                    .unwrap_or_default();
                let pid = session2.GetProcessId().ok();
                let process_path = pid.and_then(process_name_from_pid);
                let process_name = process_path
                    .as_deref()
                    .map(process_basename)
                    .map(ToString::to_string);

                let mut volume = 0.0f32;
                let mut muted = false;

                if let Ok(simple_vol) = session_control.cast::<ISimpleAudioVolume>() {
                    if let Ok(v) = simple_vol.GetMasterVolume() {
                        volume = v;
                    }
                    if let Ok(m) = simple_vol.GetMute() {
                        muted = m == TRUE;
                    }
                }

                sessions.push(SessionVolume {
                    display_name,
                    process_name,
                    process_path,
                    pid,
                    volume,
                    muted,
                });
            }
        }

        Ok(sessions)
    }

    fn find_session_index(sessions: &[SessionVolume], app_name: &str) -> Option<usize> {
        sessions
            .iter()
            .position(|session| session.matches_query(app_name))
    }
}

impl VolumeControl for WindowsController {
    fn get_master_volume(&mut self) -> Result<VolumeState, String> {
        let _com = ComGuard::initialize()?;

        unsafe {
            let volume = get_endpoint_volume()
                .map_err(|e| format!("Failed to get endpoint volume: {}", e))?;

            let level = volume
                .GetMasterVolumeLevelScalar()
                .map_err(|e| format!("GetMasterVolumeLevelScalar: {}", e))?;

            let muted = volume.GetMute().map_err(|e| format!("GetMute: {}", e))?;

            let device = get_device().map_err(|e| format!("Get device: {}", e))?;

            let device_name = device
                .GetId()
                .map(|id| id.to_string().unwrap_or_default())
                .unwrap_or_default();

            Ok(VolumeState {
                master_volume: level * 100.0,
                muted: muted == TRUE,
                default_device_name: device_name,
            })
        }
    }

    fn set_master_volume(&mut self, volume: f32) -> Result<(), String> {
        let _com = ComGuard::initialize()?;

        unsafe {
            let ep = get_endpoint_volume()
                .map_err(|e| format!("Failed to get endpoint volume: {}", e))?;

            let scalar = (volume / 100.0).clamp(0.0, 1.0);
            ep.SetMasterVolumeLevelScalar(scalar, std::ptr::null())
                .map_err(|e| format!("SetMasterVolumeLevelScalar: {}", e))?;
        }

        Ok(())
    }

    fn set_muted(&mut self, muted: bool) -> Result<(), String> {
        let _com = ComGuard::initialize()?;

        unsafe {
            let ep = get_endpoint_volume()
                .map_err(|e| format!("Failed to get endpoint volume: {}", e))?;

            let mute_val = BOOL::from(muted);
            ep.SetMute(mute_val, std::ptr::null())
                .map_err(|e| format!("SetMute: {}", e))?;
        }

        Ok(())
    }

    fn get_app_volumes(&mut self) -> Result<Vec<AppVolume>, String> {
        let _com = ComGuard::initialize()?;

        let sessions = Self::get_session_volumes()?;
        let mut seen = HashSet::new();
        let mut apps = Vec::new();

        for session in sessions {
            let name = session.name();
            let key = format!("{}|{:?}", name, session.pid).to_lowercase();
            if !seen.insert(key) {
                continue;
            }

            apps.push(AppVolume {
                name,
                volume: session.volume * 100.0,
                muted: session.muted,
                pid: session.pid,
            });
        }

        Ok(apps)
    }

    fn set_app_volume(&mut self, app_name: &str, volume: f32) -> Result<(), String> {
        let _com = ComGuard::initialize()?;

        let sessions = Self::get_session_volumes()?;
        let index = Self::find_session_index(&sessions, app_name)
            .ok_or_else(|| format!("App '{}' not found", app_name))?;

        unsafe {
            use windows::Win32::Media::Audio::{IAudioSessionManager2, ISimpleAudioVolume};

            let device = get_device().map_err(|e| format!("Get device: {}", e))?;
            let session_manager: IAudioSessionManager2 = device
                .Activate::<IAudioSessionManager2>(CLSCTX_ALL, None)
                .map_err(|e| format!("Activate: {}", e))?;

            let session_list = session_manager
                .GetSessionEnumerator()
                .map_err(|e| format!("GetSessionEnumerator: {}", e))?;

            let count = session_list
                .GetCount()
                .map_err(|e| format!("GetCount: {}", e))?;

            let scalar = (volume / 100.0).clamp(0.0, 1.0);

            for (i, session_control) in (0..count).filter_map(|i| session_list.GetSession(i).ok()).enumerate() {
                if i != index {
                    continue;
                }

                if let Ok(simple_vol) = session_control.cast::<ISimpleAudioVolume>() {
                    simple_vol
                        .SetMasterVolume(scalar, std::ptr::null())
                        .map_err(|e| format!("SetMasterVolume: {}", e))?;
                    return Ok(());
                }
            }
        }

        Err(format!("App '{}' not found", app_name))
    }

    fn set_app_muted(&mut self, app_name: &str, muted: bool) -> Result<(), String> {
        let _com = ComGuard::initialize()?;

        let sessions = Self::get_session_volumes()?;
        let index = Self::find_session_index(&sessions, app_name)
            .ok_or_else(|| format!("App '{}' not found", app_name))?;

        unsafe {
            use windows::Win32::Media::Audio::{IAudioSessionManager2, ISimpleAudioVolume};

            let device = get_device().map_err(|e| format!("Get device: {}", e))?;
            let session_manager: IAudioSessionManager2 = device
                .Activate::<IAudioSessionManager2>(CLSCTX_ALL, None)
                .map_err(|e| format!("Activate: {}", e))?;

            let session_list = session_manager
                .GetSessionEnumerator()
                .map_err(|e| format!("GetSessionEnumerator: {}", e))?;

            let count = session_list
                .GetCount()
                .map_err(|e| format!("GetCount: {}", e))?;

            let mute_val = BOOL::from(muted);

            for (i, session_control) in (0..count).filter_map(|i| session_list.GetSession(i).ok()).enumerate() {
                if i != index {
                    continue;
                }

                if let Ok(simple_vol) = session_control.cast::<ISimpleAudioVolume>() {
                    simple_vol
                        .SetMute(mute_val, std::ptr::null())
                        .map_err(|e| format!("SetMute: {}", e))?;
                    return Ok(());
                }
            }
        }

        Err(format!("App '{}' not found", app_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(display_name: &str, process_path: Option<&str>, pid: Option<u32>) -> SessionVolume {
        SessionVolume {
            display_name: display_name.to_string(),
            process_name: process_path.map(process_basename).map(ToString::to_string),
            process_path: process_path.map(ToString::to_string),
            pid,
            volume: 50.0,
            muted: false,
        }
    }

    #[test]
    fn session_matches_by_display_name_case_insensitively() {
        let session = session("Firefox", None, None);

        assert!(session.matches_query("firefox"));
    }

    #[test]
    fn session_matches_by_pid_label() {
        let session = session("", None, Some(1234));

        assert!(session.matches_query("PID:1234"));
    }

    #[test]
    fn session_matches_by_process_basename() {
        let session = session("", Some(r"C:\Program Files\Firefox\firefox.exe"), Some(1234));

        assert!(session.matches_query("firefox.exe"));
    }

    #[test]
    fn session_matches_by_full_process_path() {
        let session = session("", Some(r"C:\Program Files\Firefox\firefox.exe"), Some(1234));

        assert!(session.matches_query(r"C:\Program Files\Firefox\firefox.exe"));
    }

    #[test]
    fn process_basename_handles_windows_and_unix_separators() {
        assert_eq!(process_basename(r"C:\Program Files\App\app.exe"), "app.exe");
        assert_eq!(process_basename("/usr/bin/app"), "app");
    }
}
