use plugin_system::{Plugin, PluginContext, PluginMetadata};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

pub trait KeySimulator: Send + Sync {
    fn simulate_keys(&self, keys: &[String]) -> Result<(), String>;
}

static LISTENING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone)]
enum KeyEvent {
    Press(String),
    Release(String),
}

type Subscribers = Arc<Mutex<Vec<mpsc::Sender<KeyEvent>>>>;

struct GlobalListener {
    subscribers: Subscribers,
    started: AtomicBool,
}

impl GlobalListener {
    fn instance() -> &'static GlobalListener {
        static INSTANCE: OnceLock<GlobalListener> = OnceLock::new();
        INSTANCE.get_or_init(|| GlobalListener {
            subscribers: Arc::new(Mutex::new(Vec::new())),
            started: AtomicBool::new(false),
        })
    }

    fn ensure_started(&self) -> Result<(), String> {
        if self.started.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        if cfg!(target_os = "linux") {
            let has_access = std::fs::read_dir("/dev/input")
                .ok()
                .and_then(|entries| {
                    entries.into_iter().find_map(|e| {
                        let path = e.ok()?.path();
                        if path.to_str()?.contains("event") {
                            std::fs::File::open(&path).ok().map(|_| true)
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or(false);

            if !has_access {
                return Err(
                    "Cannot access /dev/input/event*. Run:\n\
                     sudo usermod -a -G input $USER && sudo chmod g+r /dev/input/event* && newgrp input\n\
                     Or run with: sudo -E cargo run".to_string()
                );
            }
        }

        let subs = Arc::clone(&self.subscribers);

        let handle = thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                rdev::listen(move |event| {
                    let key_event = match event.event_type {
                        rdev::EventType::KeyPress(key) => {
                            Some(KeyEvent::Press(rdev_key_to_string(&key)))
                        }
                        rdev::EventType::KeyRelease(key) => {
                            Some(KeyEvent::Release(rdev_key_to_string(&key)))
                        }
                        _ => None,
                    };

                    if let Some(ke) = key_event {
                        if let Ok(mut guard) = subs.lock() {
                            guard.retain(|tx| tx.send(ke.clone()).is_ok());
                        }
                    }
                })
            }));

            if let Err(e) = result {
                log::error!("rdev::listen panicked: {:?}", e);
            }
        });

        thread::sleep(Duration::from_millis(100));

        if handle.is_finished() {
            return Err("Failed to start key listener. Check permissions (need root or input group on Linux)".to_string());
        }

        Ok(())
    }

    fn subscribe(&self) -> mpsc::Receiver<KeyEvent> {
        let (tx, rx) = mpsc::channel();
        self.subscribers.lock().unwrap().push(tx);
        rx
    }
}

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

    pub fn interface_ids(&self) -> Vec<&'static str> {
        vec!["KeySimulator"]
    }

    pub fn simulate_keys_plugin(&mut self, keys: &[String]) -> plugin_system::Result<()> {
        KeySimulator::simulate_keys(self, keys)
            .map_err(|e| plugin_system::PluginError::PluginNotFound { name: e })
    }

    pub fn listen_for_combo_plugin(&self, timeout_ms: u64) -> plugin_system::Result<String> {
        self.listen_for_combo(timeout_ms)
            .map_err(|e| plugin_system::PluginError::PluginNotFound { name: e })
    }

    pub fn reset_recording_state_plugin(&self) {
        Self::reset_recording_state();
    }

    pub fn reset_recording_state() {
        LISTENING.store(false, Ordering::SeqCst);
    }

    pub fn listen_for_combo(&self, timeout_ms: u64) -> Result<String, String> {
        if LISTENING.swap(true, Ordering::SeqCst) {
            return Err("Already recording".to_string());
        }

        struct ListeningGuard;
        impl Drop for ListeningGuard {
            fn drop(&mut self) {
                LISTENING.store(false, Ordering::SeqCst);
            }
        }
        let _guard = ListeningGuard;

        let gl = GlobalListener::instance();
        gl.ensure_started()?;

        let rx = gl.subscribe();

        let mut pressed: Vec<String> = Vec::new();
        let max_deadline = Instant::now() + Duration::from_millis(timeout_ms);
        let idle_timeout = Duration::from_millis(1000);
        let mut last_event_time = Instant::now();

        log::info!("Recording started, waiting for keys...");

        let result = loop {
            let now = Instant::now();

            if now >= max_deadline {
                log::info!("Max timeout reached, pressed: {:?}", pressed);
                break if !pressed.is_empty() {
                    Ok(pressed.join("+").to_lowercase())
                } else {
                    Err("Recording timed out".to_string())
                };
            }

            let time_since_last = now.duration_since(last_event_time);
            let remaining_idle = idle_timeout.saturating_sub(time_since_last);
            let remaining_max = max_deadline.saturating_duration_since(now);
            let wait = remaining_idle
                .min(remaining_max)
                .min(Duration::from_millis(50));

            match rx.recv_timeout(wait) {
                Ok(KeyEvent::Press(name)) => {
                    log::debug!("Key pressed: {}", name);
                    last_event_time = Instant::now();
                    if !pressed.contains(&name) {
                        pressed.push(name);
                    }
                }
                Ok(KeyEvent::Release(name)) => {
                    log::debug!("Key released: {}", name);
                    last_event_time = Instant::now();

                    if !is_mod_str(&name) {
                        let combo = build_combo_from(&pressed, &name);
                        log::info!("Non-modifier released, combo: {}", combo);
                        if !combo.is_empty() {
                            break Ok(combo);
                        }
                    }

                    pressed.retain(|k| k.to_lowercase() != name.to_lowercase());
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let actual_now = Instant::now();
                    let idle_elapsed = actual_now.duration_since(last_event_time);

                    if !pressed.is_empty() && idle_elapsed >= idle_timeout {
                        log::info!("Idle timeout, returning: {:?}", pressed);
                        break Ok(pressed.join("+").to_lowercase());
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    break Err("Recording channel disconnected".to_string());
                }
            }
        };

        result
    }
}

fn build_combo_from(pressed: &[String], released: &str) -> String {
    let mods: Vec<&String> = pressed.iter().filter(|k| is_mod_str(k)).collect();
    let mod_str = mods
        .iter()
        .map(|s| (*s).clone())
        .collect::<Vec<_>>()
        .join("+");

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

        fn is_mod(k: &rdev::Key) -> bool {
            is_rdev_mod(k)
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
    matches!(
        key,
        rdev::Key::ControlLeft
            | rdev::Key::ControlRight
            | rdev::Key::ShiftLeft
            | rdev::Key::ShiftRight
            | rdev::Key::Alt
            | rdev::Key::AltGr
            | rdev::Key::MetaLeft
            | rdev::Key::MetaRight
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
        log::info!("KeySimulatorPlugin loaded");
    }

    fn on_unload(&mut self) {
        log::info!("KeySimulatorPlugin unloading");
    }

    fn plugin_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }

    fn handle_command(
        &mut self,
        method: &str,
        args: serde_json::Value,
    ) -> Option<serde_json::Value> {
        match method {
            "simulate_keys" => {
                let keys: Vec<String> = args
                    .get("keys")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
                match self.simulate_keys_plugin(&keys) {
                    Ok(()) => Some(serde_json::json!({"ok": true})),
                    Err(e) => Some(serde_json::json!({"ok": false, "error": e.to_string()})),
                }
            }
            "listen_for_combo" => {
                let timeout = args
                    .get("timeout_ms")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(15000);
                match self.listen_for_combo_plugin(timeout) {
                    Ok(combo) => Some(serde_json::json!({"combo": combo})),
                    Err(e) => Some(serde_json::json!({"ok": false, "error": e.to_string()})),
                }
            }
            "reset_recording" => {
                self.reset_recording_state_plugin();
                Some(serde_json::json!({"ok": true}))
            }
            _ => None,
        }
    }

    fn interface_ids(&self) -> Vec<&'static str> {
        KeySimulatorPlugin::interface_ids(self)
    }
}
