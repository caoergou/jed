use std::path::Path;

use crate::{
    command::{exit_code, load_lenient, read_file, write_file_atomic},
    engine::{fix_to_value, format_compact, format_pretty, FormatOptions},
    i18n::{get_locale, t_to},
    output::Ctx,
};

/// `fmt` — 格式化（美化）JSON，原地修改。
pub fn cmd_fmt(file: &Path, indent: usize, ctx: &Ctx) -> Result<i32, Box<dyn std::error::Error>> {
    let file_str = file.display().to_string();
    let (doc, repairs) = load_lenient(file)?;
    if !repairs.is_empty() {
        let locale = get_locale();
        let msg = t_to("err.fmt_has_issues", &locale).replace("{0}", &repairs.len().to_string());
        let fix = format!("Run 'jed fix {file_str}' to auto-repair these issues first");
        ctx.print_error(&msg, Some(&fix), &[format!("jed fix {file_str}")]);
        return Ok(exit_code::ERROR);
    }
    let content = format_pretty(
        &doc,
        &FormatOptions {
            indent,
            trailing_newline: true,
            sort_keys: false,
        },
    );
    write_file_atomic(file, &content)?;
    let actions = vec![format!("jed check {file_str}")];
    ctx.print_ok_with_actions("formatted", &actions);
    Ok(exit_code::OK)
}

/// `fix` — 自动修复格式错误，然后格式化。
pub fn cmd_fix(
    file: &Path,
    dry_run: bool,
    strip_comments: bool,
    ctx: &Ctx,
) -> Result<i32, Box<dyn std::error::Error>> {
    let locale = get_locale();
    let file_str = file.display().to_string();
    let content = read_file(file)?;

    let has_comments = content.contains("//") || content.contains("/*");
    if has_comments && !strip_comments {
        let msg = t_to("err.has_comments", &locale);
        let fix = "Re-run with '--strip-comments' to remove comments before fixing";
        let actions = vec![format!("jed fix --strip-comments {file_str}")];
        ctx.print_error(&msg, Some(fix), &actions);
        return Ok(exit_code::ERROR);
    }

    let result = fix_to_value(&content);

    if !result.errors.is_empty() {
        for err in &result.errors {
            ctx.print_error(&format!("{err}"), None, &[]);
        }
        return Ok(exit_code::ERROR);
    }

    if dry_run {
        if result.repairs.is_empty() {
            ctx.print_raw(serde_json::json!({"repairs_needed": 0, "repairs": []}));
        } else {
            let repairs: Vec<serde_json::Value> = result
                .repairs
                .iter()
                .map(|r| {
                    serde_json::json!({
                        "line": r.line,
                        "description": r.description,
                    })
                })
                .collect();
            let count = repairs.len();
            let actions = vec![format!("jed fix {file_str}")];
            ctx.print_raw_with_actions(
                serde_json::json!({"repairs_needed": count, "repairs": repairs}),
                &actions,
            );
        }
        return Ok(exit_code::OK);
    }

    let value = result
        .value
        .ok_or_else(|| t_to("err.no_value_after_fix", &locale))?;
    let fixed_content = format_pretty(
        &value,
        &FormatOptions {
            indent: 2,
            trailing_newline: true,
            sort_keys: false,
        },
    );
    write_file_atomic(file, &fixed_content)?;
    let count = result.repairs.len();
    let actions = vec![
        format!("jed check {file_str}"),
        format!("jed fmt {file_str}"),
    ];
    ctx.print_raw_with_actions(
        serde_json::json!({"fixed": count}),
        &actions,
    );
    Ok(exit_code::OK)
}

/// `minify` — 压缩 JSON（移除所有空白），原地修改。
pub fn cmd_minify(file: &Path, ctx: &Ctx) -> Result<i32, Box<dyn std::error::Error>> {
    let file_str = file.display().to_string();
    let (doc, _) = load_lenient(file)?;
    let content = format_compact(&doc);
    write_file_atomic(file, &content)?;
    let actions = vec![format!("jed check {file_str}")];
    ctx.print_ok_with_actions("minified", &actions);
    Ok(exit_code::OK)
}
