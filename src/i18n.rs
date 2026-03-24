//! Internationalization (i18n) module for bilingual support.
//!
//! Supports English (en) and Chinese (zh-CN).
//! Language detection: `JED_LANG` > `LC_ALL`/`LANG` > Default (en)
//! Platform detection: Returns OS-specific modifier key names (Ctrl vs Cmd)

use std::env;

/// Get the current locale with proper detection priority:
/// 1. `JED_LANG` (explicit user setting)
/// 2. `LC_ALL`, `LC_MESSAGES`, `LANG` (system language)
/// 3. Default: English
pub fn get_locale() -> String {
    // 1. Explicit user configuration
    if let Ok(v) = env::var("JED_LANG")
        && !v.is_empty()
    {
        return normalize_locale(&v);
    }

    // 2. System language detection
    // LANGUAGE is a special GNU gettext variable (priority over LC_* variables)
    // It typically contains a colon-separated list like "zh_CN:en_US:en"
    let candidates = ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"];
    for var in candidates {
        if let Ok(v) = env::var(var)
            && !v.is_empty()
            && !v.starts_with("C.")
        {
            // For LANGUAGE variable, extract the first language code (before any colon)
            let locale_str = if var == "LANGUAGE" {
                v.split(':').next().unwrap_or(&v)
            } else {
                v.as_str()
            };
            return normalize_locale(locale_str);
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
        "tui.action.edit" | "tui.help.edit" => tr(locale, "编辑", "Edit"),
        "tui.action.add_child" => tr(locale, "添加子级", "Add Child"),
        "tui.action.add_sibling" => tr(locale, "添加兄弟", "Add Sibling"),
        "tui.action.delete" => tr(locale, "删除", "Delete"),
        "tui.action.copy_key" => tr(locale, "复制 Key", "Copy Key"),
        "tui.action.copy_value" => tr(locale, "复制 Value", "Copy Value"),
        "tui.action.copy_path" => tr(locale, "复制路径", "Copy Path"),
        "tui.action.expand_all" => tr(locale, "展开全部", "Expand All"),
        "tui.action.collapse_all" => tr(locale, "折叠全部", "Collapse All"),

        // TUI hints
        "tui.hint.context_menu" => tr(
            locale,
            " ↑↓:选择  Enter:执行  F2:菜单  Esc:退出",
            " ↑↓:Select  Enter:Execute  F2:Menu  Esc:Exit",
        ),
        "tui.hint.menu" => tr(locale, "菜单", "Menu"),
        "tui.hint.help" | "tui.help.title" => tr(locale, "帮助", "Help"),
        "tui.hint.move" => tr(locale, "移动", "Move"),
        "tui.hint.expand" => tr(locale, "展开", "Expand"),
        "tui.hint.new" => tr(locale, "新建", "New"),
        "tui.hint.search_key" | "tui.help.search" => tr(locale, "搜索", "Search"),
        "tui.hint.save" | "tui.help.save" => tr(locale, "保存", "Save"),
        "tui.hint.toggle" => tr(locale, "切换", "Toggle"),
        "tui.hint.confirm" | "tui.hint.enter_confirm" => tr(locale, "确认", "Confirm"),
        "tui.hint.cancel" | "tui.overlay.cancel" => tr(locale, "取消", "Cancel"),
        "tui.hint.next_match" => tr(locale, "下一匹配", "Next match"),
        "tui.hint.exit" => tr(locale, "退出", "Exit"),
        "tui.hint.close" => tr(locale, "关闭", "Close"),
        "tui.hint.save_quit" => tr(locale, "保存退出", "Save & Quit"),
        "tui.hint.no_save_quit" | "tui.overlay.quit_no_save" => {
            tr(locale, "不保存退出", "Quit No Save")
        }
        "tui.hint.select" => tr(locale, "选择", "Select"),
        "tui.hint.execute" => tr(locale, "执行", "Execute"),

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
        "tui.status.save_preview" | "tui.overlay.save_preview" => {
            tr(locale, " 保存预览 ", " Save Preview ")
        }
        "tui.status.save_confirm" => tr(locale, "保存确认: ", "Save confirm: "),
        "tui.status.change" => tr(locale, "变更: ", "Change: "),
        "tui.status.old_lines" => tr(locale, "旧: ", "Old: "),
        "tui.status.new_lines" => tr(locale, "→  新: ", "→  New: "),
        "tui.status.lines" => tr(locale, " 行", " lines"),
        "tui.status.string_as_str" => tr(
            locale,
            "string (将作为字符串保存)",
            "string (will be saved as string)",
        ),
        "tui.status.file_changed" => tr(
            locale,
            " 文件已变化！按 R 重新加载或 M 忽略",
            " File changed externally! Press R to reload or M to dismiss",
        ),
        "tui.status.reloading" => tr(locale, " 正在重新加载...", " Reloading..."),
        "tui.status.reload_failed" => tr(locale, " 重新加载失败：{0}", " Reload failed: {0}"),
        "tui.status.large_file" => tr(
            locale,
            " 大文件模式：{0}，自动展开受限",
            " Large file mode: {0}, auto-expand limited",
        ),
        "tui.status.watch_mode" => tr(locale, " 监控中", " Watching"),
        "tui.status.watch_disabled" => tr(locale, " 监控已禁用", " Watch disabled"),

        // TUI overlays
        "tui.overlay.edit" => tr(locale, " 编辑 {0} - {1} ", " Edit {0} - {1} "),
        "tui.overlay.rename_key" => tr(locale, " 重命名键 {0} ", " Rename Key {0} "),
        "tui.overlay.add_field" => tr(locale, " 添加字段到 {0} ", " Add Field to {0} "),
        "tui.overlay.select_type" => tr(locale, " 选择值类型 ", " Select Value Type "),
        "tui.overlay.type_null" => tr(locale, "null (默认)", "null (default)"),
        "tui.overlay.type_object" => tr(locale, "空对象", "Empty Object"),
        "tui.overlay.type_array" => tr(locale, "空数组", "Empty Array"),
        "tui.overlay.search" => tr(locale, " 搜索 ", " Search "),
        "tui.overlay.confirm_quit" => tr(locale, " 退出确认 ", " Confirm Quit "),
        "tui.overlay.file_modified" => tr(
            locale,
            " 文件已修改，是否保存？",
            " File modified. Save before quit? ",
        ),
        "tui.overlay.save_and_quit" => tr(locale, "保存并退出", "Save & Quit"),
        "tui.overlay.type_mismatch" => tr(locale, "⚠ {0} (原类型: {1})", "⚠ {0} (original: {1})"),
        "tui.status.string_unquoted" => tr(locale, "string (未加引号)", "string (unquoted)"),
        "tui.overlay.save_hint" => tr(
            locale,
            "[Enter] / [Y] 保存   [Esc] / [N] 取消",
            "[Enter] / [Y] Save   [Esc] / [N] Cancel",
        ),
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

        // Help panel
        "tui.help.help_title" => tr(locale, "快捷键帮助", "Keyboard Shortcuts"),
        "tui.help.nav" => tr(locale, "导航", "Navigation"),
        "tui.help.file" => tr(locale, "文件", "File"),
        "tui.help.close_help" => tr(
            locale,
            "按 [F1] / [Esc] / [Enter] 关闭帮助",
            "Press [F1] / [Esc] / [Enter] to close",
        ),
        "tui.help.undo" => tr(locale, "撤销", "Undo"),
        "tui.help.redo" => tr(locale, "重做", "Redo"),
        "tui.help.quit" => tr(locale, "退出 (连续按两次)", "Quit (press twice)"),
        "tui.help.move_up_down" => tr(locale, "上下移动", "Move up/down"),
        "tui.help.collapse_expand" => tr(
            locale,
            "折叠 / 展开或进入子节点",
            "Collapse / Expand or enter child",
        ),
        "tui.help.toggle_expand" => tr(locale, "切换展开 / 折叠", "Toggle expand/collapse"),
        "tui.help.quick_scroll" => tr(locale, "快速滚动 (一次10行)", "Quick scroll (10 lines)"),
        "tui.help.jump_begin_end" => tr(locale, "跳到开头 / 末尾", "Jump to start/end"),
        "tui.help.edit_value" => tr(locale, "编辑值 / 展开节点", "Edit value / Expand node"),
        "tui.help.new_node" => tr(locale, "新建节点", "New node"),
        "tui.help.delete_node" => tr(locale, "删除节点", "Delete node"),
        "tui.help.toggle_bool" => tr(
            locale,
            "编辑布尔值时切换 true/false",
            "Toggle true/false when editing bool",
        ),
        "tui.help.watch_reload" => tr(locale, "重新加载变化的文件", "Reload changed file"),
        "tui.help.watch_dismiss" => tr(locale, "忽略文件变化", "Dismiss file change"),
        "tui.help.watch_toggle" => tr(locale, "开关文件监控", "Toggle file watch"),

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
