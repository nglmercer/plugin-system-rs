//! The timer plugin, compiled as a WASI Preview 2 component.
//!
//! This is the pilot for the FFI → WASI migration (see
//! `docs/wasi-migration.md`). It is deliberately the plugin with no native
//! dependencies, so it exercises the ABI, the host imports, and the loader
//! without needing any host capability beyond logging and the clock.
//!
//! # How a timer runs without a thread
//!
//! A component is single-threaded and has no way to schedule a callback or to
//! push an event at the host: it only ever runs inside a call the host makes.
//! So nothing here counts down in the background, and nothing needs to. A
//! timer is stored as a *deadline* — an absolute instant on the WASI wall
//! clock — and every read derives the remaining time from the clock at the
//! moment it is asked. That is what makes the numbers real rather than the
//! stored copy of whatever `start` was passed.
//!
//! Expiry is reported two ways, because there are two kinds of caller:
//!
//!  * `interface-data` carries the live remaining time and an `expired` flag,
//!    which is what a polling widget renders.
//!  * `poll` returns the timers that have expired *since the previous poll*
//!    and will not report them again, which is what a host wants if it means
//!    to turn expiry into an event exactly once.

wit_bindgen::generate!({
    // Point at the host's copy rather than vendoring a second one: a contract
    // with two definitions is a contract that drifts.
    path: "../../crates/plugin-system/wit",
    world: "streamdeck-plugin",
});

use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use exports::streamdeck::plugin::guest::Guest;
use streamdeck::plugin::host::log as host_log;
use streamdeck::plugin::types::{CommandError, Dependency, LogLevel, Metadata};

/// One running timer.
#[derive(Clone, Copy)]
struct Timer {
    /// What `start` was asked for, kept so the UI can render progress.
    duration_secs: u64,
    /// Absolute wall-clock instant the timer fires, in milliseconds since the
    /// Unix epoch. Absolute rather than a countdown because there is no tick:
    /// the only thing that advances is the clock.
    deadline_ms: u64,
    /// Whether `poll` has already handed this expiry to the host. Expiry is an
    /// edge, and an edge reported twice is a bug in whatever acts on it.
    expiry_reported: bool,
}

impl Timer {
    fn remaining_ms(&self, now_ms: u64) -> u64 {
        self.deadline_ms.saturating_sub(now_ms)
    }

    /// Remaining whole seconds, rounded up.
    ///
    /// Rounding up means a one-second timer reads "1" for its whole first
    /// second and only reaches "0" when it has actually fired — rounding down
    /// would show a zero that is still counting.
    fn remaining_secs(&self, now_ms: u64) -> u64 {
        self.remaining_ms(now_ms).div_ceil(1000)
    }

    fn is_expired(&self, now_ms: u64) -> bool {
        now_ms >= self.deadline_ms
    }
}

// Each plugin gets its own `Store`, so a component instance is never shared
// between threads and thread-local state is exactly right here.
thread_local! {
    static TIMERS: RefCell<HashMap<String, Timer>> = RefCell::new(HashMap::new());
}

/// The WASI wall clock, in milliseconds since the Unix epoch.
///
/// The host provides `wasi:clocks/wall-clock`; there is no other source of
/// time inside the sandbox, which is the point.
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn require_name(args: &serde_json::Value) -> Result<String, CommandError> {
    args.get("name")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .ok_or_else(|| CommandError::InvalidArgs("expected a string `name`".into()))
}

struct TimerPlugin;

impl TimerPlugin {
    fn start(args: &serde_json::Value) -> Result<serde_json::Value, CommandError> {
        let name = require_name(args)?;
        let seconds = args
            .get("seconds")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| CommandError::InvalidArgs("expected an integer `seconds`".into()))?;

        if seconds == 0 {
            return Err(CommandError::InvalidArgs(
                "`seconds` must be greater than zero".into(),
            ));
        }
        // A duration long enough to overflow the deadline is a caller mistake,
        // and silently wrapping it would produce a timer that fires instantly.
        let deadline_ms = now_ms()
            .checked_add(seconds.saturating_mul(1000))
            .ok_or_else(|| CommandError::InvalidArgs("`seconds` is implausibly large".into()))?;

        let timer = Timer {
            duration_secs: seconds,
            deadline_ms,
            expiry_reported: false,
        };
        TIMERS.with(|t| t.borrow_mut().insert(name.clone(), timer));
        host_log(
            LogLevel::Info,
            &format!("started timer '{name}' for {seconds}s"),
        );

        Ok(Self::describe(&name, &timer, now_ms()))
    }

    fn get(args: &serde_json::Value) -> Result<serde_json::Value, CommandError> {
        let name = require_name(args)?;
        let timer = TIMERS.with(|t| t.borrow().get(&name).copied());
        match timer {
            Some(timer) => Ok(Self::describe(&name, &timer, now_ms())),
            None => Err(CommandError::NotFound(format!("no timer named '{name}'"))),
        }
    }

    fn stop(args: &serde_json::Value) -> Result<serde_json::Value, CommandError> {
        let name = require_name(args)?;
        let existed = TIMERS.with(|t| t.borrow_mut().remove(&name).is_some());
        Ok(serde_json::json!({ "ok": true, "removed": existed }))
    }

    fn list() -> serde_json::Value {
        let mut names = TIMERS.with(|t| t.borrow().keys().cloned().collect::<Vec<_>>());
        // Deterministic ordering keeps the widget from reshuffling on refresh.
        names.sort();
        serde_json::json!({ "ok": true, "timers": names })
    }

    /// Report timers that fired since the last poll, exactly once each.
    ///
    /// The host has no way to be woken by a guest, so "the timer fired" can
    /// only be discovered by asking. Marking each expiry as reported is what
    /// makes repeated asking safe.
    fn poll() -> serde_json::Value {
        let now = now_ms();
        let mut fired = TIMERS.with(|t| {
            let mut timers = t.borrow_mut();
            let mut fired = Vec::new();
            for (name, timer) in timers.iter_mut() {
                if timer.is_expired(now) && !timer.expiry_reported {
                    timer.expiry_reported = true;
                    fired.push(name.clone());
                }
            }
            fired
        });
        fired.sort();

        for name in &fired {
            host_log(LogLevel::Info, &format!("timer '{name}' elapsed"));
        }

        serde_json::json!({ "ok": true, "expired": fired })
    }

    /// Discard every timer that has already fired and been reported.
    fn clear_expired() -> serde_json::Value {
        let now = now_ms();
        let removed = TIMERS.with(|t| {
            let mut timers = t.borrow_mut();
            let before = timers.len();
            timers.retain(|_, timer| !timer.is_expired(now));
            before - timers.len()
        });
        serde_json::json!({ "ok": true, "removed": removed })
    }

    fn describe(name: &str, timer: &Timer, now_ms: u64) -> serde_json::Value {
        serde_json::json!({
            "ok": true,
            "name": name,
            "seconds": timer.duration_secs,
            "remaining": timer.remaining_secs(now_ms),
            "remaining_ms": timer.remaining_ms(now_ms),
            "expired": timer.is_expired(now_ms),
        })
    }

    fn snapshot() -> serde_json::Value {
        let now = now_ms();
        let mut timers: Vec<_> = TIMERS.with(|t| {
            t.borrow()
                .iter()
                .map(|(name, timer)| Self::describe(name, timer, now))
                .collect()
        });
        timers.sort_by_key(|v| v["name"].as_str().unwrap_or_default().to_string());
        serde_json::json!({ "timers": timers })
    }
}

impl Guest for TimerPlugin {
    fn get_metadata() -> Metadata {
        Metadata {
            name: "timer".into(),
            version: "0.2.0".into(),
            authors: vec!["StreamDeck Core".into()],
            dependencies: Vec::<Dependency>::new(),
        }
    }

    fn on_load() {
        host_log(LogLevel::Info, "TimerPlugin loaded (wasm)");
    }

    fn on_unload() {
        host_log(LogLevel::Info, "TimerPlugin unloading (wasm)");
    }

    fn handle_command(method: String, args_json: String) -> Result<String, CommandError> {
        let args: serde_json::Value = serde_json::from_str(&args_json)
            .map_err(|e| CommandError::InvalidArgs(format!("args are not valid JSON: {e}")))?;

        let value = match method.as_str() {
            "start" => Self::start(&args)?,
            "get" => Self::get(&args)?,
            "stop" => Self::stop(&args)?,
            "list" => Self::list(),
            "poll" => Self::poll(),
            "clear_expired" => Self::clear_expired(),
            other => {
                return Err(CommandError::NotFound(format!(
                    "timer has no method '{other}'"
                )))
            }
        };

        serde_json::to_string(&value)
            .map_err(|e| CommandError::Failed(format!("failed to encode result: {e}")))
    }

    fn interface_ids() -> Vec<String> {
        vec!["Timer".into()]
    }

    fn interface_data() -> Option<String> {
        serde_json::to_string(&Self::snapshot()).ok()
    }
}

export!(TimerPlugin);

#[cfg(test)]
mod tests {
    use super::*;

    fn timer_at(now: u64, duration_secs: u64) -> Timer {
        Timer {
            duration_secs,
            deadline_ms: now + duration_secs * 1000,
            expiry_reported: false,
        }
    }

    /// The regression that mattered: a timer must derive its remaining time
    /// from the clock, not report back the duration it was handed.
    #[test]
    fn remaining_time_shrinks_as_the_clock_advances() {
        let timer = timer_at(10_000, 30);
        assert_eq!(timer.remaining_secs(10_000), 30);
        assert_eq!(timer.remaining_secs(20_000), 20);
        assert_eq!(timer.remaining_secs(39_000), 1);
    }

    #[test]
    fn a_timer_expires_at_its_deadline_and_stays_expired() {
        let timer = timer_at(0, 5);
        assert!(!timer.is_expired(4_999));
        assert!(timer.is_expired(5_000));
        assert!(timer.is_expired(500_000));
        assert_eq!(timer.remaining_secs(500_000), 0);
    }

    /// Rounding up: a timer with 1ms left reads "1 second", never "0 seconds
    /// but still running".
    #[test]
    fn remaining_seconds_round_up() {
        let timer = timer_at(0, 2);
        assert_eq!(timer.remaining_secs(1_999), 1);
        assert_eq!(timer.remaining_secs(1_001), 1);
        assert_eq!(timer.remaining_secs(2_000), 0);
    }
}
