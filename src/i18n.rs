//! Internationalization (i18n) module for bilingual support.
//!
//! Supports English (en) and Chinese (zh-CN).
//! Language detection: `JE_LANG` > `LC_ALL`/`LANG` > Default (en)
//! Platform detection: Returns OS-specific modifier key names (Ctrl vs Cmd)

use std::env;
use std::env::consts::OS;

/// Get the current locale with proper detection priority:
/// 1. `JE_LANG` (explicit user setting)
/// 2. `LC_ALL`, `LC_MESSAGES`, `LANG` (system language)
/// 3. Default: English
pub fn get_locale() -> String {
    // 1. Explicit user configuration
    if let Ok(v) = env::var("JE_LANG")
        && !v.is_empty()
    {
        return normalize_locale(&v);
    }

    // 2. System language detection
    let candidates = ["LC_ALL", "LC_MESSAGES", "LANG"];
    for var in candidates {
        if let Ok(v) = env::var(var)
            && !v.is_empty()
            && !v.starts_with("C.")
        {
            return normalize_locale(&v);
        }
    }

    // 3. Default
    "en".to_string()
}

/// Normalize locale string to supported language code.
fn normalize_locale(locale: &str) -> String {
    let l = locale.to_lowercase();
    if l.starts_with("zh") {
        if l.contains("tw") || l.contains("hk") || l.contains("taiwan") {
            return "zh-TW".to_string();
        }
        return "zh-CN".to_string();
    }
    "en".to_string()
}

/// Get list of supported locales.
#[allow(dead_code)]
pub fn supported_locales() -> Vec<&'static str> {
    vec!["en", "zh-CN", "zh-TW"]
}

/// Platform-specific modifier key display.
/// Returns "Ctrl" on Windows/Linux, "⌘" on macOS.
pub fn modifier_key() -> &'static str {
    if OS == "macos" {
        "⌘"
    } else {
        "Ctrl"
    }
}

/// Platform-specific shortcut prefix.
/// Returns "⌘" for macOS, "Ctrl" for others.
pub fn modifier_key_shortcut() -> &'static str {
    if OS == "macos" {
        "⌘"
    } else {
        "Ctrl"
    }
}

/// Check if the current platform is macOS.
pub fn is_macos() -> bool {
    OS == "macos"
}

/// Translate a key to the current locale.
#[allow(dead_code)]
pub fn t(key: &str) -> String {
    t_to(key, &get_locale())
}

/// Translate a key to a specific locale.
#[allow(clippy::too_many_lines)]
pub fn t_to(key: &str, locale: &str) -> String {
    match key {
        // Main / CLI
        "main.tui_error" => tr(locale, "TUI 错误", "TUI Error"),
        "main.need_file" => tr(
            locale,
            "错误：需要指定 JSON 文件路径",
            "Error: JSON file path required",
        ),
        "cli.about" => tr(
            locale,
            "JSON 编辑器：同时为人类和 AI Agent 设计的双接口工具",
            "JSON editor: dual-interface tool for humans and AI agents",
        ),

        // Status messages
        "status.ok" => "ok".into(),
        "status.formatted" => "formatted".into(),
        "status.minified" => "minified".into(),
        "status.identical" => "identical".into(),
        "status.saved" => tr(locale, "已保存", "Saved"),
        "status.updated" => tr(locale, "已更新", "Updated"),
        "status.renamed" => tr(locale, "已重命名", "Renamed"),
        "status.deleted" => tr(locale, "已删除", "Deleted"),
        "status.added" => tr(locale, "已添加", "Added"),

        // Errors - read
        "err.key_not_found" => tr(
            locale,
            "路径未找到：key '{0}' 不存在",
            "Path not found: key '{0}' does not exist",
        ),
        "err.index_oob" => tr(
            locale,
            "路径未找到：索引 {0} 越界（长度 {1}）",
            "Path not found: index {0} out of bounds (length {1})",
        ),
        "err.path" => tr(locale, "路径错误：{0}", "Path error: {0}"),
        "err.type_no_keys" => tr(
            locale,
            "类型错误：{0} 没有 key",
            "Type error: {0} has no keys",
        ),
        "err.type_no_len" => tr(
            locale,
            "类型错误：{0} 没有长度",
            "Type error: {0} has no length",
        ),
        "err.path_not_exists" => tr(locale, "路径不存在", "path does not exist"),

        // Errors - write
        "err.delete_failed" => tr(locale, "删除失败：{0}", "Delete failed: {0}"),
        "err.patch_format" => tr(
            locale,
            "patch 格式无效（期望 JSON Patch RFC 6902 数组）: {0}",
            "Invalid patch format (expected JSON Patch RFC 6902 array): {0}",
        ),
        "err.patch_op_failed" => tr(
            locale,
            "patch 操作 #{0} 失败，已回滚：{1}",
            "Patch operation #{0} failed, rolled back: {1}",
        ),
        "err.patch_need_value" => tr(
            locale,
            "add/replace 操作需要 'value' 字段",
            "add/replace operation requires 'value' field",
        ),
        "err.patch_need_from" => tr(
            locale,
            "{0} 操作需要 'from' 字段",
            "{0} operation requires 'from' field",
        ),
        "err.patch_test_failed" => tr(
            locale,
            "test 断言失败：路径 {0} 的值不符合预期",
            "Test assertion failed: value at path {0} does not match expected",
        ),
        "err.patch_unknown" => tr(
            locale,
            "未知的 patch 操作：'{0}'",
            "Unknown patch operation: '{0}'",
        ),
        "err.add_failed" => tr(locale, "添加失败: {0}", "Add failed: {0}"),
        "err.edit_failed" => tr(locale, "编辑失败：{0}", "Edit failed: {0}"),
        "err.rename_failed" => tr(locale, "重命名失败：{0}", "Rename failed: {0}"),

        // Errors - repair
        "err.fmt_has_issues" => tr(
            locale,
            "文件含 {0} 个格式问题，请先使用 `fix` 修复",
            "File has {0} format issues, use `fix` to repair first",
        ),
        "err.has_comments" => tr(
            locale,
            "文件含注释，使用 `--strip-comments` 剥离注释或手动处理",
            "File contains comments, use `--strip-comments` to strip or handle manually",
        ),
        "err.no_repairs_needed" => tr(locale, "无需修复", "No repairs needed"),
        "err.total_repairs" => tr(locale, "共 {0} 处修复", "{0} repairs total"),
        "err.no_value_after_fix" => tr(locale, "修复后无有效值", "No valid value after fix"),

        // Errors - file I/O
        "err.read_failed" => tr(locale, "无法读取 '{0}': {1}", "Cannot read '{0}': {1}"),
        "err.parse_failed" => tr(locale, "解析失败 '{0}': {1}", "Parse failed '{0}': {1}"),
        "err.write_tmp_failed" => tr(
            locale,
            "写入临时文件失败: {0}",
            "Failed to write temp file: {0}",
        ),
        "err.rename_failed_file" => tr(locale, "重命名文件失败: {0}", "Failed to rename file: {0}"),
        "err.save_failed" => tr(locale, "保存失败：{0}", "Save failed: {0}"),

        // TUI actions
        "tui.action.edit" => tr(locale, "编辑", "Edit"),
        "tui.action.add_child" => tr(locale, "添加子级", "Add Child"),
        "tui.action.add_sibling" => tr(locale, "添加兄弟", "Add Sibling"),
        "tui.action.delete" => tr(locale, "删除", "Delete"),
        "tui.action.copy_key" => tr(locale, "复制 Key", "Copy Key"),
        "tui.action.copy_value" => tr(locale, "复制 Value", "Copy Value"),
        "tui.action.copy_path" => tr(locale, "复制路径", "Copy Path"),
        "tui.action.expand_all" => tr(locale, "展开全部", "Expand All"),
        "tui.action.collapse_all" => tr(locale, "折叠全部", "Collapse All"),

        // TUI hints
        "tui.hint.normal" => tr(
            locale,
            " Alt:菜单 ↑↓:移动 ←:折叠 →/Space:展开 Enter:编辑 N:新建 Del:删除 Ctrl+S:保存 Ctrl+F:搜索 ",
            " Alt:Menu ↑↓:Move ←:Collapse →/Space:Expand Enter:Edit N:New Del:Delete Ctrl+S:Save Ctrl+F:Search ",
        ),
        "tui.hint.edit" => tr(
            locale,
            " 输入值  Enter:确认  Esc:取消",
            " Enter value  Enter:Confirm  Esc:Cancel",
        ),
        "tui.hint.edit_key" => tr(
            locale,
            " 输入新键名  Enter:确认  Esc:取消",
            " Enter new key  Enter:Confirm  Esc:Cancel",
        ),
        "tui.hint.search" => tr(
            locale,
            " 输入搜索  Enter:跳转下一匹配  Esc:退出",
            " Enter search  Enter:Next match  Esc:Exit",
        ),
        "tui.hint.add_node" => tr(
            locale,
            " 输入字段名  Enter:确认  Esc:取消",
            " Enter field name  Enter:Confirm  Esc:Cancel",
        ),
        "tui.hint.confirm_strip" => tr(
            locale,
            " [Y]:确认保存  [N]:取消  ",
            " [Y]:Confirm  [N]:Cancel  ",
        ),
        "tui.hint.context_menu" => tr(
            locale,
            " ↑↓:选择  Enter:执行  F2:菜单  Esc:退出",
            " ↑↓:Select  Enter:Execute  F2:Menu  Esc:Exit",
        ),
        "tui.hint.help" => tr(
            locale,
            " ↑↓:移动 ←:折叠 →/Space:展开 Enter:编辑 N:新建 Del:删除 Ctrl+S:保存 Ctrl+F:搜索 F1:帮助 ",
            " ↑↓:Move ←:Collapse →/Space:Expand Enter:Edit N:New Del:Delete Ctrl+S:Save Ctrl+F:Search F1:Help ",
        ),

        // TUI status messages
        "tui.status.edit_value_only" => tr(
            locale,
            "只能编辑基本类型的值（string/number/boolean/null）",
            "Can only edit primitive values (string/number/boolean/null)",
        ),
        "tui.status.cannot_rename_root" => {
            tr(locale, "不能重命名根节点", "Cannot rename root node")
        }
        "tui.status.cannot_rename_index" => {
            tr(locale, "数组索引不能重命名", "Cannot rename array index")
        }
        "tui.status.key_empty" => tr(locale, "key 不能为空", "key cannot be empty"),
        "tui.status.cannot_delete_root" => tr(locale, "不能删除根节点", "Cannot delete root node"),
        "tui.status.no_key" => tr(locale, "当前节点没有 key", "Current node has no key"),
        "tui.status.no_value" => tr(locale, "当前节点没有 value", "Current node has no value"),
        "tui.status.copy_failed" => tr(locale, "复制失败：{0}", "Copy failed: {0}"),
        "tui.status.copied_key" => tr(locale, "已复制 key: {0}", "Copied key: {0}"),
        "tui.status.copied_value" => tr(locale, "已复制 value", "Copied value"),
        "tui.status.copied_path" => tr(locale, "已复制路径: {0}", "Copied path: {0}"),
        "tui.status.expanded_all" => tr(locale, "已展开全部节点", "Expanded all nodes"),
        "tui.status.collapsed_all" => tr(locale, "已折叠全部节点", "Collapsed all nodes"),
        "tui.status.add_sibling_wip" => tr(
            locale,
            "添加兄弟节点功能开发中",
            "Add sibling feature is work in progress",
        ),
        "tui.status.no_undo" => tr(locale, "没有可撤销的操作", "No operations to undo"),
        "tui.status.undone" => tr(locale, "已撤销", "Undone"),
        "tui.status.no_redo" => tr(locale, "没有可重做的操作", "No operations to redo"),
        "tui.status.redone" => tr(locale, "已重做", "Redone"),
        "tui.status.file_modified" => tr(
            locale,
            " 文件已修改！Ctrl+S 保存，Ctrl+Q 强制退出 ",
            " File modified! Ctrl+S to save, Ctrl+Q to quit ",
        ),
        "tui.status.cancel_save" => tr(locale, "已取消保存", "Save cancelled"),
        "tui.status.need_field_name" => tr(locale, "需要输入字段名", "Need to enter field name"),
        "tui.status.added_null" => tr(locale, "已添加空元素", "Added null element"),
        "tui.status.no_changes" => tr(locale, "文件无变化，无需保存", "No changes to save"),
        "tui.status.save_preview" => tr(locale, " 保存预览 ", " Save Preview "),
        "tui.status.save_confirm" => tr(locale, "保存确认: ", "Save confirm: "),
        "tui.status.change" => tr(locale, "变更: ", "Change: "),
        "tui.status.old_lines" => tr(locale, "旧: ", "Old: "),
        "tui.status.new_lines" => tr(locale, "→  新: ", "→  New: "),
        "tui.status.lines" => tr(locale, " 行", " lines"),
        "tui.status.string_as_str" => tr(locale, "string (将作为字符串保存)", "string (will be saved as string)"),

        // TUI overlays
        "tui.overlay.edit" => tr(locale, " 编辑 {0} - {1} ", " Edit {0} - {1} "),
        "tui.overlay.rename_key" => tr(locale, " 重命名键 {0} ", " Rename Key {0} "),
        "tui.overlay.add_field" => tr(locale, " 添加字段到 {0} ", " Add Field to {0} "),
        "tui.overlay.search" => tr(locale, " 搜索 ", " Search "),
        "tui.confirm.has_comments" => tr(
            locale,
            " 此文件含有注释（JSONC 格式）。",
            " This file contains comments (JSONC format).",
        ),
        "tui.confirm.strip_warn" => tr(
            locale,
            " 保存后注释将被移除，是否继续？",
            " Comments will be removed after saving. Continue?",
        ),
        "tui.confirm.yes_no" => tr(
            locale,
            "   [ Y ] 确认    [ N ] 取消   ",
            "   [ Y ] Confirm   [ N ] Cancel   ",
        ),
        "tui.confirm.notice" => tr(locale, " 注意 ", " Notice "),
        "tui.confirm.actions" => tr(locale, " Actions ", " Actions "),

        _ => key.to_string(),
    }
}

/// Helper for binary translation: (zh, en).
#[inline]
#[allow(clippy::trivially_copy_pass_by_ref)]
fn tr(locale: &str, cn: &str, en: &str) -> String {
    if locale.starts_with("zh") {
        cn.to_string()
    } else {
        en.to_string()
    }
}
