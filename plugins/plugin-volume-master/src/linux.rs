use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

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

// ---------------------------------------------------------------------------
// PulseController
// ---------------------------------------------------------------------------

pub struct PulseController {
    mainloop: Rc<RefCell<Mainloop>>,
    context: Rc<RefCell<Context>>,
}

// SAFETY: The threaded mainloop's lock()/unlock() provide synchronization.
// The Rc<RefCell<>> is never moved between threads; only shared via PA's locking.
// This follows libpulse-binding's documented pattern for threaded mainloop usage.
unsafe impl Send for PulseController {}
unsafe impl Sync for PulseController {}

impl PulseController {
    fn new() -> Result<Self, String> {
        let ml = Mainloop::new().ok_or("Mainloop::new() failed")?;
        let ml_rc = Rc::new(RefCell::new(ml));

        let mut ctx = Context::new(ml_rc.borrow().deref(), "plugin-volume-master")
            .ok_or("Context::new() failed")?;

        {
            let ml_ref = Rc::clone(&ml_rc);
            let ctx_ptr = &ctx as *const Context;
            ctx.set_state_callback(Some(Box::new(move || {
                let state = unsafe { (*ctx_ptr).get_state() };
                match state {
                    ContextState::Ready | ContextState::Failed | ContextState::Terminated => {
                        unsafe { (*ml_ref.as_ptr()).signal(false); }
                    }
                    _ => {}
                }
            })));
        }

        ctx.connect(None, ContextFlagSet::NOFLAGS, None)
            .map_err(|e| format!("context.connect() failed: {:?}", e))?;

        {
            let mut ml = ml_rc.borrow_mut();
            ml.lock();
            ml.start().map_err(|e| {
                ml.unlock();
                format!("mainloop.start() failed: {:?}", e)
            })?;

            loop {
                match ctx.get_state() {
                    ContextState::Ready => break,
                    ContextState::Failed | ContextState::Terminated => {
                        let s = ctx.get_state();
                        ml.unlock();
                        return Err(format!("Context state: {:?}", s));
                    }
                    _ => ml.wait(),
                }
            }
            ctx.set_state_callback(None);
            ml.unlock();
        }

        Ok(Self {
            mainloop: ml_rc,
            context: Rc::new(RefCell::new(ctx)),
        })
    }

    /// Get the default sink name from the PA server.
    fn get_default_sink_name(&self) -> Result<String, String> {
        let mut ml = self.mainloop.borrow_mut();
        ml.lock();
        let ctx = self.context.borrow();
        let intro = ctx.introspect();

        let result: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let r = result.clone();
        let ml_ptr = Rc::clone(&self.mainloop);

        let op = intro.get_server_info(move |info| {
            *r.borrow_mut() = info.default_sink_name.as_ref().map(|s| s.to_string());
            unsafe { (*ml_ptr.as_ptr()).signal(false); }
        });

        loop {
            match op.get_state() {
                OpState::Done => break,
                _ => ml.wait(),
            }
        }

        drop(intro);
        drop(ctx);
        let val = result.borrow().clone();
        val.ok_or_else(|| "No default sink".to_string())
    }

    /// Get all sink inputs. Must be called with mainloop lock held.
    fn get_sink_input_infos(ml: &mut Mainloop, intro: &mut Introspector, ml_ptr: &Rc<RefCell<Mainloop>>) -> Vec<(u32, String, f32, bool, Option<u32>)> {
        let inputs: Rc<RefCell<Vec<(u32, String, f32, bool, Option<u32>)>>> = Rc::new(RefCell::new(Vec::new()));
        let inp = inputs.clone();
        let ml_signal = Rc::clone(ml_ptr);

        let op = intro.get_sink_input_info_list(move |result| {
            match result {
                ListResult::Item(info) => {
                    let raw_name = info.name.as_ref()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Unknown".into());
                    let app_name = info.proplist
                        .get_str(proplist::properties::APPLICATION_NAME)
                        .or_else(|| info.proplist.get_str(proplist::properties::APPLICATION_PROCESS_BINARY))
                        .unwrap_or_else(|| raw_name.clone());
                    let pid = info.proplist
                        .get_str(proplist::properties::APPLICATION_PROCESS_ID)
                        .and_then(|s| s.parse::<u32>().ok());
                    let vol = percent_from_channel_volumes(&info.volume);
                    let muted = info.mute;

                    inp.borrow_mut().push((info.index, app_name, vol, muted, pid));
                }
                ListResult::End => {
                    unsafe { (*ml_signal.as_ptr()).signal(false); }
                }
                ListResult::Error => {}
            }
        });

        loop {
            match op.get_state() {
                OpState::Done => break,
                _ => ml.wait(),
            }
        }

        let result = inputs.borrow().clone();
        result
    }
}

impl VolumeControl for PulseController {
    fn get_master_volume(&mut self) -> Result<VolumeState, String> {
        let mut ml = self.mainloop.borrow_mut();
        ml.lock();
        let ctx = self.context.borrow();
        let intro = ctx.introspect();

        // Step 1: get default sink name
        let default_name: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let dn = default_name.clone();
        let ml_ptr = Rc::clone(&self.mainloop);

        let op = intro.get_server_info(move |info| {
            *dn.borrow_mut() = info.default_sink_name.as_ref().map(|s| s.to_string());
            unsafe { (*ml_ptr.as_ptr()).signal(false); }
        });
        loop {
            match op.get_state() {
                OpState::Done => break,
                _ => ml.wait(),
            }
        }

        let sink_name = default_name.borrow().clone()
            .ok_or_else(|| "No default sink".to_string())?;

        // Step 2: get sink info by name
        let vol_out: Rc<RefCell<Option<f32>>> = Rc::new(RefCell::new(None));
        let mute_out: Rc<RefCell<Option<bool>>> = Rc::new(RefCell::new(None));
        let name_out: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

        let v = vol_out.clone();
        let m = mute_out.clone();
        let n = name_out.clone();
        let ml_ptr = Rc::clone(&self.mainloop);

        let op = intro.get_sink_info_by_name(&sink_name, move |result| {
            if let ListResult::Item(info) = result {
                *v.borrow_mut() = Some(percent_from_channel_volumes(&info.volume));
                *m.borrow_mut() = Some(info.mute);
                *n.borrow_mut() = info.name.as_ref().map(|s| s.to_string());
            }
            unsafe { (*ml_ptr.as_ptr()).signal(false); }
        });
        loop {
            match op.get_state() {
                OpState::Done => break,
                _ => ml.wait(),
            }
        }

        let volume = vol_out.borrow().unwrap_or(0.0);
        let muted = mute_out.borrow().unwrap_or(false);
        let device_name = name_out.borrow().clone()
            .unwrap_or_else(|| "Default".to_string());

        drop(intro);
        drop(ctx);

        Ok(VolumeState {
            master_volume: volume,
            muted,
            default_device_name: device_name,
        })
    }

    fn set_master_volume(&mut self, volume: f32) -> Result<(), String> {
        let clamped = volume.clamp(0.0, 100.0);
        let sink_name = self.get_default_sink_name()?;

        let mut ml = self.mainloop.borrow_mut();
        ml.lock();
        let ctx = self.context.borrow();
        let mut intro = ctx.introspect();

        let new_cv = channel_volumes_from_percent(clamped);
        let success: Rc<RefCell<Option<bool>>> = Rc::new(RefCell::new(None));
        let sc = success.clone();
        let ml_ptr = Rc::clone(&self.mainloop);

        let op = intro.set_sink_volume_by_name(&sink_name, &new_cv, Some(Box::new(move |ok| {
            *sc.borrow_mut() = Some(ok);
            unsafe { (*ml_ptr.as_ptr()).signal(false); }
        })));
        loop {
            match op.get_state() {
                OpState::Done => break,
                _ => ml.wait(),
            }
        }

        drop(intro);
        drop(ctx);
        Ok(())
    }

    fn set_muted(&mut self, muted: bool) -> Result<(), String> {
        let sink_name = self.get_default_sink_name()?;

        let mut ml = self.mainloop.borrow_mut();
        ml.lock();
        let ctx = self.context.borrow();
        let mut intro = ctx.introspect();

        let success: Rc<RefCell<Option<bool>>> = Rc::new(RefCell::new(None));
        let sc = success.clone();
        let ml_ptr = Rc::clone(&self.mainloop);

        let op = intro.set_sink_mute_by_name(&sink_name, muted, Some(Box::new(move |ok| {
            *sc.borrow_mut() = Some(ok);
            unsafe { (*ml_ptr.as_ptr()).signal(false); }
        })));
        loop {
            match op.get_state() {
                OpState::Done => break,
                _ => ml.wait(),
            }
        }

        drop(intro);
        drop(ctx);
        Ok(())
    }

    fn get_app_volumes(&mut self) -> Result<Vec<AppVolume>, String> {
        let mut ml = self.mainloop.borrow_mut();
        ml.lock();
        let ctx = self.context.borrow();
        let mut intro = ctx.introspect();

        let list = Self::get_sink_input_infos(&mut *ml, &mut intro, &self.mainloop);

        drop(intro);
        drop(ctx);

        Ok(list.into_iter().map(|(_, name, volume, muted, pid)| {
            AppVolume { name, volume, muted, pid }
        }).collect())
    }

    fn set_app_volume(&mut self, app_name: &str, volume: f32) -> Result<(), String> {
        let clamped = volume.clamp(0.0, 100.0);

        // Find the sink input index
        let index = {
            let mut ml = self.mainloop.borrow_mut();
            ml.lock();
            let ctx = self.context.borrow();
            let mut intro = ctx.introspect();

            let list = Self::get_sink_input_infos(&mut *ml, &mut intro, &self.mainloop);

            drop(intro);
            drop(ctx);

            list.iter()
                .find(|(_, name, _, _, _)| name == app_name)
                .map(|(idx, _, _, _, _)| *idx)
                .ok_or_else(|| format!("App '{}' not found", app_name))?
        };

        // Set the volume
        let mut ml = self.mainloop.borrow_mut();
        ml.lock();
        let ctx = self.context.borrow();
        let mut intro = ctx.introspect();

        let new_cv = channel_volumes_from_percent(clamped);
        let success: Rc<RefCell<Option<bool>>> = Rc::new(RefCell::new(None));
        let sc = success.clone();
        let ml_ptr = Rc::clone(&self.mainloop);

        let op = intro.set_sink_input_volume(index, &new_cv, Some(Box::new(move |ok| {
            *sc.borrow_mut() = Some(ok);
            unsafe { (*ml_ptr.as_ptr()).signal(false); }
        })));
        loop {
            match op.get_state() {
                OpState::Done => break,
                _ => ml.wait(),
            }
        }

        drop(intro);
        drop(ctx);
        Ok(())
    }

    fn set_app_muted(&mut self, app_name: &str, muted: bool) -> Result<(), String> {
        // Find the sink input index
        let index = {
            let mut ml = self.mainloop.borrow_mut();
            ml.lock();
            let ctx = self.context.borrow();
            let mut intro = ctx.introspect();

            let list = Self::get_sink_input_infos(&mut *ml, &mut intro, &self.mainloop);

            drop(intro);
            drop(ctx);

            list.iter()
                .find(|(_, name, _, _, _)| name == app_name)
                .map(|(idx, _, _, _, _)| *idx)
                .ok_or_else(|| format!("App '{}' not found", app_name))?
        };

        // Set the mute
        let mut ml = self.mainloop.borrow_mut();
        ml.lock();
        let ctx = self.context.borrow();
        let mut intro = ctx.introspect();

        let success: Rc<RefCell<Option<bool>>> = Rc::new(RefCell::new(None));
        let sc = success.clone();
        let ml_ptr = Rc::clone(&self.mainloop);

        let op = intro.set_sink_input_mute(index, muted, Some(Box::new(move |ok| {
            *sc.borrow_mut() = Some(ok);
            unsafe { (*ml_ptr.as_ptr()).signal(false); }
        })));
        loop {
            match op.get_state() {
                OpState::Done => break,
                _ => ml.wait(),
            }
        }

        drop(intro);
        drop(ctx);
        Ok(())
    }
}

pub fn create_controller() -> Box<dyn VolumeControl> {
    Box::new(PulseController::new().expect("Failed to create PulseController"))
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
