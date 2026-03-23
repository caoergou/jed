use std::path::Path;

use crate::{
    command::{exit_code, load_lenient, write_file_atomic},
    engine::{add, delete, format_pretty, get, move_value, set, FormatOptions, JsonValue},
    i18n::{get_locale, t_to},
    output::Ctx,
};

/// `set <path> <value>` — 设置值，路径不存在时自动创建。
pub fn cmd_set(
    file: &Path,
    path: &str,
    raw_value: &str,
    ctx: &Ctx,
) -> Result<i32, Box<dyn std::error::Error>> {
    let file_str = file.display().to_string();
    let (mut doc, _) = load_lenient(file)?;
    let value = parse_value_arg(raw_value);
    set(&mut doc, path, value)?;
    save(file, &doc)?;
    let actions = vec![format!("jed get {path} {file_str}")];
    // Return the new value as confirmation
    if let Ok(new_val) = get(&doc, path) {
        ctx.print_value_with_actions(new_val, &actions);
    } else {
        ctx.print_ok_with_actions("ok", &actions);
    }
    Ok(exit_code::OK)
}

/// `del <path>` — 删除 key 或数组元素。
pub fn cmd_del(file: &Path, path: &str, ctx: &Ctx) -> Result<i32, Box<dyn std::error::Error>> {
    let locale = get_locale();
    let file_str = file.display().to_string();
    let (mut doc, _) = load_lenient(file)?;
    match delete(&mut doc, path) {
        Ok(_) => {
            save(file, &doc)?;
            let actions = vec![format!("jed keys . {file_str}")];
            ctx.print_ok_with_actions("deleted", &actions);
            Ok(exit_code::OK)
        }
        Err(e) => {
            let msg = t_to("err.delete_failed", &locale).replace("{0}", &e.to_string());
            let fix = format!("Run 'jed keys . {file_str}' to verify the path exists");
            ctx.print_error(&msg, Some(&fix), &[format!("jed keys . {file_str}")]);
            Ok(exit_code::NOT_FOUND)
        }
    }
}

/// `add <path> <value>` — 向数组追加，或向对象合并字段。
pub fn cmd_add(
    file: &Path,
    path: &str,
    raw_value: &str,
    ctx: &Ctx,
) -> Result<i32, Box<dyn std::error::Error>> {
    let file_str = file.display().to_string();
    let (mut doc, _) = load_lenient(file)?;
    let value = parse_value_arg(raw_value);
    add(&mut doc, path, value)?;
    save(file, &doc)?;
    let actions = vec![
        format!("jed len {path} {file_str}"),
        format!("jed get {path} {file_str}"),
    ];
    if let Ok(updated) = get(&doc, path) {
        ctx.print_value_with_actions(updated, &actions);
    } else {
        ctx.print_ok_with_actions("added", &actions);
    }
    Ok(exit_code::OK)
}

/// `mv <src> <dst>` — 移动/重命名 key。
pub fn cmd_mv(
    file: &Path,
    src: &str,
    dst: &str,
    ctx: &Ctx,
) -> Result<i32, Box<dyn std::error::Error>> {
    let file_str = file.display().to_string();
    let (mut doc, _) = load_lenient(file)?;
    move_value(&mut doc, src, dst)?;
    save(file, &doc)?;
    let actions = vec![format!("jed get {dst} {file_str}")];
    if let Ok(moved_val) = get(&doc, dst) {
        ctx.print_value_with_actions(moved_val, &actions);
    } else {
        ctx.print_ok_with_actions("moved", &actions);
    }
    Ok(exit_code::OK)
}

/// `patch <operations>` — 批量操作（JSON Patch RFC 6902）。
pub fn cmd_patch(
    file: &Path,
    raw_ops: &str,
    ctx: &Ctx,
) -> Result<i32, Box<dyn std::error::Error>> {
    let locale = get_locale();
    let file_str = file.display().to_string();
    let (mut doc, _) = load_lenient(file)?;

    let ops: Vec<PatchOp> = serde_json::from_str(raw_ops)
        .map_err(|e| t_to("err.patch_format", &locale).replace("{0}", &e.to_string()))?;

    let mut applied = 0usize;

    for op in &ops {
        let result = apply_patch_op(&mut doc, op);
        if let Err(e) = result {
            let msg = t_to("err.patch_op_failed", &locale)
                .replace("{0}", &(applied + 1).to_string())
                .replace("{1}", &e.to_string());
            let fix = "Check operation path and value, then retry";
            ctx.print_error(&msg, Some(fix), &[]);
            return Ok(exit_code::ERROR);
        }
        applied += 1;
    }

    save(file, &doc)?;
    let actions = vec![format!("jed get . {file_str}")];
    ctx.print_raw_with_actions(
        serde_json::json!({"patched": applied}),
        &actions,
    );
    Ok(exit_code::OK)
}

// ── Patch 内部实现 ───────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct PatchOp {
    op: String,
    path: String,
    #[serde(default)]
    value: Option<serde_json::Value>,
    #[serde(default)]
    from: Option<String>,
}

fn apply_patch_op(doc: &mut JsonValue, op: &PatchOp) -> Result<(), Box<dyn std::error::Error>> {
    match op.op.as_str() {
        "add" | "replace" => {
            let val = op
                .value
                .as_ref()
                .ok_or("add/replace 操作需要 'value' 字段")?;
            let je_val = JsonValue::from(val.clone());
            set(doc, &op.path, je_val)?;
        }
        "remove" => {
            delete(doc, &op.path)?;
        }
        "move" => {
            let from = op.from.as_deref().ok_or("move 操作需要 'from' 字段")?;
            move_value(doc, from, &op.path)?;
        }
        "copy" => {
            let from = op.from.as_deref().ok_or("copy 操作需要 'from' 字段")?;
            let val = crate::engine::get(doc, from)?.clone();
            set(doc, &op.path, val)?;
        }
        "test" => {
            let expected = op.value.as_ref().ok_or("test 操作需要 'value' 字段")?;
            let actual = crate::engine::get(doc, &op.path)?;
            let expected_je = JsonValue::from(expected.clone());
            if *actual != expected_je {
                return Err(
                    format!("test 断言失败：路径 {} 的值不符合预期", op.path).into(),
                );
            }
        }
        unknown => {
            return Err(format!("未知的 patch 操作：'{unknown}'").into());
        }
    }
    Ok(())
}

// ── 辅助函数 ─────────────────────────────────────────────────────────────────

fn parse_value_arg(raw: &str) -> JsonValue {
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(v) => JsonValue::from(v),
        Err(_) => JsonValue::String(raw.to_string()),
    }
}

fn save(file: &Path, doc: &JsonValue) -> Result<(), Box<dyn std::error::Error>> {
    let content = format_pretty(doc, &FormatOptions::default());
    write_file_atomic(file, &content)
}
