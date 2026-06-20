use std::sync::{Arc, Mutex};
use std::time::Duration;

use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::introspect::Introspector;
use libpulse_binding::context::{Context, FlagSet as ContextFlagSet, State as ContextState};
use libpulse_binding::mainloop::threaded::Mainloop;
use libpulse_binding::operation::State as OpState;
use libpulse_binding::proplist;
use libpulse_binding::volume::{ChannelVolumes, Volume};

use crate::{AppVolume, VolumeControl, VolumeState};

// ---------------------------------------------------------------------------
// Volume conversion helpers
// ---------------------------------------------------------------------------

fn percent_to_volume(percent: f32) -> Volume {
    let p = percent.clamp(0.0, 100.0);
    Volume(((p / 100.0) * Volume::NORMAL.0 as f32).round() as u32)
}

fn channel_volumes_from_percent(percent: f32) -> ChannelVolumes {
    let mut cv = ChannelVolumes::default();
    cv.set(1, percent_to_volume(percent));
    cv
}

fn percent_from_volume(vol: Volume) -> f32 {
    (vol.0 as f32 / Volume::NORMAL.0 as f32) * 100.0
}

fn percent_from_channel_volumes(cv: &ChannelVolumes) -> f32 {
    percent_from_volume(cv.avg())
}

const POLL_MS: u64 = 10;

type SinkInput = (u32, String, f32, bool, Option<u32>);

// ---------------------------------------------------------------------------
// PulseController
// ---------------------------------------------------------------------------

#[allow(clippy::arc_with_non_send_sync)]
pub struct PulseController {
    mainloop: Arc<Mutex<Mainloop>>,
    context: Arc<Mutex<Context>>,
}

unsafe impl Send for PulseController {}
unsafe impl Sync for PulseController {}

impl PulseController {
    fn new() -> Result<Self, String> {
        let mut ml = Mainloop::new().ok_or("Mainloop::new() failed")?;

        let mut ctx = Context::new(&ml, "plugin-volume-master").ok_or("Context::new() failed")?;

        ctx.connect(None, ContextFlagSet::NOFLAGS, None)
            .map_err(|e| format!("context.connect() failed: {:?}", e))?;

        ml.lock();
        ml.start().map_err(|e| {
            ml.unlock();
            format!("mainloop.start() failed: {:?}", e)
        })?;

        // Poll for context ready — unlock/sleep/lock to let event loop process
        let poll_result = loop {
            match ctx.get_state() {
                ContextState::Ready => break Ok(()),
                ContextState::Failed | ContextState::Terminated => {
                    break Err(format!("Context state: {:?}", ctx.get_state()));
                }
                _ => {
                    ml.unlock();
                    std::thread::sleep(Duration::from_millis(POLL_MS));
                    ml.lock();
                }
            }
        };

        if let Err(e) = poll_result {
            // Clean up: disconnect context, unlock, stop mainloop
            ctx.disconnect();
            ml.unlock();
            ml.stop();
            return Err(e);
        }

        ctx.set_state_callback(None);
        ml.unlock();

        #[allow(clippy::arc_with_non_send_sync)]
        let mainloop = Arc::new(Mutex::new(ml));
        #[allow(clippy::arc_with_non_send_sync)]
        let context = Arc::new(Mutex::new(ctx));

        Ok(Self { mainloop, context })
    }

    fn shutdown(&self) {
        if let Ok(mut ctx) = self.context.lock() {
            ctx.disconnect();
        }
        if let Ok(mut ml) = self.mainloop.lock() {
            ml.lock();
            ml.unlock();
            ml.stop();
        }
    }

    fn with_introspect<F, R>(&self, f: F) -> Result<R, String>
    where
        F: FnOnce(&mut Mainloop, &mut Introspector) -> Result<R, String>,
    {
        let mut ml = self.mainloop.lock().map_err(|e| format!("lock: {}", e))?;
        let ctx = self
            .context
            .lock()
            .map_err(|e| format!("lock ctx: {}", e))?;
        ml.lock();
        let mut intro = ctx.introspect();
        let result = f(&mut ml, &mut intro);
        drop(intro);
        drop(ctx);
        ml.unlock();
        result
    }

    /// Poll an operation until Done. Unlocks mainloop between checks.
    fn poll_op(ml: &mut Mainloop, op_state: impl Fn() -> OpState) -> Result<(), String> {
        loop {
            match op_state() {
                OpState::Done => return Ok(()),
                OpState::Cancelled => return Err("Operation cancelled".into()),
                _ => {
                    ml.unlock();
                    std::thread::sleep(Duration::from_millis(POLL_MS));
                    ml.lock();
                }
            }
        }
    }

    fn get_default_sink_name(
        ml: &mut Mainloop,
        intro: &mut Introspector,
    ) -> Result<String, String> {
        let result: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let r = result.clone();

        let op = intro.get_server_info(move |info| {
            *r.lock().unwrap() = info.default_sink_name.as_ref().map(|s| s.to_string());
        });
        Self::poll_op(ml, || op.get_state())?;

        let val = result.lock().unwrap().take();
        val.ok_or_else(|| "No default sink".to_string())
    }

    fn get_sink_input_infos(
        ml: &mut Mainloop,
        intro: &mut Introspector,
    ) -> Vec<(u32, String, f32, bool, Option<u32>)> {
        let inputs: Arc<Mutex<Vec<SinkInput>>> = Arc::new(Mutex::new(Vec::new()));
        let inp = inputs.clone();

        let op = intro.get_sink_input_info_list(move |result| {
            if let ListResult::Item(info) = result {
                let raw_name = info
                    .name
                    .as_ref()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "Unknown".into());
                let app_name = info
                    .proplist
                    .get_str(proplist::properties::APPLICATION_NAME)
                    .or_else(|| {
                        info.proplist
                            .get_str(proplist::properties::APPLICATION_PROCESS_BINARY)
                    })
                    .unwrap_or_else(|| raw_name.clone());
                let pid = info
                    .proplist
                    .get_str(proplist::properties::APPLICATION_PROCESS_ID)
                    .and_then(|s| s.parse::<u32>().ok());
                let vol = percent_from_channel_volumes(&info.volume);
                let muted = info.mute;

                inp.lock()
                    .unwrap()
                    .push((info.index, app_name, vol, muted, pid));
            }
        });
        let _ = Self::poll_op(ml, || op.get_state());

        let out = inputs.lock().unwrap().clone();
        out
    }
}

impl VolumeControl for PulseController {
    fn get_master_volume(&mut self) -> Result<VolumeState, String> {
        self.with_introspect(|ml, intro| {
            let sink_name = Self::get_default_sink_name(ml, intro)?;

            let vol_out: Arc<Mutex<Option<f32>>> = Arc::new(Mutex::new(None));
            let mute_out: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));
            let name_out: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

            let v = vol_out.clone();
            let m = mute_out.clone();
            let n = name_out.clone();

            let op = intro.get_sink_info_by_name(&sink_name, move |result| {
                if let ListResult::Item(info) = result {
                    *v.lock().unwrap() = Some(percent_from_channel_volumes(&info.volume));
                    *m.lock().unwrap() = Some(info.mute);
                    *n.lock().unwrap() = info.name.as_ref().map(|s| s.to_string());
                }
            });
            Self::poll_op(ml, || op.get_state())?;

            let volume = vol_out.lock().unwrap().take().unwrap_or(0.0);
            let muted = mute_out.lock().unwrap().take().unwrap_or(false);
            let device_name = name_out
                .lock()
                .unwrap()
                .take()
                .unwrap_or_else(|| "Default".to_string());

            Ok(VolumeState {
                master_volume: volume,
                muted,
                default_device_name: device_name,
            })
        })
    }

    fn set_master_volume(&mut self, volume: f32) -> Result<(), String> {
        let clamped = volume.clamp(0.0, 100.0);
        let sink_name = self.get_default_sink_name_outer()?;

        self.with_introspect(|ml, intro| {
            let new_cv = channel_volumes_from_percent(clamped);
            let success: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));
            let sc = success.clone();

            let op = intro.set_sink_volume_by_name(
                &sink_name,
                &new_cv,
                Some(Box::new(move |ok| {
                    *sc.lock().unwrap() = Some(ok);
                })),
            );
            Self::poll_op(ml, || op.get_state())?;
            Ok(())
        })
    }

    fn set_muted(&mut self, muted: bool) -> Result<(), String> {
        let sink_name = self.get_default_sink_name_outer()?;

        self.with_introspect(|ml, intro| {
            let success: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));
            let sc = success.clone();

            let op = intro.set_sink_mute_by_name(
                &sink_name,
                muted,
                Some(Box::new(move |ok| {
                    *sc.lock().unwrap() = Some(ok);
                })),
            );
            Self::poll_op(ml, || op.get_state())?;
            Ok(())
        })
    }

    fn get_app_volumes(&mut self) -> Result<Vec<AppVolume>, String> {
        self.with_introspect(|ml, intro| {
            let list = Self::get_sink_input_infos(ml, intro);
            Ok(list
                .into_iter()
                .map(|(_, name, volume, muted, pid)| AppVolume {
                    name,
                    volume,
                    muted,
                    pid,
                })
                .collect())
        })
    }

    fn set_app_volume(&mut self, app_name: &str, volume: f32) -> Result<(), String> {
        let clamped = volume.clamp(0.0, 100.0);

        let index = self.with_introspect(|ml, intro| {
            let list = Self::get_sink_input_infos(ml, intro);
            list.iter()
                .find(|(_, name, _, _, _)| name == app_name)
                .map(|(idx, _, _, _, _)| *idx)
                .ok_or_else(|| format!("App '{}' not found", app_name))
        })?;

        self.with_introspect(|ml, intro| {
            let new_cv = channel_volumes_from_percent(clamped);
            let success: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));
            let sc = success.clone();

            let op = intro.set_sink_input_volume(
                index,
                &new_cv,
                Some(Box::new(move |ok| {
                    *sc.lock().unwrap() = Some(ok);
                })),
            );
            Self::poll_op(ml, || op.get_state())?;
            Ok(())
        })
    }

    fn set_app_muted(&mut self, app_name: &str, muted: bool) -> Result<(), String> {
        let index = self.with_introspect(|ml, intro| {
            let list = Self::get_sink_input_infos(ml, intro);
            list.iter()
                .find(|(_, name, _, _, _)| name == app_name)
                .map(|(idx, _, _, _, _)| *idx)
                .ok_or_else(|| format!("App '{}' not found", app_name))
        })?;

        self.with_introspect(|ml, intro| {
            let success: Arc<Mutex<Option<bool>>> = Arc::new(Mutex::new(None));
            let sc = success.clone();

            let op = intro.set_sink_input_mute(
                index,
                muted,
                Some(Box::new(move |ok| {
                    *sc.lock().unwrap() = Some(ok);
                })),
            );
            Self::poll_op(ml, || op.get_state())?;
            Ok(())
        })
    }
}

impl PulseController {
    fn get_default_sink_name_outer(&self) -> Result<String, String> {
        let mut ml = self.mainloop.lock().map_err(|e| format!("lock: {}", e))?;
        let ctx = self
            .context
            .lock()
            .map_err(|e| format!("lock ctx: {}", e))?;
        ml.lock();
        let mut intro = ctx.introspect();
        let result = Self::get_default_sink_name(&mut ml, &mut intro);
        drop(intro);
        drop(ctx);
        ml.unlock();
        result
    }
}

pub fn create_controller() -> Box<dyn VolumeControl> {
    match PulseController::new() {
        Ok(ctrl) => Box::new(ctrl),
        Err(e) => {
            log::warn!("PulseAudio unavailable ({}), using fallback controller", e);
            Box::new(FallbackController)
        }
    }
}

struct FallbackController;

impl VolumeControl for FallbackController {
    fn get_master_volume(&mut self) -> Result<VolumeState, String> {
        Err("PulseAudio unavailable".into())
    }

    fn set_master_volume(&mut self, _volume: f32) -> Result<(), String> {
        Err("PulseAudio unavailable".into())
    }

    fn set_muted(&mut self, _muted: bool) -> Result<(), String> {
        Err("PulseAudio unavailable".into())
    }

    fn get_app_volumes(&mut self) -> Result<Vec<AppVolume>, String> {
        Err("PulseAudio unavailable".into())
    }

    fn set_app_volume(&mut self, _app_name: &str, _volume: f32) -> Result<(), String> {
        Err("PulseAudio unavailable".into())
    }

    fn set_app_muted(&mut self, _app_name: &str, _muted: bool) -> Result<(), String> {
        Err("PulseAudio unavailable".into())
    }
}

impl Drop for PulseController {
    fn drop(&mut self) {
        self.shutdown();
    }
}

pub fn per_app_supported() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_percent_to_volume_roundtrip() {
        assert_eq!(percent_from_volume(percent_to_volume(0.0)), 0.0);
        assert_eq!(percent_from_volume(percent_to_volume(50.0)), 50.0);
        assert_eq!(percent_from_volume(percent_to_volume(100.0)), 100.0);
    }

    #[test]
    fn test_percent_to_volume_clamping() {
        let v = percent_to_volume(-10.0);
        assert_eq!(v.0, 0);
        let v = percent_to_volume(200.0);
        assert_eq!(v.0, Volume::NORMAL.0);
    }

    #[test]
    fn test_channel_volumes_from_percent() {
        let cv = channel_volumes_from_percent(75.0);
        let vol = cv.avg();
        let pct = percent_from_volume(vol);
        assert!((pct - 75.0).abs() < 1.0, "expected ~75%, got {}%", pct);
    }

    #[test]
    fn test_volume_normal_is_100_percent() {
        assert_eq!(percent_from_volume(Volume::NORMAL), 100.0);
    }

    #[test]
    fn test_volume_muted_is_0_percent() {
        assert_eq!(percent_from_volume(Volume::MUTED), 0.0);
    }
}
