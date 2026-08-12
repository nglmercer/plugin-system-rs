//! `input` capability: synthetic keyboard events, backed by `rdev`.
//!
//! This is the code that used to live in `plugin-key-simulator`. A component
//! cannot reach X11, uinput or the Win32 input queue, so the mapping from key
//! names to platform key codes — and the press/release ordering that makes a
//! chord work — moved here.
//!
//! # This is the sharpest capability
//!
//! A plugin granted `input` can type anything into whichever window has focus.
//! There is no way to scope that down to "only these keys" or "only this
//! window" through `rdev`, so the grant is all-or-nothing and should be given
//! deliberately. The sandbox constrains what a plugin can *reach*; once it can
//! reach the keyboard, it can use it fully.

use std::thread;
use std::time::Duration;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use plugin_system::InputProvider;

/// How long a key is held before release.
///
/// Some applications sample the keyboard rather than reading events, and miss
/// a press and release delivered in the same tick.
const KEY_HOLD: Duration = Duration::from_millis(10);

/// Synthetic keyboard input via `rdev`.
pub struct RdevProvider;

impl RdevProvider {
    pub fn new() -> Self {
        Self
    }

    /// Press modifiers, tap each main key, then release modifiers in reverse.
    ///
    /// Reverse order on release matters: releasing Ctrl before Shift in a
    /// Ctrl+Shift+T leaves some window managers believing Shift is still down.
    fn press_chord(keys: &[rdev::Key]) -> Result<(), String> {
        if keys.is_empty() {
            return Err("no mappable keys".into());
        }

        let (mods, mains): (Vec<&rdev::Key>, Vec<&rdev::Key>) =
            keys.iter().partition(|k| is_rdev_mod(k));

        for m in &mods {
            rdev::simulate(&rdev::EventType::KeyPress(**m))
                .map_err(|e| format!("modifier press failed: {e}"))?;
        }

        for k in &mains {
            rdev::simulate(&rdev::EventType::KeyPress(**k))
                .map_err(|e| format!("key press failed: {e}"))?;
            thread::sleep(KEY_HOLD);
            rdev::simulate(&rdev::EventType::KeyRelease(**k))
                .map_err(|e| format!("key release failed: {e}"))?;
        }

        for m in mods.iter().rev() {
            rdev::simulate(&rdev::EventType::KeyRelease(**m))
                .map_err(|e| format!("modifier release failed: {e}"))?;
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Recording
//
// Reading the keyboard needs a process-wide `rdev::listen` thread, which can
// only be started once. The listener below is therefore a singleton that
// fans events out to whichever recording is in progress.
// ---------------------------------------------------------------------------

/// Guards against two concurrent recordings racing for the same keystrokes.
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


impl RdevProvider {
    /// Watch the keyboard until a chord is completed or the clock runs out.
    ///
    /// Lifted from the native plugin unchanged in behaviour: a chord ends when
    /// a non-modifier is released, or after a second of idleness with keys
    /// still held (so a user holding Ctrl+Shift gets a result without having
    /// to release into a race).
    fn record(timeout_ms: u32) -> Result<String, String> {
        if LISTENING.swap(true, Ordering::SeqCst) {
            Err("Already recording".to_string())
        } else {
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
            let max_deadline = Instant::now() + Duration::from_millis(timeout_ms as u64);
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

}

impl Default for RdevProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl InputProvider for RdevProvider {
    fn send_key(&self, key: &str) -> Result<(), String> {
        let mapped = map_key_to_rdev(key).ok_or_else(|| format!("unknown key '{key}'"))?;
        Self::press_chord(&[mapped])
    }

    fn send_hotkey(&self, modifiers: &[String], key: &str) -> Result<(), String> {
        let mut keys = Vec::with_capacity(modifiers.len() + 1);
        for m in modifiers {
            keys.push(map_key_to_rdev(m).ok_or_else(|| format!("unknown modifier '{m}'"))?);
        }
        keys.push(map_key_to_rdev(key).ok_or_else(|| format!("unknown key '{key}'"))?);
        Self::press_chord(&keys)
    }

    fn record_hotkey(&self, timeout_ms: u32) -> Result<String, String> {
        Self::record(timeout_ms)
    }

    fn reset_recording(&self) {
        LISTENING.store(false, Ordering::SeqCst);
    }

    fn send_text(&self, text: &str) -> Result<(), String> {
        // One character at a time through the same name mapping. Anything
        // outside the mapped set is skipped rather than failing the whole
        // string, so a stray emoji does not lose the sentence around it.
        for ch in text.chars() {
            let name = ch.to_string();
            if let Some(mapped) = map_key_to_rdev(&name) {
                Self::press_chord(&[mapped])?;
            } else if ch == ' ' {
                Self::press_chord(&[rdev::Key::Space])?;
            } else {
                log::debug!("send_text: skipping unmappable character {ch:?}");
            }
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

/// Whether a key *name produced by [`rdev_key_to_string`]* is a modifier.
///
/// Deliberately case-sensitive, and only correct for that function's output:
/// it is used on the recording path, where every name comes from there. Do not
/// reach for it to classify user input — `map_key_to_rdev` handles that, and
/// it lowercases first.
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The name mapping is pure, so it can be tested without a display server.
    #[test]
    fn maps_key_names_case_insensitively() {
        assert_eq!(map_key_to_rdev("ctrl"), Some(rdev::Key::ControlLeft));
        assert_eq!(map_key_to_rdev("CTRL"), Some(rdev::Key::ControlLeft));
        assert_eq!(map_key_to_rdev("f5"), Some(rdev::Key::F5));
        assert_eq!(map_key_to_rdev("a"), Some(rdev::Key::KeyA));
        assert_eq!(map_key_to_rdev("A"), Some(rdev::Key::KeyA));
        assert_eq!(map_key_to_rdev("7"), Some(rdev::Key::Num7));
    }

    #[test]
    fn accepts_the_documented_aliases() {
        assert_eq!(map_key_to_rdev("esc"), map_key_to_rdev("escape"));
        assert_eq!(map_key_to_rdev("return"), map_key_to_rdev("enter"));
        assert_eq!(map_key_to_rdev("del"), map_key_to_rdev("delete"));
        for alias in ["win", "meta", "super"] {
            assert_eq!(map_key_to_rdev(alias), Some(rdev::Key::MetaLeft));
        }
    }

    /// Names arrive from `rdev_key_to_string`, so the fixtures use its
    /// capitalisation rather than what a user would type.
    #[test]
    fn builds_a_combo_with_modifiers_first() {
        let pressed = vec!["Ctrl".to_string(), "Shift".to_string(), "A".to_string()];
        assert_eq!(build_combo_from(&pressed, "A"), "ctrl+shift+a");
    }

    #[test]
    fn a_bare_key_needs_no_modifiers() {
        assert_eq!(build_combo_from(&["F5".to_string()], "F5"), "f5");
    }

    /// Releasing a modifier does not end a chord, so the combo is just the
    /// modifiers held so far.
    #[test]
    fn releasing_a_modifier_yields_only_the_modifiers() {
        let pressed = vec!["Ctrl".to_string(), "Shift".to_string()];
        assert_eq!(build_combo_from(&pressed, "Shift"), "ctrl+shift");
    }

    #[test]
    fn rejects_unknown_names() {
        assert_eq!(map_key_to_rdev("nonsense"), None);
        assert_eq!(map_key_to_rdev(""), None);
    }

    #[test]
    fn modifiers_are_recognised_as_such() {
        assert!(is_rdev_mod(&rdev::Key::ControlLeft));
        assert!(is_rdev_mod(&rdev::Key::ShiftRight));
        assert!(is_rdev_mod(&rdev::Key::MetaLeft));
        assert!(!is_rdev_mod(&rdev::Key::KeyA));
        assert!(!is_rdev_mod(&rdev::Key::F5));
    }
}
