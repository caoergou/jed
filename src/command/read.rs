use std::path::Path;

use crate::{
    command::{exit_code, load_lenient, read_file},
    engine::{
        exists, get, infer_schema, parse_lenient, FormatOptions, JsonValue, PathError,
        format_pretty,
    },
    i18n::{get_locale, t_to},
    output::Ctx,
};

/// `get <path>` — 输出路径处的值。
pub fn cmd_get(file: &Path, path: &str, ctx: &Ctx) -> Result<i32, Box<dyn std::error::Error>> {
    let locale = get_locale();
    let file_str = file.display().to_string();
    let (doc, _) = load_lenient(file)?;
    match get(&doc, path) {
        Ok(value) => {
            let actions = vec![
                format!("jed set {path} <value> {file_str}"),
                format!("jed del {path} {file_str}"),
                format!("jed keys {path} {file_str}"),
            ];
            ctx.print_value_with_actions(value, &actions);
            Ok(exit_code::OK)
        }
        Err(PathError::KeyNotFound { key }) => {
            let msg = t_to("err.key_not_found", &locale).replace("{0}", &key);
            let fix = format!("Run 'jed keys . {file_str}' to list available keys");
            let actions = vec![format!("jed keys . {file_str}")];
            ctx.print_error(&msg, Some(&fix), &actions);
            Ok(exit_code::NOT_FOUND)
        }
        Err(PathError::IndexOutOfBounds { index, len }) => {
            let msg = t_to("err.index_oob", &locale)
                .replace("{0}", &index.to_string())
                .replace("{1}", &len.to_string());
            let fix = format!("Run 'jed len {path} {file_str}' to check the array length");
            let actions = vec![format!("jed len {path} {file_str}")];
            ctx.print_error(&msg, Some(&fix), &actions);
            Ok(exit_code::NOT_FOUND)
        }
        Err(e) => {
            let msg = t_to("err.path", &locale).replace("{0}", &e.to_string());
            let fix = format!("Run 'jed type {path} {file_str}' to inspect the value type");
            let actions = vec![format!("jed type {path} {file_str}")];
            ctx.print_error(&msg, Some(&fix), &actions);
            Ok(exit_code::TYPE_MISMATCH)
        }
    }
}

/// `keys <path>` — 列出 key 或索引。
pub fn cmd_keys(file: &Path, path: &str, ctx: &Ctx) -> Result<i32, Box<dyn std::error::Error>> {
    let locale = get_locale();
    let file_str = file.display().to_string();
    let (doc, _) = load_lenient(file)?;
    let node = match get(&doc, path) {
        Ok(v) => v,
        Err(e) => {
            let msg = t_to("err.path", &locale).replace("{0}", &e.to_string());
            let fix = format!("Run 'jed keys . {file_str}' to list root-level keys");
            ctx.print_error(&msg, Some(&fix), &[format!("jed keys . {file_str}")]);
            return Ok(exit_code::NOT_FOUND);
        }
    };

    match node {
        JsonValue::Object(map) => {
            let keys: Vec<String> = map.keys().cloned().collect();
            let actions = keys
                .iter()
                .take(3)
                .map(|k| {
                    let key_path = if path == "." {
                        format!(".{k}")
                    } else {
                        format!("{path}.{k}")
                    };
                    format!("jed get {key_path} {file_str}")
                })
                .collect::<Vec<_>>();
            ctx.print_list_with_actions(&keys, &actions);
        }
        JsonValue::Array(arr) => {
            let indices: Vec<String> = (0..arr.len()).map(|i| i.to_string()).collect();
            let actions = vec![format!("jed get {path}[0] {file_str}")];
            ctx.print_list_with_actions(&indices, &actions);
        }
        other => {
            let msg = t_to("err.type_no_keys", &locale).replace("{0}", other.type_name());
            let fix = format!("Run 'jed type {path} {file_str}' to check the actual type");
            ctx.print_error(&msg, Some(&fix), &[format!("jed type {path} {file_str}")]);
            return Ok(exit_code::TYPE_MISMATCH);
        }
    }
    Ok(exit_code::OK)
}

/// `len <path>` — 数组长度或对象 key 数量。
pub fn cmd_len(file: &Path, path: &str, ctx: &Ctx) -> Result<i32, Box<dyn std::error::Error>> {
    let locale = get_locale();
    let file_str = file.display().to_string();
    let (doc, _) = load_lenient(file)?;
    let node = match get(&doc, path) {
        Ok(v) => v,
        Err(e) => {
            let msg = t_to("err.path", &locale).replace("{0}", &e.to_string());
            ctx.print_error(&msg, None, &[]);
            return Ok(exit_code::NOT_FOUND);
        }
    };

    if let Some(n) = node.len() {
        let actions = if n > 0 {
            vec![format!("jed get {path}[0] {file_str}")]
        } else {
            vec![]
        };
        ctx.print_raw_with_actions(serde_json::json!(n), &actions);
        Ok(exit_code::OK)
    } else {
        let msg = t_to("err.type_no_len", &locale).replace("{0}", node.type_name());
        let fix = format!("Run 'jed type {path} {file_str}' to check the actual type");
        ctx.print_error(&msg, Some(&fix), &[format!("jed type {path} {file_str}")]);
        Ok(exit_code::TYPE_MISMATCH)
    }
}

/// `type <path>` — 输出值的类型名称。
pub fn cmd_type(file: &Path, path: &str, ctx: &Ctx) -> Result<i32, Box<dyn std::error::Error>> {
    let locale = get_locale();
    let file_str = file.display().to_string();
    let (doc, _) = load_lenient(file)?;
    match get(&doc, path) {
        Ok(v) => {
            ctx.print_str(v.type_name());
            Ok(exit_code::OK)
        }
        Err(e) => {
            let msg = t_to("err.path", &locale).replace("{0}", &e.to_string());
            let fix = format!("Run 'jed keys . {file_str}' to list available paths");
            ctx.print_error(&msg, Some(&fix), &[format!("jed keys . {file_str}")]);
            Ok(exit_code::NOT_FOUND)
        }
    }
}

/// `exists <path>` — exit 0 表示存在，exit 2 表示不存在。
pub fn cmd_exists(file: &Path, path: &str, ctx: &Ctx) -> Result<i32, Box<dyn std::error::Error>> {
    let locale = get_locale();
    let file_str = file.display().to_string();
    let (doc, _) = load_lenient(file)?;
    if exists(&doc, path) {
        let actions = vec![format!("jed get {path} {file_str}")];
        ctx.print_raw_with_actions(serde_json::json!(true), &actions);
        Ok(exit_code::OK)
    } else {
        let msg = t_to("err.path_not_exists", &locale);
        let fix = format!("Run 'jed keys . {file_str}' to list available paths");
        ctx.print_error(&msg, Some(&fix), &[format!("jed keys . {file_str}")]);
        Ok(exit_code::NOT_FOUND)
    }
}

/// `schema` — 推断并输出文件结构。
pub fn cmd_schema(file: &Path, ctx: &Ctx) -> Result<i32, Box<dyn std::error::Error>> {
    let file_str = file.display().to_string();
    let (doc, _) = load_lenient(file)?;
    let actions = vec![
        format!("jed get <path> {file_str}"),
        format!("jed validate <schema_file> {file_str}"),
    ];
    if ctx.json {
        let schema = build_json_schema(&doc);
        ctx.print_raw_with_actions(schema, &actions);
    } else {
        println!("{}", infer_schema(&doc));
    }
    Ok(exit_code::OK)
}

fn build_json_schema(value: &JsonValue) -> serde_json::Value {
    match value {
        JsonValue::Null => serde_json::json!({"type": "null"}),
        JsonValue::Bool(_) => serde_json::json!({"type": "boolean"}),
        JsonValue::Number(_) => serde_json::json!({"type": "number"}),
        JsonValue::String(_) => serde_json::json!({"type": "string"}),
        JsonValue::Array(arr) => {
            let items = arr.first().map_or(serde_json::json!({}), build_json_schema);
            serde_json::json!({"type": "array", "items": items})
        }
        JsonValue::Object(map) => {
            let mut props = serde_json::Map::new();
            for (k, v) in map {
                props.insert(k.clone(), build_json_schema(v));
            }
            serde_json::json!({"type": "object", "properties": props})
        }
    }
}

/// `check` — 校验 JSON 格式。
pub fn cmd_check(file: &Path, ctx: &Ctx) -> Result<i32, Box<dyn std::error::Error>> {
    let file_str = file.display().to_string();
    let content = read_file(file)?;
    match parse_lenient(&content) {
        Ok(_) => {
            let actions = vec![format!("jed fmt {file_str}")];
            ctx.print_raw_with_actions(serde_json::json!({"valid": true}), &actions);
            Ok(exit_code::OK)
        }
        Err(e) => {
            let fix = format!("Run 'jed fix {file_str}' to auto-repair common JSON errors");
            let actions = vec![format!("jed fix {file_str}")];
            ctx.print_error(&format!("{e}"), Some(&fix), &actions);
            Ok(exit_code::ERROR)
        }
    }
}

/// `diff <other>` — 对比两个 JSON 文件的差异。
///
/// Exit codes: 0 = identical, 1 = has differences, non-zero on parse error.
pub fn cmd_diff(file: &Path, other: &Path, ctx: &Ctx) -> Result<i32, Box<dyn std::error::Error>> {
    let (a, _) = load_lenient(file)?;
    let (b, _) = load_lenient(other)?;

    let a_str = format_pretty(&a, &FormatOptions::default());
    let b_str = format_pretty(&b, &FormatOptions::default());

    if a_str == b_str {
        ctx.print_raw(serde_json::json!({"identical": true, "diff": []}));
        return Ok(exit_code::OK);
    }

    let a_lines: Vec<&str> = a_str.lines().collect();
    let b_lines: Vec<&str> = b_str.lines().collect();
    let max = a_lines.len().max(b_lines.len());

    let mut diff_lines: Vec<String> = Vec::new();
    for i in 0..max {
        match (a_lines.get(i), b_lines.get(i)) {
            (Some(al), Some(bl)) if al == bl => {}
            (Some(al), Some(bl)) => {
                diff_lines.push(format!("- {al}"));
                diff_lines.push(format!("+ {bl}"));
            }
            (Some(al), None) => diff_lines.push(format!("- {al}")),
            (None, Some(bl)) => diff_lines.push(format!("+ {bl}")),
            (None, None) => {}
        }
    }

    let file_a = file.display().to_string();
    let file_b = other.display().to_string();
    let actions = vec![
        format!("jed set <path> <value> {file_a}"),
        format!("jed set <path> <value> {file_b}"),
    ];

    if ctx.json {
        let lines: Vec<serde_json::Value> = diff_lines
            .iter()
            .map(|s| serde_json::Value::String(s.clone()))
            .collect();
        ctx.print_raw_with_actions(
            serde_json::json!({"identical": false, "diff": lines}),
            &actions,
        );
    } else {
        for line in &diff_lines {
            println!("{line}");
        }
    }

    // Exit 1 = has differences (not an error, just a result)
    Ok(1)
}
