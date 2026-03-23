use serde_json::{Value, json};

use crate::engine::{format_compact, JsonValue};

/// Per-invocation context: which command is running and what output mode.
pub struct Ctx {
    pub cmd: &'static str,
    pub json: bool,
}

impl Ctx {
    pub fn new(cmd: &'static str, json: bool) -> Self {
        Self { cmd, json }
    }

    // ── value output ─────────────────────────────────────────────────────────

    pub fn print_value_with_actions(&self, v: &JsonValue, actions: &[String]) {
        if self.json {
            println!("{}", build_ok(self.cmd, je_to_serde(v), actions));
        } else {
            println!("{}", format_compact(v));
        }
    }

    pub fn print_str(&self, s: &str) {
        if self.json {
            println!("{}", build_ok(self.cmd, json!(s), &[]));
        } else {
            println!("{s}");
        }
    }

    pub fn print_list_with_actions(&self, items: &[String], actions: &[String]) {
        if self.json {
            let arr: Value = items.iter().map(|s| Value::String(s.clone())).collect();
            println!("{}", build_ok(self.cmd, arr, actions));
        } else {
            for item in items {
                println!("{item}");
            }
        }
    }

    /// Print a structured `serde_json::Value` as the result.
    pub fn print_raw(&self, v: Value) {
        self.print_raw_with_actions(v, &[]);
    }

    pub fn print_raw_with_actions(&self, v: Value, actions: &[String]) {
        if self.json {
            println!("{}", build_ok(self.cmd, v, actions));
        } else {
            match &v {
                Value::String(s) => println!("{s}"),
                Value::Array(arr) => {
                    for item in arr {
                        if let Value::String(s) = item {
                            println!("{s}");
                        } else {
                            println!("{item}");
                        }
                    }
                }
                other => println!("{other}"),
            }
        }
    }

    // ── success / ok ─────────────────────────────────────────────────────────

    pub fn print_ok_with_actions(&self, msg: &str, actions: &[String]) {
        if self.json {
            println!("{}", build_ok(self.cmd, json!(msg), actions));
        } else {
            println!("{msg}");
        }
    }

    // ── error ─────────────────────────────────────────────────────────────────

    pub fn print_error(&self, msg: &str, fix: Option<&str>, actions: &[String]) {
        if self.json {
            println!("{}", build_err(self.cmd, msg, fix, actions));
        } else {
            eprintln!("{msg}");
        }
    }
}

// ── private helpers ───────────────────────────────────────────────────────────

fn build_ok(cmd: &str, result: Value, actions: &[String]) -> Value {
    let mut m = serde_json::Map::new();
    m.insert("ok".into(), true.into());
    m.insert("command".into(), cmd.into());
    m.insert("result".into(), result);
    if !actions.is_empty() {
        m.insert(
            "next_actions".into(),
            actions
                .iter()
                .map(|s| Value::String(s.clone()))
                .collect::<Vec<_>>()
                .into(),
        );
    }
    Value::Object(m)
}

fn build_err(cmd: &str, error: &str, fix: Option<&str>, actions: &[String]) -> Value {
    let mut m = serde_json::Map::new();
    m.insert("ok".into(), false.into());
    m.insert("command".into(), cmd.into());
    m.insert("error".into(), error.into());
    if let Some(f) = fix {
        m.insert("fix".into(), f.into());
    }
    if !actions.is_empty() {
        m.insert(
            "next_actions".into(),
            actions
                .iter()
                .map(|s| Value::String(s.clone()))
                .collect::<Vec<_>>()
                .into(),
        );
    }
    Value::Object(m)
}

/// Convert our engine `JsonValue` to `serde_json::Value` via serialization.
fn je_to_serde(v: &JsonValue) -> Value {
    let s = format_compact(v);
    serde_json::from_str(&s).unwrap_or(Value::Null)
}
