use plugin_system::{Plugin, PluginContext, PluginMetadata};
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

pub trait KeySimulator: Send + Sync {
    fn simulate_keys(&self, keys: &[String]) -> Result<(), String>;
}

pub struct KeySimulatorPlugin;

impl Default for KeySimulatorPlugin {
    fn default() -> Self { Self }
}

impl KeySimulatorPlugin {
    pub fn new() -> Self { Self }

    pub fn listen_for_combo(timeout_ms: u64) -> Result<String, String> {
        let (tx, rx) = mpsc::channel();
        let done = Arc::new(AtomicBool::new(false));

        let _handle = thread::spawn(move || {
            let mut pressed: Vec<String> = Vec::new();
            let done_listener = Arc::clone(&done);

            let _ = rdev::listen(move |event| {
                if done_listener.load(Ordering::SeqCst) { return; }

                match event.event_type {
                    rdev::EventType::KeyPress(key) => {
                        let name = rdev_key_to_string(&key);
                        if !pressed.contains(&name) {
                            pressed.push(name);
                        }
                    }
                    rdev::EventType::KeyRelease(key) => {
                        let name = rdev_key_to_string(&key);
                        pressed.retain(|k| k != &name);

                        if !is_rdev_mod(&key) {
                            let mods: Vec<String> = pressed.iter()
                                .filter(|k| is_mod_str(k))
                                .cloned().collect();
                            let mains: Vec<String> = pressed.iter()
                                .filter(|k| !is_mod_str(k))
                                .cloned().collect();
                            let combo = if mains.is_empty() { 
                                mods.join("+") 
                            } else {
                                [mods.join("+"), mains.join("+")].iter()
                                    .filter(|s| !s.is_empty())
                                    .cloned().collect::<Vec<_>>()
                                    .join("+")
                            };
                            if !combo.is_empty() {
                                let _ = tx.send(combo.to_lowercase());
                            }
                        }
                    }
                    _ => {}
                }
            });
        });

        rx.recv_timeout(Duration::from_millis(timeout_ms))
            .map_err(|_| "Recording timed out".to_string())
    }

    pub fn list_input_devices() -> Vec<(String, String)> {
        let mut devices = Vec::new();
        if let Ok(content) = std::fs::read_to_string("/proc/bus/input/devices") {
        let mut current_name = String::new();
        for line in content.lines() {
                if line.starts_with("N: Name=") {
                    current_name = line.trim_start_matches("N: Name=\"").trim_end_matches('"').to_string();
                }
                if line.starts_with("H: Handlers=") {
                    if let Some(pos) = line.find("event") {
                        let event_num = line[pos..].split_whitespace().next().unwrap_or("");
                        if !current_name.is_empty() && !event_num.is_empty() {
                            devices.push((
                                format!("/dev/input/{}", event_num),
                                current_name.clone(),
                            ));
                        }
                    }
                    current_name.clear();
                }
            }
        }
        if devices.is_empty() {
            devices.push(("/dev/input/by-path/platform-i8042-serio-0-event-kbd".to_string(), "PS/2 Keyboard".to_string()));
            devices.push(("/dev/input/by-path/platform-i8042-serio-1-event-kbd".to_string(), "PS/2 Mouse".to_string()));
        }
        devices
    }
}

fn rdev_key_to_string(key: &rdev::Key) -> String {
    use rdev::Key;
    match key {
        Key::ControlLeft | Key::ControlRight => "Ctrl".to_string(),
        Key::ShiftLeft | Key::ShiftRight => "Shift".to_string(),
        Key::Alt => "Alt".to_string(),
        Key::AltGr => "AltGr".to_string(),
        Key::MetaLeft | Key::MetaRight => "Win".to_string(),
        Key::Space => "Space".to_string(),
        Key::Return => "Enter".to_string(),
        Key::Tab => "Tab".to_string(),
        Key::Escape => "Esc".to_string(),
        Key::Backspace => "Backspace".to_string(),
        Key::Delete => "Del".to_string(),
        Key::Home => "Home".to_string(),
        Key::End => "End".to_string(),
        Key::PageUp => "PageUp".to_string(),
        Key::PageDown => "PageDown".to_string(),
        Key::UpArrow => "Up".to_string(),
        Key::DownArrow => "Down".to_string(),
        Key::LeftArrow => "Left".to_string(),
        Key::RightArrow => "Right".to_string(),
        Key::F1 => "F1".to_string(),
        Key::F2 => "F2".to_string(),
        Key::F3 => "F3".to_string(),
        Key::F4 => "F4".to_string(),
        Key::F5 => "F5".to_string(),
        Key::F6 => "F6".to_string(),
        Key::F7 => "F7".to_string(),
        Key::F8 => "F8".to_string(),
        Key::F9 => "F9".to_string(),
        Key::F10 => "F10".to_string(),
        Key::F11 => "F11".to_string(),
        Key::F12 => "F12".to_string(),
        _ => format!("{:?}", key).replace("Key", ""),
    }
}

fn is_rdev_mod(key: &rdev::Key) -> bool {
    matches!(key,
        rdev::Key::ControlLeft | rdev::Key::ControlRight |
        rdev::Key::ShiftLeft | rdev::Key::ShiftRight |
        rdev::Key::Alt | rdev::Key::AltGr |
        rdev::Key::MetaLeft | rdev::Key::MetaRight
    )
}

fn is_mod_str(key: &str) -> bool {
    matches!(key, "Ctrl" | "Shift" | "Alt" | "AltGr" | "Win")
}

impl KeySimulator for KeySimulatorPlugin {
    fn simulate_keys(&self, keys: &[String]) -> Result<(), String> {
        let rdev_keys: Vec<rdev::Key> = keys.iter().filter_map(|k| map_key_to_rdev(k)).collect();

        if rdev_keys.is_empty() {
            return Err("No mappable keys".to_string());
        }

        let mods: Vec<&rdev::Key> = rdev_keys.iter().filter(|k| is_rdev_mod(k)).collect();
        let mains: Vec<&rdev::Key> = rdev_keys.iter().filter(|k| !is_rdev_mod(k)).collect();

        for m in &mods {
            rdev::simulate(&rdev::EventType::KeyPress(*(*m)))
                .map_err(|e| format!("Modifier press failed: {}", e))?;
        }

        for k in &mains {
            rdev::simulate(&rdev::EventType::KeyPress(*(*k)))
                .map_err(|e| format!("Key press failed: {}", e))?;
            thread::sleep(Duration::from_millis(10));
            rdev::simulate(&rdev::EventType::KeyRelease(*(*k)))
                .map_err(|e| format!("Key release failed: {}", e))?;
        }

        for m in mods.iter().rev() {
            rdev::simulate(&rdev::EventType::KeyRelease(*(*m)))
                .map_err(|e| format!("Modifier release failed: {}", e))?;
        }

        Ok(())
    }
}

fn map_key_to_rdev(key: &str) -> Option<rdev::Key> {
    use rdev::Key;
    Some(match key.to_lowercase().as_str() {
        "ctrl" => Key::ControlLeft,
        "shift" => Key::ShiftLeft,
        "alt" => Key::Alt,
        "altgr" => Key::AltGr,
        "win" | "meta" | "super" => Key::MetaLeft,
        "space" => Key::Space,
        "enter" | "return" => Key::Return,
        "tab" => Key::Tab,
        "escape" | "esc" => Key::Escape,
        "backspace" => Key::Backspace,
        "delete" | "del" => Key::Delete,
        "home" => Key::Home,
        "end" => Key::End,
        "pageup" => Key::PageUp,
        "pagedown" => Key::PageDown,
        "up" => Key::UpArrow,
        "down" => Key::DownArrow,
        "left" => Key::LeftArrow,
        "right" => Key::RightArrow,
        "f1" => Key::F1, "f2" => Key::F2, "f3" => Key::F3,
        "f4" => Key::F4, "f5" => Key::F5, "f6" => Key::F6,
        "f7" => Key::F7, "f8" => Key::F8, "f9" => Key::F9,
        "f10" => Key::F10, "f11" => Key::F11, "f12" => Key::F12,
        _ => {
            let upper = key.to_uppercase();
            let c = upper.as_str();
            match c {
                "A" => Key::KeyA, "B" => Key::KeyB, "C" => Key::KeyC,
                "D" => Key::KeyD, "E" => Key::KeyE, "F" => Key::KeyF,
                "G" => Key::KeyG, "H" => Key::KeyH, "I" => Key::KeyI,
                "J" => Key::KeyJ, "K" => Key::KeyK, "L" => Key::KeyL,
                "M" => Key::KeyM, "N" => Key::KeyN, "O" => Key::KeyO,
                "P" => Key::KeyP, "Q" => Key::KeyQ, "R" => Key::KeyR,
                "S" => Key::KeyS, "T" => Key::KeyT, "U" => Key::KeyU,
                "V" => Key::KeyV, "W" => Key::KeyW, "X" => Key::KeyX,
                "Y" => Key::KeyY, "Z" => Key::KeyZ,
                "0" => Key::Num0, "1" => Key::Num1, "2" => Key::Num2,
                "3" => Key::Num3, "4" => Key::Num4, "5" => Key::Num5,
                "6" => Key::Num6, "7" => Key::Num7, "8" => Key::Num8,
                "9" => Key::Num9,
                _ => return None,
            }
        }
    })
}

#[plugin_system::plugin_export]
impl Plugin for KeySimulatorPlugin {
    fn metadata(&self) -> PluginMetadata {
        plugin_system::plugin_metadata! {
            name: "key-simulator",
            version: "0.1.0",
            authors: ["StreamDeck Core"],
            dependencies: []
        }
    }

    fn on_load(&mut self, _ctx: &PluginContext) {
        log::info!("KeySimulatorPlugin loaded (rdev key simulation + recording)");
    }

    fn on_unload(&mut self) {
        log::info!("KeySimulatorPlugin unloading");
    }

    fn plugin_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn interface_ids(&self) -> Vec<&'static str> {
        vec!["KeySimulator"]
    }
}