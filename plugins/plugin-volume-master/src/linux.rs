use std::process::Command;
use std::sync::{Arc, Mutex};

use crate::{AppVolume, VolumeControl, VolumeState};

pub struct PulseController {
    cache: Arc<Mutex<Option<VolumeState>>>,
}

pub fn create_controller() -> Box<dyn VolumeControl> {
    Box::new(PulseController {
        cache: Arc::new(Mutex::new(None)),
    })
}

pub fn per_app_supported() -> bool {
    true
}

fn run_pactl(args: &[&str]) -> Result<String, String> {
    let output = Command::new("pactl")
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run pactl: {}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(format!(
            "pactl failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

fn parse_volume(s: &str) -> Option<f32> {
    for line in s.lines() {
        if line.contains("Volume:") {
            for part in line.split_whitespace() {
                if part.ends_with('%') {
                    if let Ok(v) = part.trim_end_matches('%').parse::<f32>() {
                        return Some(v);
                    }
                }
            }
        }
    }
    None
}

fn get_default_sink_name() -> Option<String> {
    Command::new("pactl")
        .args(["get-default-sink"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let name = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if name.is_empty() { None } else { Some(name) }
            } else {
                None
            }
        })
}

impl VolumeControl for PulseController {
    fn get_master_volume(&self) -> Result<VolumeState, String> {
        let output = run_pactl(&["get-sink-volume", "@DEFAULT_SINK@"])?;
        let volume = parse_volume(&output).unwrap_or(0.0);

        let mute_output = run_pactl(&["get-sink-mute", "@DEFAULT_SINK@"])?;
        let muted = mute_output.contains("yes");

        let device_name = get_default_sink_name()
            .unwrap_or_else(|| "Default".to_string());

        let state = VolumeState {
            master_volume: volume,
            muted,
            default_device_name: device_name,
        };

        *self.cache.lock().unwrap() = Some(state.clone());
        Ok(state)
    }

    fn set_master_volume(&mut self, volume: f32) -> Result<(), String> {
        let clamped = volume.clamp(0.0, 100.0);
        run_pactl(&[
            "set-sink-volume",
            "@DEFAULT_SINK@",
            &format!("{}%", clamped as i32),
        ])?;
        Ok(())
    }

    fn set_muted(&mut self, muted: bool) -> Result<(), String> {
        let val = if muted { "1" } else { "0" };
        run_pactl(&["set-sink-mute", "@DEFAULT_SINK@", val])?;
        Ok(())
    }

    fn get_app_volumes(&self) -> Result<Vec<AppVolume>, String> {
        let output = run_pactl(&["list", "sink-inputs"])?;
        let mut apps = Vec::new();
        let mut current_app: Option<(String, Option<u32>)> = None;
        let mut current_vol = 0.0f32;
        let mut current_muted = false;

        for line in output.lines() {
            let line = line.trim();

            if line.starts_with("Sink Input #") {
                if let Some((name, pid)) = current_app.take() {
                    apps.push(AppVolume {
                        name,
                        volume: current_vol,
                        muted: current_muted,
                        pid,
                    });
                }
                current_vol = 0.0;
                current_muted = false;
            } else if line.starts_with("application.process.binary")
                || line.starts_with("application.name")
            {
                if let Some(val) = line.split('=').nth(1) {
                    let val = val.trim().trim_matches('"');
                    if current_app.is_none() {
                        current_app = Some((val.to_string(), None));
                    }
                }
            } else if line.starts_with("application.process.id") {
                if let Some(val) = line.split('=').nth(1) {
                    if let Ok(pid) = val.trim().parse::<u32>() {
                        if let Some(ref mut app) = current_app {
                            app.1 = Some(pid);
                        }
                    }
                }
            } else if line.contains("Volume:") {
                if let Some(v) = parse_volume(line) {
                    current_vol = v;
                }
            } else if line.contains("Muted:") {
                current_muted = line.contains("yes");
            }
        }

        if let Some((name, pid)) = current_app.take() {
            apps.push(AppVolume {
                name,
                volume: current_vol,
                muted: current_muted,
                pid,
            });
        }

        Ok(apps)
    }

    fn set_app_volume(&mut self, app_name: &str, volume: f32) -> Result<(), String> {
        let apps = self.get_app_volumes()?;
        let app = apps
            .iter()
            .find(|a| a.name == app_name)
            .ok_or_else(|| format!("App '{}' not found", app_name))?;

        if let Some(pid) = app.pid {
            let output = run_pactl(&["list", "sink-inputs"])?;
            for line in output.lines() {
                if line.contains(&format!("application.process.id = {}", pid)) {
                    if let Some(idx) = line
                        .split('#')
                        .nth(1)
                        .and_then(|s| s.split_whitespace().next())
                    {
                        run_pactl(&["set-sink-input-volume", idx, &format!("{}%", volume as i32)])?;
                        return Ok(());
                    }
                }
            }
        }

        Err(format!("Could not find sink-input for app '{}'", app_name))
    }

    fn set_app_muted(&mut self, app_name: &str, muted: bool) -> Result<(), String> {
        let apps = self.get_app_volumes()?;
        let app = apps
            .iter()
            .find(|a| a.name == app_name)
            .ok_or_else(|| format!("App '{}' not found", app_name))?;

        if let Some(pid) = app.pid {
            let output = run_pactl(&["list", "sink-inputs"])?;
            for line in output.lines() {
                if line.contains(&format!("application.process.id = {}", pid)) {
                    if let Some(idx) = line
                        .split('#')
                        .nth(1)
                        .and_then(|s| s.split_whitespace().next())
                    {
                        let val = if muted { "1" } else { "0" };
                        run_pactl(&["set-sink-input-mute", idx, val])?;
                        return Ok(());
                    }
                }
            }
        }

        Err(format!("Could not find sink-input for app '{}'", app_name))
    }
}
