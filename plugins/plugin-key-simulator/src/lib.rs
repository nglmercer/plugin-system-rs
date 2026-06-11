use plugin_system::{Plugin, PluginContext, PluginMetadata};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use std::collections::HashMap;

pub trait KeySimulator: Send + Sync {
    fn simulate_keys(&self, keys: &[String]) -> Result<(), String>;
}

pub struct RecordSession {
    pub current_combo: Arc<Mutex<String>>,
    pub cancel: Arc<AtomicBool>,
    pub done: Arc<AtomicBool>,
    pub result: Arc<Mutex<Option<String>>>,
}

impl RecordSession {
    pub fn current(&self) -> String {
        self.current_combo.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    pub fn is_done(&self) -> bool {
        self.done.load(Ordering::SeqCst)
    }

    pub fn wait_for_result(&self, timeout: Duration) -> Result<String, String> {
        let start = std::time::Instant::now();
        loop {
            if start.elapsed() > timeout {
                self.cancel();
                return Err("Recording timed out".to_string());
            }
            {
                let guard = self.result.lock().unwrap_or_else(|e| e.into_inner());
                if let Some(ref combo) = *guard {
                    return Ok(combo.clone());
                }
            }
            if self.cancel.load(Ordering::SeqCst) {
                return Err("Cancelled".to_string());
            }
            thread::sleep(Duration::from_millis(50));
        }
    }
}

static SESSIONS: once_cell::sync::Lazy<Mutex<HashMap<String, Arc<RecordSession>>>> =
    once_cell::sync::Lazy::new(|| Mutex::new(HashMap::new()));
static LISTENING: AtomicBool = AtomicBool::new(false);

pub struct KeySimulatorPlugin;

impl Default for KeySimulatorPlugin {
    fn default() -> Self { Self }
}

impl KeySimulatorPlugin {
    pub fn new() -> Self { Self }

    pub fn generate_session_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        format!("rec_{:x}", ts)
    }

    pub fn start_recording(timeout_ms: u64) -> String {
        // Cancel any existing recording first (rdev can only listen on one at a time)
        if LISTENING.load(Ordering::SeqCst) {
            if let Ok(sessions) = SESSIONS.lock() {
                for (id, s) in sessions.iter() {
                    s.cancel();
                    log::info!("[KeySimulator] Cancelling previous session: {}", id);
                }
            }
        }
        LISTENING.store(true, Ordering::SeqCst);

        let session_id = Self::generate_session_id();
        let session = Arc::new(RecordSession {
            current_combo: Arc::new(Mutex::new(String::new())),
            cancel: Arc::new(AtomicBool::new(false)),
            done: Arc::new(AtomicBool::new(false)),
            result: Arc::new(Mutex::new(None)),
        });

        let session_clone = Arc::clone(&session);
        let timeout_clone = timeout_ms;

        thread::spawn(move || {
            let start = std::time::Instant::now();
            let mut pressed: Vec<String> = Vec::new();
            let session = session_clone;

            let _ = rdev::listen(move |event| {
                if session.cancel.load(Ordering::SeqCst) { return; }
                if start.elapsed() > Duration::from_millis(timeout_clone) {
                    session.cancel.store(true, Ordering::SeqCst);
                    return;
                }

                match event.event_type {
                    rdev::EventType::KeyPress(key) => {
                        let name = rdev_key_to_string(&key);
                        if !pressed.contains(&name) {
                            pressed.push(name);
                        }
                        let combo = KeySimulatorPlugin::build_combo(&pressed);
                        *session.current_combo.lock().unwrap_or_else(|e| e.into_inner()) = combo;
                    }
                    rdev::EventType::KeyRelease(key) => {
                        let name = rdev_key_to_string(&key);

                        if !is_rdev_mod(&key) {
                            let combo = KeySimulatorPlugin::build_combo(&pressed);
                            if !combo.is_empty() {
                                let lower = combo.to_lowercase();
                                *session.result.lock().unwrap_or_else(|e| e.into_inner()) = Some(lower.clone());
                                *session.current_combo.lock().unwrap_or_else(|e| e.into_inner()) = lower;
                                session.done.store(true, Ordering::SeqCst);
                            }
                        }

                        pressed.retain(|k| k != &name);
                        let combo = KeySimulatorPlugin::build_combo(&pressed);
                        *session.current_combo.lock().unwrap_or_else(|e| e.into_inner()) = combo;
                    }
                    _ => {}
                }
            });
        });

        if let Ok(mut sessions) = SESSIONS.lock() {
            sessions.insert(session_id.clone(), session);
        }

        session_id
    }

    pub fn get_session(session_id: &str) -> Option<Arc<RecordSession>> {
        SESSIONS.lock().ok().and_then(|s| s.get(session_id).cloned())
    }

    pub fn remove_session(session_id: &str) {
        if let Ok(mut sessions) = SESSIONS.lock() {
            sessions.remove(session_id);
        }
        LISTENING.store(false, Ordering::SeqCst);
    }

    pub fn build_combo(pressed: &[String]) -> String {
        let mods: Vec<&String> = pressed.iter().filter(|k| is_mod_str(k)).collect();
        let mains: Vec<&String> = pressed.iter().filter(|k| !is_mod_str(k)).collect();

        let mod_str = mods.iter().map(|s| (*s).clone()).collect::<Vec<_>>().join("+");
        let main_str = mains.iter().map(|s| (*s).clone()).collect::<Vec<_>>().join("+");

        if main_str.is_empty() {
            mod_str
        } else if mod_str.is_empty() {
            main_str
        } else {
            format!("{}+{}", mod_str, main_str)
        }
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