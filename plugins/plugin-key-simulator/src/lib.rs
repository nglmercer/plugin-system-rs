use plugin_interfaces::KeySimulator;
use plugin_system::{Plugin, PluginContext, PluginMetadata};

pub struct KeySimulatorPlugin;

impl Default for KeySimulatorPlugin {
    fn default() -> Self {
        Self
    }
}

impl KeySimulatorPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl KeySimulator for KeySimulatorPlugin {
    fn simulate_keys(&self, keys: &[String]) -> Result<(), String> {
        let rdev_keys: Vec<rdev::Key> = keys.iter().filter_map(|k| map_key_to_rdev(k)).collect();

        if rdev_keys.is_empty() {
            return Err("No mappable keys".to_string());
        }

        fn is_mod(k: &rdev::Key) -> bool {
            matches!(
                k,
                rdev::Key::ControlLeft
                    | rdev::Key::ControlRight
                    | rdev::Key::ShiftLeft
                    | rdev::Key::ShiftRight
                    | rdev::Key::Alt
                    | rdev::Key::MetaLeft
                    | rdev::Key::MetaRight
            )
        }

        let mods: Vec<&rdev::Key> = rdev_keys.iter().filter(|k| is_mod(k)).collect();
        let mains: Vec<&rdev::Key> = rdev_keys.iter().filter(|k| !is_mod(k)).collect();

        for m in &mods {
            rdev::simulate(&rdev::EventType::KeyPress(*(*m)))
                .map_err(|e| format!("Modifier press failed: {}", e))?;
        }

        for k in &mains {
            rdev::simulate(&rdev::EventType::KeyPress(*(*k)))
                .map_err(|e| format!("Key press failed: {}", e))?;
            std::thread::sleep(std::time::Duration::from_millis(10));
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
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        _ => {
            let upper = key.to_uppercase();
            let c = upper.as_str();
            match c {
                "A" => Key::KeyA,
                "B" => Key::KeyB,
                "C" => Key::KeyC,
                "D" => Key::KeyD,
                "E" => Key::KeyE,
                "F" => Key::KeyF,
                "G" => Key::KeyG,
                "H" => Key::KeyH,
                "I" => Key::KeyI,
                "J" => Key::KeyJ,
                "K" => Key::KeyK,
                "L" => Key::KeyL,
                "M" => Key::KeyM,
                "N" => Key::KeyN,
                "O" => Key::KeyO,
                "P" => Key::KeyP,
                "Q" => Key::KeyQ,
                "R" => Key::KeyR,
                "S" => Key::KeyS,
                "T" => Key::KeyT,
                "U" => Key::KeyU,
                "V" => Key::KeyV,
                "W" => Key::KeyW,
                "X" => Key::KeyX,
                "Y" => Key::KeyY,
                "Z" => Key::KeyZ,
                "0" => Key::Num0,
                "1" => Key::Num1,
                "2" => Key::Num2,
                "3" => Key::Num3,
                "4" => Key::Num4,
                "5" => Key::Num5,
                "6" => Key::Num6,
                "7" => Key::Num7,
                "8" => Key::Num8,
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
        log::info!("KeySimulatorPlugin loaded (rdev key simulation)");
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
