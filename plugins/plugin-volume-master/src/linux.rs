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

fn parse_volume_percent(s: &str) -> Option<f32> {
    for part in s.split_whitespace() {
        if part.ends_with('%') {
            if let Ok(v) = part.trim_end_matches('%').trim().parse::<f32>() {
                return Some(v);
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
                if name.is_empty() {
                    None
                } else {
                    Some(name)
                }
            } else {
                None
            }
        })
}

struct SinkInput {
    index: u32,
    name: String,
    pid: Option<u32>,
    volume: f32,
    muted: bool,
}

fn parse_sink_inputs(output: &str) -> Vec<SinkInput> {
    let mut inputs = Vec::new();
    let mut current_index: Option<u32> = None;
    let mut current_name: Option<String> = None;
    let mut current_pid: Option<u32> = None;
    let mut current_vol: f32 = 0.0;
    let mut current_muted = false;

    for line in output.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("Sink Input #") {
            if let Some(idx) = current_index.take() {
                inputs.push(SinkInput {
                    index: idx,
                    name: current_name.take().unwrap_or_else(|| "Unknown".to_string()),
                    pid: current_pid.take(),
                    volume: current_vol,
                    muted: current_muted,
                });
            }
            if let Some(num) = trimmed.strip_prefix("Sink Input #") {
                current_index = num.split_whitespace().next().and_then(|s| s.parse().ok());
            }
            current_name = None;
            current_pid = None;
            current_vol = 0.0;
            current_muted = false;
        } else if trimmed.starts_with("application.name")
            || trimmed.starts_with("application.process.binary")
        {
            if current_name.is_none() {
                if let Some(val) = trimmed.split('=').nth(1) {
                    current_name = Some(val.trim().trim_matches('"').to_string());
                }
            }
        } else if trimmed.starts_with("application.process.id") {
            if let Some(val) = trimmed.split('=').nth(1) {
                if let Ok(pid) = val.trim().trim_matches('"').parse::<u32>() {
                    current_pid = Some(pid);
                }
            }
        } else if trimmed.contains("Volume:") {
            if let Some(v) = parse_volume_percent(trimmed) {
                current_vol = v;
            }
        } else if trimmed.starts_with("Mute:") {
            current_muted = trimmed.contains("yes");
        }
    }

    if let Some(idx) = current_index {
        inputs.push(SinkInput {
            index: idx,
            name: current_name.unwrap_or_else(|| "Unknown".to_string()),
            pid: current_pid,
            volume: current_vol,
            muted: current_muted,
        });
    }

    inputs
}

impl VolumeControl for PulseController {
    fn get_master_volume(&self) -> Result<VolumeState, String> {
        let output = run_pactl(&["get-sink-volume", "@DEFAULT_SINK@"])?;
        let volume = parse_volume_percent(&output).unwrap_or(0.0);

        let mute_output = run_pactl(&["get-sink-mute", "@DEFAULT_SINK@"])?;
        let muted = mute_output.contains("yes");

        let device_name = get_default_sink_name().unwrap_or_else(|| "Default".to_string());

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
        let inputs = parse_sink_inputs(&output);

        Ok(inputs
            .into_iter()
            .map(|inp| AppVolume {
                name: inp.name,
                volume: inp.volume,
                muted: inp.muted,
                pid: inp.pid,
            })
            .collect())
    }

    fn set_app_volume(&mut self, app_name: &str, volume: f32) -> Result<(), String> {
        let output = run_pactl(&["list", "sink-inputs"])?;
        let inputs = parse_sink_inputs(&output);

        let input = inputs
            .iter()
            .find(|i| i.name == app_name)
            .ok_or_else(|| format!("App '{}' not found", app_name))?;

        let idx = input.index.to_string();
        let clamped = volume.clamp(0.0, 100.0);
        run_pactl(&[
            "set-sink-input-volume",
            &idx,
            &format!("{}%", clamped as i32),
        ])?;
        Ok(())
    }

    fn set_app_muted(&mut self, app_name: &str, muted: bool) -> Result<(), String> {
        let output = run_pactl(&["list", "sink-inputs"])?;
        let inputs = parse_sink_inputs(&output);

        let input = inputs
            .iter()
            .find(|i| i.name == app_name)
            .ok_or_else(|| format!("App '{}' not found", app_name))?;

        let idx = input.index.to_string();
        let val = if muted { "1" } else { "0" };
        run_pactl(&["set-sink-input-mute", &idx, val])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_volume_percent() {
        assert_eq!(
            parse_volume_percent("Volume: front-left: 52429 /  80% / -5,81 dB"),
            Some(80.0)
        );
        assert_eq!(
            parse_volume_percent("Volume: front-left: 22702 /  35% / -27,62 dB"),
            Some(35.0)
        );
        assert_eq!(parse_volume_percent("no volume here"), None);
        assert_eq!(parse_volume_percent("Volume: 0%"), Some(0.0));
        assert_eq!(parse_volume_percent("Volume: 100%"), Some(100.0));
    }

    #[test]
    fn test_parse_sink_inputs_empty() {
        let inputs = parse_sink_inputs("");
        assert!(inputs.is_empty());
    }

    #[test]
    fn test_parse_sink_inputs_single() {
        let output = r#"Sink Input #11139
	Driver: PipeWire
	Owner Module: n/a
	Client: 163
	Sink: 2182
	Volume: front-left: 22702 /  35% / -27,62 dB,   front-right: 22702 /  35% / -27,62 dB
	        balance 0,00
	Mute: no
	Properties:
		application.name = "Firefox"
		application.process.id = "2231"
		application.process.binary = "firefox"
"#;
        let inputs = parse_sink_inputs(output);
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].index, 11139);
        assert_eq!(inputs[0].name, "Firefox");
        assert_eq!(inputs[0].pid, Some(2231));
        assert_eq!(inputs[0].volume, 35.0);
        assert!(!inputs[0].muted);
    }

    #[test]
    fn test_parse_sink_inputs_multiple() {
        let output = r#"Sink Input #100
	Mute: no
	Volume: front-left: 32768 /  50% / -6,00 dB
	Properties:
		application.name = "Firefox"
		application.process.id = "1000"
Sink Input #200
	Mute: yes
	Volume: front-left: 16384 /  25% / -12,00 dB
	Properties:
		application.name = "Spotify"
		application.process.id = "2000"
"#;
        let inputs = parse_sink_inputs(output);
        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0].name, "Firefox");
        assert_eq!(inputs[0].volume, 50.0);
        assert!(!inputs[0].muted);
        assert_eq!(inputs[1].name, "Spotify");
        assert_eq!(inputs[1].volume, 25.0);
        assert!(inputs[1].muted);
    }

    #[test]
    fn test_parse_sink_inputs_no_props() {
        let output = r#"Sink Input #500
	Volume: front-left: 40000 /  61%
	Mute: no
"#;
        let inputs = parse_sink_inputs(output);
        assert_eq!(inputs.len(), 1);
        assert_eq!(inputs[0].name, "Unknown");
        assert_eq!(inputs[0].pid, None);
    }
}
