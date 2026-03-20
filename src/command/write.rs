use std::path::Path;

use crate::{
    command::{exit_code, load_lenient, write_file_atomic},
    engine::{add, delete, format_pretty, move_value, set, FormatOptions, JsonValue},
};

/// `set <path> <value>` — 设置值，路径不存在时自动创建。
pub fn cmd_set(
    file: &Path,
    path: &str,
    raw_value: &str,
) -> Result<i32, Box<dyn std::error::Error>> {
    let (mut doc, _) = load_lenient(file)?;
    let value = parse_value_arg(raw_value)?;
    set(&mut doc, path, value)?;
    save(file, &doc)?;
    println!("ok");
    Ok(exit_code::OK)
}

/// `del <path>` — 删除 key 或数组元素。
pub fn cmd_del(file: &Path, path: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let (mut doc, _) = load_lenient(file)?;
    match delete(&mut doc, path) {
        Ok(_) => {
            save(file, &doc)?;
            println!("ok");
            Ok(exit_code::OK)
        }
        Err(e) => {
            eprintln!("删除失败：{e}");
            Ok(exit_code::NOT_FOUND)
        }
    }
}

/// `add <path> <value>` — 向数组追加，或向对象合并字段。
pub fn cmd_add(
    file: &Path,
    path: &str,
    raw_value: &str,
) -> Result<i32, Box<dyn std::error::Error>> {
    let (mut doc, _) = load_lenient(file)?;
    let value = parse_value_arg(raw_value)?;
    add(&mut doc, path, value)?;
    save(file, &doc)?;
    println!("ok");
    Ok(exit_code::OK)
}

/// `mv <src> <dst>` — 移动/重命名 key。
pub fn cmd_mv(
    file: &Path,
    src: &str,
    dst: &str,
) -> Result<i32, Box<dyn std::error::Error>> {
    let (mut doc, _) = load_lenient(file)?;
    move_value(&mut doc, src, dst)?;
    save(file, &doc)?;
    println!("ok");
    Ok(exit_code::OK)
}

/// `patch <operations>` — 批量操作（JSON Patch RFC 6902）。
pub fn cmd_patch(
    file: &Path,
    raw_ops: &str,
) -> Result<i32, Box<dyn std::error::Error>> {
    let (mut doc, _) = load_lenient(file)?;

    let ops: Vec<PatchOp> = serde_json::from_str(raw_ops)
        .map_err(|e| format!("patch 格式无效（期望 JSON Patch RFC 6902 数组）: {e}"))?;

    let mut applied = 0usize;

    // 先克隆文档，若出错则回滚
    let backup = doc.clone();

    for op in &ops {
        let result = apply_patch_op(&mut doc, op);
        if let Err(e) = result {
            // 回滚
            doc = backup;
            eprintln!("patch 操作 #{} 失败，已回滚：{e}", applied + 1);
            return Ok(exit_code::ERROR);
        }
        applied += 1;
    }

    save(file, &doc)?;
    println!("patched {applied} ops");
    Ok(exit_code::OK)
}

// ── Patch 内部实现 ────────────────────────────────────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct PatchOp {
    op: String,
    path: String,
    #[serde(default)]
    value: Option<serde_json::Value>,
    /// 用于 `move` 和 `copy` 操作
    #[serde(default)]
    from: Option<String>,
}

fn apply_patch_op(
    doc: &mut JsonValue,
    op: &PatchOp,
) -> Result<(), Box<dyn std::error::Error>> {
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
            let expected = op
                .value
                .as_ref()
                .ok_or("test 操作需要 'value' 字段")?;
            let actual = crate::engine::get(doc, &op.path)?;
            let expected_je = JsonValue::from(expected.clone());
            if *actual != expected_je {
                return Err(format!(
                    "test 断言失败：路径 {} 的值不符合预期",
                    op.path
                )
                .into());
            }
        }
        unknown => {
            return Err(format!("未知的 patch 操作：'{unknown}'").into());
        }
    }
    Ok(())
}

// ── 辅助函数 ─────────────────────────────────────────────────────────────────

/// 将命令行参数字符串解析为 JSON 值。
///
/// 优先尝试解析为 JSON，若失败则视为普通字符串。
fn parse_value_arg(raw: &str) -> Result<JsonValue, Box<dyn std::error::Error>> {
    match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(v) => Ok(JsonValue::from(v)),
        Err(_) => {
            // 裸字符串（不带引号）视为 JSON 字符串值
            Ok(JsonValue::String(raw.to_string()))
        }
    }
}

/// 将文档格式化后原子写入文件。
fn save(file: &Path, doc: &JsonValue) -> Result<(), Box<dyn std::error::Error>> {
    let content = format_pretty(doc, &FormatOptions::default());
    write_file_atomic(file, &content)
}
