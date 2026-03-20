use std::path::Path;

use crate::{
    command::{exit_code, load_lenient, read_file, write_file_atomic},
    engine::{fix_to_value, format_compact, format_pretty, FormatOptions},
};

/// `fmt` — 格式化 JSON 文件，原地修改。
pub fn cmd_fmt(file: &Path, indent: usize) -> Result<i32, Box<dyn std::error::Error>> {
    let (doc, _) = load_lenient(file)?;
    let opts = FormatOptions {
        indent,
        ..Default::default()
    };
    let content = format_pretty(&doc, &opts);
    write_file_atomic(file, &content)?;
    println!("ok");
    Ok(exit_code::OK)
}

/// `fix` — 自动修复 JSON 格式错误，然后格式化。
pub fn cmd_fix(
    file: &Path,
    dry_run: bool,
    strip_comments: bool,
) -> Result<i32, Box<dyn std::error::Error>> {
    let content = read_file(file)?;

    // 检查文件是否含注释
    let has_comments = content.contains("//") || content.contains("/*");
    if has_comments && !strip_comments {
        eprintln!(
            "警告：文件含注释（JSONC 格式），保存后注释将被移除。\n\
             使用 --strip-comments 标志确认操作，或使用 --dry-run 预览。"
        );
        if !dry_run {
            return Ok(exit_code::ERROR);
        }
    }

    let result = fix_to_value(&content);

    if result.has_unfixable() {
        for err in &result.errors {
            eprintln!("{err}");
        }
        return Ok(exit_code::ERROR);
    }

    let repair_count = result.repairs.len();

    if dry_run {
        println!("预览模式（不写入文件）：");
        for repair in &result.repairs {
            println!("  第 {} 行第 {} 列：{}", repair.line, repair.col, repair.description);
        }
        println!("共 {repair_count} 处修复");
        return Ok(exit_code::OK);
    }

    if let Some(doc) = result.value {
        let fixed_content = format_pretty(&doc, &FormatOptions::default());
        write_file_atomic(file, &fixed_content)?;
        if repair_count > 0 {
            println!("fixed {repair_count} errors");
            for repair in &result.repairs {
                println!("  第 {} 行第 {} 列：{}", repair.line, repair.col, repair.description);
            }
        } else {
            println!("ok（无需修复）");
        }
    }

    Ok(exit_code::OK)
}

/// `minify` — 压缩 JSON 文件，移除所有空白，原地修改。
pub fn cmd_minify(file: &Path) -> Result<i32, Box<dyn std::error::Error>> {
    let (doc, _) = load_lenient(file)?;
    let content = format_compact(&doc);
    write_file_atomic(file, &content)?;
    println!("ok");
    Ok(exit_code::OK)
}
