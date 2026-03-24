pub mod read;
pub mod repair;
pub mod write;

use std::{fs, path::Path, process};

use crate::{
    cli::Command,
    engine::{JsonValue, get, parse_lenient},
    i18n::{get_locale, t_to},
    output::Ctx,
};

/// 命令模式的统一退出码。
pub mod exit_code {
    pub const OK: i32 = 0;
    pub const ERROR: i32 = 1;
    pub const NOT_FOUND: i32 = 2;
    pub const TYPE_MISMATCH: i32 = 3;
}

/// 执行命令并以适当的退出码退出。
pub fn run(file: &Path, cmd: Command, json: bool) {
    let cmd_name = cmd_static_name(&cmd);
    let ctx = Ctx::new(cmd_name, json);
    let result = dispatch(file, cmd, &ctx);
    match result {
        Ok(code) => process::exit(code),
        Err(e) => {
            ctx.print_error(&e.to_string(), None, &[]);
            process::exit(exit_code::ERROR);
        }
    }
}

fn dispatch(file: &Path, cmd: Command, ctx: &Ctx) -> Result<i32, Box<dyn std::error::Error>> {
    match cmd {
        Command::Get { path, .. } => read::cmd_get(file, &path, ctx),
        Command::Keys { path, .. } => read::cmd_keys(file, &path, ctx),
        Command::Len { path, .. } => read::cmd_len(file, &path, ctx),
        Command::Type { path, .. } => read::cmd_type(file, &path, ctx),
        Command::Exists { path, .. } => read::cmd_exists(file, &path, ctx),
        Command::Schema { .. } => read::cmd_schema(file, ctx),
        Command::Check { .. } => read::cmd_check(file, ctx),
        Command::Set { path, value, .. } => write::cmd_set(file, &path, &value, ctx),
        Command::Del { path, .. } => write::cmd_del(file, &path, ctx),
        Command::Add { path, value, .. } => write::cmd_add(file, &path, &value, ctx),
        Command::Patch { operations, .. } => write::cmd_patch(file, &operations, ctx),
        Command::Mv { src, dst, .. } => write::cmd_mv(file, &src, &dst, ctx),
        Command::Fmt { indent, .. } => repair::cmd_fmt(file, indent, ctx),
        Command::Fix {
            dry_run,
            strip_comments,
            ..
        } => repair::cmd_fix(file, dry_run, strip_comments, ctx),
        Command::Minify { .. } => repair::cmd_minify(file, ctx),
        Command::Diff { other, .. } => read::cmd_diff(file, &other, ctx),
        // These are handled in main.rs before reaching dispatch
        Command::Completions { .. }
        | Command::Commands
        | Command::Explain { .. }
        | Command::Tree { .. }
        | Command::Query { .. }
        | Command::Validate { .. }
        | Command::Convert { .. } => Ok(exit_code::OK),
    }
}

/// Map a `Command` variant to a static name string.
fn cmd_static_name(cmd: &Command) -> &'static str {
    match cmd {
        Command::Get { .. } => "get",
        Command::Keys { .. } => "keys",
        Command::Len { .. } => "len",
        Command::Type { .. } => "type",
        Command::Exists { .. } => "exists",
        Command::Schema { .. } => "schema",
        Command::Check { .. } => "check",
        Command::Set { .. } => "set",
        Command::Del { .. } => "del",
        Command::Add { .. } => "add",
        Command::Patch { .. } => "patch",
        Command::Mv { .. } => "mv",
        Command::Fmt { .. } => "fmt",
        Command::Fix { .. } => "fix",
        Command::Minify { .. } => "minify",
        Command::Diff { .. } => "diff",
        Command::Tree { .. } => "tree",
        Command::Query { .. } => "query",
        Command::Validate { .. } => "validate",
        Command::Convert { .. } => "convert",
        Command::Commands => "commands",
        Command::Explain { .. } => "explain",
        Command::Completions { .. } => "completions",
    }
}

// ── 独立运行的命令（在 main.rs 中调用） ───────────────────────────────────────

/// 运行 tree 命令
pub fn run_tree(file: &Path, expand_all: bool, path: Option<&str>, json: bool) {
    let ctx = Ctx::new("tree", json);
    let locale = get_locale();
    let file_str = file.display().to_string();
    let (doc, _) = match load_lenient(file) {
        Ok(v) => v,
        Err(e) => {
            let msg = t_to("err.parse_failed", &locale)
                .replace("{0}", &file_str)
                .replace("{1}", &e.to_string());
            let fix = format!("Run 'jed fix {file_str}' to auto-repair JSON errors");
            ctx.print_error(&msg, Some(&fix), &[format!("jed fix {file_str}")]);
            process::exit(exit_code::ERROR);
        }
    };

    let display_doc = if let Some(p) = path {
        match get(&doc, p) {
            Ok(v) => v.clone(),
            Err(e) => {
                let fix = format!("Run 'jed keys . {file_str}' to list available paths");
                ctx.print_error(
                    &format!("Path error: {e}"),
                    Some(&fix),
                    &[format!("jed keys . {file_str}")],
                );
                process::exit(exit_code::NOT_FOUND);
            }
        }
    } else {
        doc
    };

    let tree_lines = render_tree(&display_doc, 0, expand_all);
    let actions = vec![format!("jed get <path> {file_str}")];
    ctx.print_raw_with_actions(
        serde_json::Value::Array(
            tree_lines
                .iter()
                .map(|s| serde_json::Value::String(s.clone()))
                .collect(),
        ),
        &actions,
    );
}

fn render_tree(value: &JsonValue, depth: usize, expand_all: bool) -> Vec<String> {
    let indent = "  ".repeat(depth);
    let mut lines = Vec::new();
    match value {
        JsonValue::Object(map) => {
            lines.push(format!("{indent}{{"));
            for (key, val) in map {
                if expand_all || !matches!(val, JsonValue::Object(_) | JsonValue::Array(_)) {
                    for sub in render_tree(val, depth + 1, expand_all) {
                        lines.push(format!("{indent}  {key}: {sub}"));
                    }
                } else {
                    lines.push(format!("{indent}  {key}: ..."));
                }
            }
            lines.push(format!("{indent}}}"));
        }
        JsonValue::Array(arr) => {
            lines.push(format!("{indent}["));
            for val in arr {
                for sub in render_tree(val, depth + 1, expand_all) {
                    lines.push(sub);
                }
            }
            lines.push(format!("{indent}]"));
        }
        other => {
            lines.push(format!("{indent}{other}"));
        }
    }
    lines
}

/// 运行 query 命令（路径过滤，与 get 等价）
pub fn run_query(file: &Path, filter: &str, json: bool) {
    let ctx = Ctx::new("query", json);
    let locale = get_locale();
    let file_str = file.display().to_string();
    let (doc, _) = match load_lenient(file) {
        Ok(v) => v,
        Err(e) => {
            let msg = t_to("err.parse_failed", &locale)
                .replace("{0}", &file_str)
                .replace("{1}", &e.to_string());
            ctx.print_error(&msg, None, &[]);
            process::exit(exit_code::ERROR);
        }
    };

    match get(&doc, filter) {
        Ok(value) => {
            let actions = vec![format!("jed set {filter} <value> {file_str}")];
            ctx.print_value_with_actions(value, &actions);
        }
        Err(e) => {
            let fix = format!("Run 'jed keys . {file_str}' to list available paths");
            ctx.print_error(
                &format!("Query error: {e}"),
                Some(&fix),
                &[format!("jed keys . {file_str}")],
            );
            process::exit(exit_code::NOT_FOUND);
        }
    }
}

/// 运行 validate 命令（JSON Schema 验证）
pub fn run_validate(file: &Path, schema_file: &Path, json: bool) {
    let ctx = Ctx::new("validate", json);
    let locale = get_locale();
    let file_str = file.display().to_string();

    let schema_content = match fs::read_to_string(schema_file) {
        Ok(c) => c,
        Err(e) => {
            let msg = t_to("err.read_failed", &locale)
                .replace("{0}", &schema_file.display().to_string())
                .replace("{1}", &e.to_string());
            ctx.print_error(&msg, None, &[]);
            process::exit(exit_code::ERROR);
        }
    };

    let (doc, _) = match load_lenient(file) {
        Ok(v) => v,
        Err(e) => {
            let msg = t_to("err.parse_failed", &locale)
                .replace("{0}", &file_str)
                .replace("{1}", &e.to_string());
            let fix = format!("Run 'jed fix {file_str}' to auto-repair JSON errors");
            ctx.print_error(&msg, Some(&fix), &[format!("jed fix {file_str}")]);
            process::exit(exit_code::ERROR);
        }
    };

    let schema_val = parse_json(&schema_content);
    let mut errors: Vec<ValidationError> = Vec::new();
    validate_against_schema(&doc, &schema_val, ".", &mut errors);

    if errors.is_empty() {
        ctx.print_raw_with_actions(
            serde_json::json!({"valid": true}),
            &[format!("jed check {file_str}")],
        );
    } else {
        let error_list: Vec<serde_json::Value> = errors
            .iter()
            .map(|e| serde_json::json!({"path": e.path, "message": e.message}))
            .collect();
        let fix = "Fix the validation errors in the JSON file";
        ctx.print_error(
            &format!("{} validation error(s)", errors.len()),
            Some(fix),
            &[format!("jed set <path> <value> {file_str}")],
        );
        if json {
            // --json 模式下通过 print_raw 输出结构化错误
            ctx.print_raw(serde_json::json!({
                "valid": false,
                "errors": error_list,
            }));
        } else {
            for e in &errors {
                eprintln!("  {}: {}", e.path, e.message);
            }
        }
        process::exit(exit_code::ERROR);
    }
}

// ── JSON Schema 验证 ──────────────────────────────────────────────────────────

struct ValidationError {
    path: String,
    message: String,
}

/// 递归地根据 JSON Schema 验证 `value`，收集所有验证错误。
///
/// 支持的关键字：`type`、`required`、`properties`、`minimum`、`maximum`、
/// `minLength`、`maxLength`、`minItems`、`maxItems`、`items`、`enum`。
fn validate_against_schema(
    value: &JsonValue,
    schema: &JsonValue,
    path: &str,
    errors: &mut Vec<ValidationError>,
) {
    let Some(schema_obj) = schema.as_object() else {
        return;
    };

    // type
    if let Some(JsonValue::String(type_str)) = schema_obj.get("type") {
        let matches = match type_str.as_str() {
            "integer" => matches!(value, JsonValue::Number(n) if n.fract() == 0.0),
            t => value.type_name() == t,
        };
        if !matches {
            errors.push(ValidationError {
                path: path.to_string(),
                message: format!("expected type '{}', got '{}'", type_str, value.type_name()),
            });
            return; // 类型不匹配时停止深入验证
        }
    }

    // required
    if let (Some(JsonValue::Array(reqs)), Some(obj)) =
        (schema_obj.get("required"), value.as_object())
    {
        for req in reqs {
            if let JsonValue::String(key) = req
                && !obj.contains_key(key.as_str())
            {
                errors.push(ValidationError {
                    path: path.to_string(),
                    message: format!("missing required field '{key}'"),
                });
            }
        }
    }

    // properties
    if let (Some(JsonValue::Object(props)), Some(obj)) =
        (schema_obj.get("properties"), value.as_object())
    {
        for (key, prop_schema) in props {
            if let Some(child_val) = obj.get(key.as_str()) {
                let child_path = schema_child_path(path, key);
                validate_against_schema(child_val, prop_schema, &child_path, errors);
            }
        }
    }

    // 数字、字符串、数组的类型特定验证委托给独立函数
    if let JsonValue::Number(n) = value {
        validate_number(*n, path, schema_obj, errors);
    }
    if let JsonValue::String(s) = value {
        validate_string(s, path, schema_obj, errors);
    }
    if let JsonValue::Array(arr) = value {
        validate_array(arr, path, schema_obj, errors);
    }

    // enum（枚举值）
    if let Some(JsonValue::Array(enum_vals)) = schema_obj.get("enum")
        && !enum_vals.contains(value)
    {
        let options: Vec<String> = enum_vals.iter().map(ToString::to_string).collect();
        errors.push(ValidationError {
            path: path.to_string(),
            message: format!("value not in enum: [{}]", options.join(", ")),
        });
    }
}

fn validate_number(
    n: f64,
    path: &str,
    schema_obj: &indexmap::IndexMap<String, JsonValue>,
    errors: &mut Vec<ValidationError>,
) {
    if let Some(JsonValue::Number(min)) = schema_obj.get("minimum")
        && n < *min
    {
        errors.push(ValidationError {
            path: path.to_string(),
            message: format!("value {n} is less than minimum {min}"),
        });
    }
    if let Some(JsonValue::Number(max)) = schema_obj.get("maximum")
        && n > *max
    {
        errors.push(ValidationError {
            path: path.to_string(),
            message: format!("value {n} is greater than maximum {max}"),
        });
    }
    if let Some(JsonValue::Number(excl_min)) = schema_obj.get("exclusiveMinimum")
        && n <= *excl_min
    {
        errors.push(ValidationError {
            path: path.to_string(),
            message: format!("value {n} must be greater than {excl_min}"),
        });
    }
    if let Some(JsonValue::Number(excl_max)) = schema_obj.get("exclusiveMaximum")
        && n >= *excl_max
    {
        errors.push(ValidationError {
            path: path.to_string(),
            message: format!("value {n} must be less than {excl_max}"),
        });
    }
}

fn validate_string(
    s: &str,
    path: &str,
    schema_obj: &indexmap::IndexMap<String, JsonValue>,
    errors: &mut Vec<ValidationError>,
) {
    let char_len = s.chars().count();
    if let Some(JsonValue::Number(min)) = schema_obj.get("minLength") {
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        if char_len < min.max(0.0) as usize {
            errors.push(ValidationError {
                path: path.to_string(),
                message: format!("string length {char_len} is less than minLength {min}"),
            });
        }
    }
    if let Some(JsonValue::Number(max)) = schema_obj.get("maxLength") {
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        if char_len > max.max(0.0) as usize {
            errors.push(ValidationError {
                path: path.to_string(),
                message: format!("string length {char_len} is greater than maxLength {max}"),
            });
        }
    }
}

fn validate_array(
    arr: &[JsonValue],
    path: &str,
    schema_obj: &indexmap::IndexMap<String, JsonValue>,
    errors: &mut Vec<ValidationError>,
) {
    if let Some(JsonValue::Number(min)) = schema_obj.get("minItems") {
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        if arr.len() < min.max(0.0) as usize {
            errors.push(ValidationError {
                path: path.to_string(),
                message: format!("array has {} item(s), minimum is {min}", arr.len()),
            });
        }
    }
    if let Some(JsonValue::Number(max)) = schema_obj.get("maxItems") {
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        if arr.len() > max.max(0.0) as usize {
            errors.push(ValidationError {
                path: path.to_string(),
                message: format!("array has {} item(s), maximum is {max}", arr.len()),
            });
        }
    }
    if let Some(items_schema) = schema_obj.get("items") {
        for (i, item) in arr.iter().enumerate() {
            let child_path = format!("{path}[{i}]");
            validate_against_schema(item, items_schema, &child_path, errors);
        }
    }
}

fn schema_child_path(parent: &str, key: &str) -> String {
    if parent == "." {
        format!(".{key}")
    } else {
        format!("{parent}.{key}")
    }
}

/// 运行 convert 命令（格式转换）
pub fn run_convert(file: &Path, format: &str, json: bool) {
    let ctx = Ctx::new("convert", json);
    let locale = get_locale();
    let file_str = file.display().to_string();
    let (doc, _) = match load_lenient(file) {
        Ok(v) => v,
        Err(e) => {
            let msg = t_to("err.parse_failed", &locale)
                .replace("{0}", &file_str)
                .replace("{1}", &e.to_string());
            ctx.print_error(&msg, None, &[]);
            process::exit(exit_code::ERROR);
        }
    };

    match format.to_lowercase().as_str() {
        "yaml" => {
            let yaml = to_yaml(&doc, 0);
            if ctx.json {
                ctx.print_raw(serde_json::json!({"format": "yaml", "content": yaml}));
            } else {
                print!("{yaml}");
            }
        }
        "toml" => match to_toml(&doc) {
            Ok(toml_str) => {
                if ctx.json {
                    ctx.print_raw(serde_json::json!({"format": "toml", "content": toml_str}));
                } else {
                    print!("{toml_str}");
                }
            }
            Err(e) => {
                ctx.print_error(
                    &format!("TOML conversion failed: {e}"),
                    Some("Remove null values or use 'jed convert yaml' instead"),
                    &[format!("jed convert yaml {file_str}")],
                );
                process::exit(exit_code::ERROR);
            }
        },
        other => {
            ctx.print_error(
                &format!("Unknown format: '{other}'. Supported: yaml, toml"),
                Some("Use 'yaml' or 'toml'"),
                &[],
            );
            process::exit(exit_code::ERROR);
        }
    }
}

fn to_toml(value: &JsonValue) -> Result<String, String> {
    let toml_val = json_to_toml_value(value)?;
    toml::to_string_pretty(&toml_val).map_err(|e| e.to_string())
}

fn json_to_toml_value(value: &JsonValue) -> Result<toml::Value, String> {
    match value {
        JsonValue::Null => Ok(toml::Value::String("null".to_string())),
        JsonValue::Bool(b) => Ok(toml::Value::Boolean(*b)),
        JsonValue::Number(n) => {
            if n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0 {
                #[allow(clippy::cast_possible_truncation)]
                Ok(toml::Value::Integer(*n as i64))
            } else {
                Ok(toml::Value::Float(*n))
            }
        }
        JsonValue::String(s) => Ok(toml::Value::String(s.clone())),
        JsonValue::Array(arr) => {
            let items: Result<Vec<_>, _> = arr.iter().map(json_to_toml_value).collect();
            Ok(toml::Value::Array(items?))
        }
        JsonValue::Object(map) => {
            let mut table = toml::map::Map::new();
            for (k, v) in map {
                table.insert(k.clone(), json_to_toml_value(v)?);
            }
            Ok(toml::Value::Table(table))
        }
    }
}

fn to_yaml(value: &JsonValue, depth: usize) -> String {
    let indent = "  ".repeat(depth);
    match value {
        JsonValue::Object(map) => map
            .iter()
            .map(|(key, val)| match val {
                JsonValue::Object(_) | JsonValue::Array(_) => {
                    format!("{indent}{key}:\n{}", to_yaml(val, depth + 1))
                }
                _ => format!("{indent}{key}: {}\n", yaml_scalar(val)),
            })
            .collect(),
        JsonValue::Array(arr) => arr
            .iter()
            .map(|val| match val {
                JsonValue::Object(_) | JsonValue::Array(_) => {
                    format!("{indent}-\n{}", to_yaml(val, depth + 1))
                }
                _ => format!("{indent}- {}\n", yaml_scalar(val)),
            })
            .collect(),
        other => format!("{}\n", yaml_scalar(other)),
    }
}

fn yaml_scalar(value: &JsonValue) -> String {
    match value {
        JsonValue::String(s) => format!("\"{s}\""),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Null => "null".to_string(),
        other => format!("{other}"),
    }
}

fn parse_json(s: &str) -> JsonValue {
    parse_lenient(s).map(|o| o.value).unwrap_or(JsonValue::Null)
}

// ── 文件 I/O 帮助函数 ─────────────────────────────────────────────────────────

/// 读取文件内容。
pub(crate) fn read_file(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let locale = get_locale();
    fs::read_to_string(path).map_err(|e| {
        t_to("err.read_failed", &locale)
            .replace("{0}", &path.display().to_string())
            .replace("{1}", &e.to_string())
            .into()
    })
}

/// 读取并宽松解析文件，返回文档和修复列表。
pub(crate) fn load_lenient(
    path: &Path,
) -> Result<(JsonValue, Vec<crate::engine::Repair>), Box<dyn std::error::Error>> {
    let locale = get_locale();
    let content = read_file(path)?;
    let output = parse_lenient(&content).map_err(|e| {
        t_to("err.parse_failed", &locale)
            .replace("{0}", &path.display().to_string())
            .replace("{1}", &e.to_string())
    })?;
    Ok((output.value, output.repairs))
}

/// 原子写入文件：写临时文件 → fsync → 重命名。
pub(crate) fn write_file_atomic(
    path: &Path,
    content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let locale = get_locale();
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, content)
        .map_err(|e| t_to("err.write_tmp_failed", &locale).replace("{0}", &e.to_string()))?;
    fs::rename(&tmp_path, path)
        .map_err(|e| t_to("err.rename_failed_file", &locale).replace("{0}", &e.to_string()))?;
    Ok(())
}
