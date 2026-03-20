use crate::engine::value::{escape_str, JsonValue};

/// 格式化选项。
#[derive(Debug, Clone)]
pub struct FormatOptions {
    /// 缩进空格数，默认 2。
    pub indent: usize,
    /// 是否在文件末尾添加换行符，默认 true。
    pub trailing_newline: bool,
    /// 是否按字母顺序排序对象的 key，默认 false（保留插入顺序）。
    pub sort_keys: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent: 2,
            trailing_newline: true,
            sort_keys: false,
        }
    }
}

/// 将 JSON 值格式化为美化输出字符串。
pub fn format_pretty(value: &JsonValue, opts: &FormatOptions) -> String {
    let mut out = String::new();
    write_pretty(value, opts, 0, &mut out);
    if opts.trailing_newline {
        out.push('\n');
    }
    out
}

/// 将 JSON 值格式化为紧凑（无空白）字符串，不带末尾换行。
pub fn format_compact(value: &JsonValue) -> String {
    let mut out = String::new();
    write_compact(value, &mut out);
    out
}

// ── 内部实现 ─────────────────────────────────────────────────────────────────

fn write_pretty(value: &JsonValue, opts: &FormatOptions, depth: usize, out: &mut String) {
    match value {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        JsonValue::Number(n) => out.push_str(&fmt_number(*n)),
        JsonValue::String(s) => {
            out.push('"');
            out.push_str(&escape_str(s));
            out.push('"');
        }
        JsonValue::Array(arr) => {
            if arr.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push_str("[\n");
            let inner_indent = make_indent(opts.indent, depth + 1);
            let close_indent = make_indent(opts.indent, depth);
            for (i, item) in arr.iter().enumerate() {
                out.push_str(&inner_indent);
                write_pretty(item, opts, depth + 1, out);
                if i + 1 < arr.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str(&close_indent);
            out.push(']');
        }
        JsonValue::Object(map) => {
            if map.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push_str("{\n");
            let inner_indent = make_indent(opts.indent, depth + 1);
            let close_indent = make_indent(opts.indent, depth);

            let pairs: Box<dyn Iterator<Item = (&String, &JsonValue)>> = if opts.sort_keys {
                let mut keys: Vec<&String> = map.keys().collect();
                keys.sort();
                Box::new(keys.into_iter().filter_map(|k| map.get(k).map(|v| (k, v))))
            } else {
                Box::new(map.iter())
            };

            let entries: Vec<_> = pairs.collect();
            let last = entries.len().saturating_sub(1);
            for (i, (k, v)) in entries.into_iter().enumerate() {
                out.push_str(&inner_indent);
                out.push('"');
                out.push_str(&escape_str(k));
                out.push_str("\": ");
                write_pretty(v, opts, depth + 1, out);
                if i < last {
                    out.push(',');
                }
                out.push('\n');
            }
            out.push_str(&close_indent);
            out.push('}');
        }
    }
}

fn write_compact(value: &JsonValue, out: &mut String) {
    match value {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        JsonValue::Number(n) => out.push_str(&fmt_number(*n)),
        JsonValue::String(s) => {
            out.push('"');
            out.push_str(&escape_str(s));
            out.push('"');
        }
        JsonValue::Array(arr) => {
            out.push('[');
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_compact(item, out);
            }
            out.push(']');
        }
        JsonValue::Object(map) => {
            out.push('{');
            for (i, (k, v)) in map.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('"');
                out.push_str(&escape_str(k));
                out.push_str("\":");
                write_compact(v, out);
            }
            out.push('}');
        }
    }
}

fn fmt_number(n: f64) -> String {
    if n.is_nan() || n.is_infinite() {
        "null".into()
    } else if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

fn make_indent(size: usize, depth: usize) -> String {
    " ".repeat(size * depth)
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;

    use super::*;

    fn simple_object() -> JsonValue {
        let mut m = IndexMap::new();
        m.insert("a".into(), JsonValue::Number(1.0));
        m.insert("b".into(), JsonValue::String("hello".into()));
        JsonValue::Object(m)
    }

    mod format_compact {
        use super::*;

        #[test]
        fn object_has_no_whitespace() {
            let v = simple_object();
            let s = format_compact(&v);
            assert_eq!(s, r#"{"a":1,"b":"hello"}"#);
        }

        #[test]
        fn empty_array_is_compact() {
            assert_eq!(format_compact(&JsonValue::Array(vec![])), "[]");
        }

        #[test]
        fn null_is_literal_null() {
            assert_eq!(format_compact(&JsonValue::Null), "null");
        }

        #[test]
        fn integer_number_has_no_decimal() {
            assert_eq!(format_compact(&JsonValue::Number(42.0)), "42");
        }
    }

    mod format_pretty {
        use super::*;

        #[test]
        fn object_has_correct_indentation() {
            let v = simple_object();
            let opts = FormatOptions::default();
            let s = format_pretty(&v, &opts);
            assert!(s.contains("  \"a\": 1"));
            assert!(s.contains("  \"b\": \"hello\""));
        }

        #[test]
        fn trailing_newline_present_by_default() {
            let v = simple_object();
            let s = format_pretty(&v, &FormatOptions::default());
            assert!(s.ends_with('\n'));
        }

        #[test]
        fn no_trailing_newline_when_disabled() {
            let v = simple_object();
            let opts = FormatOptions {
                trailing_newline: false,
                ..Default::default()
            };
            let s = format_pretty(&v, &opts);
            assert!(!s.ends_with('\n'));
        }

        #[test]
        fn sort_keys_outputs_alphabetically() {
            let mut m = IndexMap::new();
            m.insert("z".into(), JsonValue::Null);
            m.insert("a".into(), JsonValue::Null);
            let v = JsonValue::Object(m);
            let opts = FormatOptions {
                sort_keys: true,
                ..Default::default()
            };
            let s = format_pretty(&v, &opts);
            let a_pos = s.find("\"a\"").unwrap();
            let z_pos = s.find("\"z\"").unwrap();
            assert!(a_pos < z_pos, "a 应在 z 之前");
        }

        #[test]
        fn empty_object_is_compact_inline() {
            let v = JsonValue::Object(IndexMap::new());
            let s = format_pretty(&v, &FormatOptions::default());
            assert!(s.starts_with("{}"));
        }
    }
}
