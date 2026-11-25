//! Manual desktop acceptance helper. Never opens a microphone or uses ASR.
//! stdin accepts fixed test cases; never accepts arbitrary text for insertion.
#[allow(dead_code)]
#[path = "../src/native.rs"]
mod native;

use std::io::{self, BufRead, Write};

fn main() {
    let mut target = None;
    for line in io::stdin().lock().lines() {
        let command = line.expect("stdin unavailable");
        let result = match command.trim() {
            "capture" => {
                target = None;
                native::focused_target().map(|value| target = Some(value))
            }
            "insert" | "unicode" | "long" | "boundary" | "cancel_after_first" => {
                let text = match command.trim() {
                    "unicode" => "नमस्ते café 😀.".to_string(),
                    "long" => "SONIQ acceptance 123. ".repeat(30),
                    "boundary" => "SONIQ acceptance 123. ".repeat(5) + "End",
                    _ => "SONIQ acceptance 123.".to_string(),
                };
                let checks = std::cell::Cell::new(0);
                target
                    .as_ref()
                    .ok_or_else(|| "No verified target".to_string())
                    .and_then(|value| {
                        native::insert(value, &text, || {
                            checks.set(checks.get() + 1);
                            command.trim() != "cancel_after_first" || checks.get() <= 4
                        })
                    })
            }
            "quit" => break,
            _ => Err("Expected capture, insert or quit".into()),
        };
        println!(
            "{}",
            match result {
                Ok(()) => serde_json::json!({"ok": true}),
                Err(error) => serde_json::json!({"ok": false, "error": error}),
            }
        );
        io::stdout().flush().expect("stdout unavailable");
    }
}
