use plugin_system::{Plugin, PluginContext, PluginMetadata};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub trait KeySimulator: Send + Sync {
    fn simulate_keys(&self, keys: &[String]) -> Result<(), String>;
}

static LISTENING: AtomicBool = AtomicBool::new(false);

pub struct KeySimulatorPlugin;

impl Default for KeySimulatorPlugin {
    fn default() -> Self { Self }
}

impl KeySimulatorPlugin {
    pub fn new() -> Self { Self }

    pub fn listen_for_combo(timeout_ms: u64) -> Result<String, String> {
        if LISTENING.swap(true, Ordering::SeqCst) {
            return Err("Already recording".to_string());
        }

        let (tx, rx) = mpsc::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let cancel_clone = Arc::clone(&cancel);

        thread::spawn(move || {
            let start = std::time::Instant::now();
            let mut pressed: Vec<String> = Vec::new();
            let cancel = cancel_clone;

            let _ = rdev::listen(move |event| {
                if cancel.load(Ordering::SeqCst) { return; }
                if start.elapsed() > Duration::from_millis(timeout_ms) {
                    let _ = tx.send(Err("Timed out".to_string()));
                    cancel.store(true, Ordering::SeqCst);
                    return;
                }

                match event.event_type {
                    rdev::EventType::KeyPress(key) => {
                        let name = rdev_key_to_string(&key);
                        if !pressed.contains(&name) {
                            pressed.push(name);
                        }
                    }
                    rdev::EventType::KeyRelease(key) => {
                        let name = rdev_key_to_string(&key);

                        if !is_rdev_mod(&key) {
                            let combo = build_combo_from(&pressed, &name);
                            if !combo.is_empty() {
                                let _ = tx.send(Ok(combo.to_lowercase()));
                                cancel.store(true, Ordering::SeqCst);
                                return;
                            }
                        }

                        pressed.retain(|k| k != &name);
                    }
                    _ => {}
                }
            });
        });

        let result = rx.recv_timeout(Duration::from_millis(timeout_ms + 2000))
            .unwrap_or(Err("Recording timed out".to_string()));

        LISTENING.store(false, Ordering::SeqCst);
        result
    }
}

fn build_combo_from(pressed: &[String], released: &str) -> String {
    let mods: Vec<&String> = pressed.iter().filter(|k| is_mod_str(k)).collect();
    let mod_str = mods.iter().map(|s| (*s).clone()).collect::<Vec<_>>().join("+");

    let main_str = if is_mod_str(released) {
        String::new()
    } else {
        released.to_lowercase()
    };

    if main_str.is_empty() {
        mod_str.to_lowercase()
    } else if mod_str.is_empty() {
        main_str
    } else {
        format!("{}+{}", mod_str.to_lowercase(), main_str)
    }
}

impl KeySimulator for KeySimulatorPlugin {
    fn simulate_keys(&self, keys: &[String]) -> Result<(), String> {
        let rdev_keys: Vec<rdev::Key> = keys.iter().filter_map(|k| map_key_to_rdev(k)).collect();

        if rdev_keys.is_empty() {
            return Err("No mappable keys".to_string());
        }

        fn is_mod(k: &rdev::Key) -> bool { is_rdev_mod(k) }
        let mods: Vec<&rdev::Key> = rdev_keys.iter().filter(|k| is_mod(k)).collect();
        let mains: Vec<&rdev::Key> = rdev_keys.iter().filter(|k| !is_mod(k)).collect();

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
        Key::F1 => "F1".to_string(), Key::F2 => "F2".to_string(),
        Key::F3 => "F3".to_string(), Key::F4 => "F4".to_string(),
        Key::F5 => "F5".to_string(), Key::F6 => "F6".to_string(),
        Key::F7 => "F7".to_string(), Key::F8 => "F8".to_string(),
        Key::F9 => "F9".to_string(), Key::F10 => "F10".to_string(),
        Key::F11 => "F11".to_string(), Key::F12 => "F12".to_string(),
        _ => format!("{:?}", key).replace("Key", ""),
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
        "home" => Key::Home, "end" => Key::End,
        "pageup" => Key::PageUp, "pagedown" => Key::PageDown,
        "up" => Key::UpArrow, "down" => Key::DownArrow,
        "left" => Key::LeftArrow, "right" => Key::RightArrow,
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
        log::info!("KeySimulatorPlugin loaded");
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
