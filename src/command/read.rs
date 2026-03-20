use std::path::Path;

use crate::{
    command::{exit_code, load_lenient, read_file},
    engine::{exists, format_compact, format_pretty, get, infer_schema, parse_lenient, FormatOptions, PathError},
};

/// `get <path>` — 输出路径处的值，Agent 友好（最小化输出）。
pub fn cmd_get(file: &Path, path: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let (doc, _) = load_lenient(file)?;
    match get(&doc, path) {
        Ok(value) => {
            println!("{}", format_compact(value));
            Ok(exit_code::OK)
        }
        Err(PathError::KeyNotFound { key }) => {
            eprintln!("路径未找到：key '{key}' 不存在");
            Ok(exit_code::NOT_FOUND)
        }
        Err(PathError::IndexOutOfBounds { index, len }) => {
            eprintln!("路径未找到：索引 {index} 越界（长度 {len}）");
            Ok(exit_code::NOT_FOUND)
        }
        Err(e) => {
            eprintln!("路径错误：{e}");
            Ok(exit_code::TYPE_MISMATCH)
        }
    }
}

/// `keys <path>` — 每行输出一个 key 或索引。
pub fn cmd_keys(file: &Path, path: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let (doc, _) = load_lenient(file)?;
    let node = match get(&doc, path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("路径错误：{e}");
            return Ok(exit_code::NOT_FOUND);
        }
    };

    match node {
        crate::engine::JsonValue::Object(map) => {
            for key in map.keys() {
                println!("{key}");
            }
        }
        crate::engine::JsonValue::Array(arr) => {
            for i in 0..arr.len() {
                println!("{i}");
            }
        }
        other => {
            eprintln!("类型错误：{} 没有 key", other.type_name());
            return Ok(exit_code::TYPE_MISMATCH);
        }
    }
    Ok(exit_code::OK)
}

/// `len <path>` — 输出数组长度或对象 key 数量。
pub fn cmd_len(file: &Path, path: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let (doc, _) = load_lenient(file)?;
    let node = match get(&doc, path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("路径错误：{e}");
            return Ok(exit_code::NOT_FOUND);
        }
    };

    match node.len() {
        Some(n) => {
            println!("{n}");
            Ok(exit_code::OK)
        }
        None => {
            eprintln!("类型错误：{} 没有长度", node.type_name());
            Ok(exit_code::TYPE_MISMATCH)
        }
    }
}

/// `type <path>` — 输出值的类型名称。
pub fn cmd_type(file: &Path, path: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let (doc, _) = load_lenient(file)?;
    match get(&doc, path) {
        Ok(v) => {
            println!("{}", v.type_name());
            Ok(exit_code::OK)
        }
        Err(e) => {
            eprintln!("路径错误：{e}");
            Ok(exit_code::NOT_FOUND)
        }
    }
}

/// `exists <path>` — exit 0 表示存在，exit 2 表示不存在，无 stdout 输出。
pub fn cmd_exists(file: &Path, path: &str) -> Result<i32, Box<dyn std::error::Error>> {
    let (doc, _) = load_lenient(file)?;
    if exists(&doc, path) {
        Ok(exit_code::OK)
    } else {
        Ok(exit_code::NOT_FOUND)
    }
}

/// `schema` — 推断并输出文件结构（不含实际值）。
pub fn cmd_schema(file: &Path) -> Result<i32, Box<dyn std::error::Error>> {
    let (doc, _) = load_lenient(file)?;
    println!("{}", infer_schema(&doc));
    Ok(exit_code::OK)
}

/// `check` — 校验 JSON，成功无输出，错误输出到 stderr。
pub fn cmd_check(file: &Path) -> Result<i32, Box<dyn std::error::Error>> {
    let content = read_file(file)?;
    match parse_lenient(&content) {
        Ok(_) => Ok(exit_code::OK),
        Err(e) => {
            eprintln!("{e}");
            Ok(exit_code::ERROR)
        }
    }
}

/// `diff <other>` — 输出两个 JSON 文件的结构差异。
pub fn cmd_diff(
    file: &Path,
    other: &Path,
) -> Result<i32, Box<dyn std::error::Error>> {
    let (a, _) = load_lenient(file)?;
    let (b, _) = load_lenient(other)?;

    let a_str = format_pretty(&a, &FormatOptions::default());
    let b_str = format_pretty(&b, &FormatOptions::default());

    if a_str == b_str {
        // 无差异
        Ok(exit_code::OK)
    } else {
        // 简单行级别 diff
        let a_lines: Vec<&str> = a_str.lines().collect();
        let b_lines: Vec<&str> = b_str.lines().collect();

        let max = a_lines.len().max(b_lines.len());
        let mut has_diff = false;
        for i in 0..max {
            match (a_lines.get(i), b_lines.get(i)) {
                (Some(al), Some(bl)) if al == bl => {}
                (Some(al), Some(bl)) => {
                    println!("- {al}");
                    println!("+ {bl}");
                    has_diff = true;
                }
                (Some(al), None) => {
                    println!("- {al}");
                    has_diff = true;
                }
                (None, Some(bl)) => {
                    println!("+ {bl}");
                    has_diff = true;
                }
                (None, None) => {}
            }
        }

        if has_diff {
            Ok(exit_code::ERROR)
        } else {
            Ok(exit_code::OK)
        }
    }
}
