//! A deliberately badly-behaved plugin, used only by the host test suite.
//!
//! Under `dlopen` every one of these behaviours takes down the whole process.
//! As a component they must each be contained, and the tests in
//! `crates/plugin-system/tests/wasm_tests.rs` assert exactly that.
//!
//! This crate is a test fixture and is never shipped.

wit_bindgen::generate!({
    path: "../../crates/plugin-system/wit",
    world: "streamdeck-plugin",
});

use exports::streamdeck::plugin::guest::Guest;
use streamdeck::plugin::types::{CommandError, Dependency, Metadata};

struct Misbehaving;

impl Guest for Misbehaving {
    fn get_metadata() -> Metadata {
        Metadata {
            name: "misbehaving".into(),
            version: "0.0.1".into(),
            authors: vec!["test fixture".into()],
            dependencies: Vec::<Dependency>::new(),
        }
    }

    fn on_load() {}
    fn on_unload() {}

    fn handle_command(method: String, _args_json: String) -> Result<String, CommandError> {
        match method.as_str() {
            // Never returns. Must be cut short by the epoch deadline.
            "hang" => {
                #[allow(clippy::empty_loop)]
                loop {
                    std::hint::spin_loop();
                }
            }
            // Panics unwind to a wasm trap at the component boundary.
            "panic" => panic!("this plugin panicked on purpose"),
            // Tries to blow past the store's memory ceiling.
            "hog" => {
                let mut chunks: Vec<Vec<u8>> = Vec::new();
                loop {
                    chunks.push(vec![0u8; 8 * 1024 * 1024]);
                    std::hint::black_box(&chunks);
                }
            }
            // A well-behaved command, to prove the plugin works at all.
            "ping" => Ok("{\"ok\":true}".to_string()),
            other => Err(CommandError::NotFound(other.to_string())),
        }
    }

    fn interface_ids() -> Vec<String> {
        vec!["Misbehaving".into()]
    }

    fn interface_data() -> Option<String> {
        None
    }
}

export!(Misbehaving);
