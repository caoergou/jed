#![allow(clippy::collapsible_else_if)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::cast_possible_truncation)]

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use super::app::{App, AppMode, ContextAction, StatusLevel};
use super::tree::TreeLine;
use crate::i18n::{get_locale, t_to};

/// 每帧的主渲染入口。
pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let lines = app.tree_lines();

    // 布局：树形主区域 + 底部状态栏 + 快捷键提示条
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

    render_tree(frame, app, chunks[0], &lines);
    render_statusbar(frame, app, chunks[1], &lines);
    render_helpbar(frame, app, chunks[2]);

    // 编辑覆盖层（值）
    if matches!(app.mode, AppMode::Edit { .. }) {
        render_edit_overlay(frame, app, area);
    }

    // 编辑覆盖层（键名）
    if matches!(app.mode, AppMode::EditKey { .. }) {
        render_edit_key_overlay(frame, app, area);
    }

    // 搜索覆盖层
    if matches!(app.mode, AppMode::Search { .. }) {
        render_search_overlay(frame, app, area);
    }

    // 添加节点覆盖层
    if matches!(app.mode, AppMode::AddNode { .. }) {
        render_add_node_overlay(frame, app, area);
    }

    // 帮助面板覆盖层
    if matches!(app.mode, AppMode::Help) {
        render_help_panel(frame, area);
    }

    // 退出确认覆盖层
    if matches!(app.mode, AppMode::ConfirmQuit { .. }) {
        render_confirm_quit_overlay(frame, area);
    }

    // 保存预览覆盖层
    if matches!(app.mode, AppMode::ConfirmSave { .. }) {
        render_save_preview(frame, app, area);
    }

    // 右键菜单覆盖层
    if matches!(app.mode, AppMode::ContextMenu { .. }) {
        render_context_menu(frame, app, area);
    }
}

// ── 树形视图 ─────────────────────────────────────────────────────────────────

fn render_tree(frame: &mut Frame, app: &mut App, area: Rect, lines: &[TreeLine]) {
    let modified_marker = if app.modified { " [*]" } else { "" };
    let title = format!(
        " jed: {}{modified_marker} ",
        app.file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
    );

    // 搜索模式下高亮匹配项
    let search_query = if let AppMode::Search { query, .. } = &app.mode {
        if query.is_empty() {
            None
        } else {
            Some(query.to_lowercase())
        }
    } else {
        None
    };

    let items: Vec<ListItem> = lines
        .iter()
        .map(|line| make_list_item(line, search_query.as_deref()))
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(
                    title,
                    Style::default().add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn make_list_item<'a>(line: &'a TreeLine, search_query: Option<&'a str>) -> ListItem<'a> {
    let indent = "  ".repeat(line.depth);

    // 展开/折叠指示符
    let indicator = if line.path.starts_with("__close__") {
        "  "
    } else if line.has_children {
        if line.is_expanded { "▼ " } else { "▶ " }
    } else {
        "  "
    };

    // 检查是否匹配搜索
    let is_match = search_query.is_some_and(|q| {
        line.display_key.to_lowercase().contains(q) || line.value_preview.to_lowercase().contains(q)
    });

    // 搜索匹配时的高亮样式
    let match_style = if is_match {
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::REVERSED)
    } else {
        Style::default()
    };

    // key 部分的颜色
    let key_span = if line.display_key.is_empty() {
        Span::raw("")
    } else {
        Span::styled(
            format!("{}: ", line.display_key),
            Style::default().fg(Color::Cyan),
        )
    };

    // 值的颜色
    let value_color = match line.value_type {
        "string" => Color::Green,
        "number" => Color::Yellow,
        "boolean" => Color::Magenta,
        "null" => Color::DarkGray,
        _ => Color::White,
    };

    let value_span = Span::styled(line.value_preview.clone(), Style::default().fg(value_color));

    ListItem::new(Line::from(vec![
        Span::styled(format!("{indent}{indicator}"), match_style),
        key_span,
        value_span,
    ]))
    .style(match_style)
}

// ── 状态栏 ───────────────────────────────────────────────────────────────────

fn render_statusbar(frame: &mut Frame, app: &App, area: Rect, _lines: &[TreeLine]) {
    let path = app.current_path();

    let status_text = if let Some((msg, level)) = &app.status {
        let color = match level {
            StatusLevel::Info => Color::Green,
            StatusLevel::Warn => Color::Yellow,
            StatusLevel::Error => Color::Red,
        };
        vec![
            Span::styled(format!(" {path} ",), Style::default().fg(Color::DarkGray)),
            Span::styled("│", Style::default().fg(Color::DarkGray)),
            Span::styled(format!(" {msg} ",), Style::default().fg(color)),
        ]
    } else {
        vec![Span::styled(
            format!(" {path} ",),
            Style::default().fg(Color::DarkGray),
        )]
    };

    let bar = Paragraph::new(Line::from(status_text)).style(Style::default().bg(Color::Black));
    frame.render_widget(bar, area);
}

/// 底部快捷键提示条
fn render_helpbar(frame: &mut Frame, app: &App, area: Rect) {
    let locale = get_locale();

    // 辅助函数：创建带颜色的单个快捷键
    let key = |k: &str| -> String { format!("[{k}]") };

    // 辅助函数：创建组合键 [Ctrl]+[S] 格式
    let combo = |c: &str, k: &str| -> String {
        let uk = k.to_uppercase();
        format!("[{c}]+[{uk}]")
    };

    // 获取修饰键文本
    let ctrl = if cfg!(target_os = "macos") {
        "⌘"
    } else {
        "Ctrl"
    };

    // 使用格式化后的键位 - Normal 模式下只显示核心快捷键，更多快捷键请按 F1
    let hints: Vec<(String, String)> = match &app.mode {
        AppMode::Normal => vec![
            (key("↑↓"), t_to("tui.hint.move", &locale)),
            (key("Enter"), t_to("tui.help.edit_value", &locale)),
            (key("N"), t_to("tui.hint.new", &locale)),
            (combo(ctrl, "S"), t_to("tui.hint.save", &locale)),
            (key("F1"), t_to("tui.hint.help", &locale)),
        ],
        AppMode::Edit { value_type, .. } => {
            if *value_type == "boolean" {
                vec![
                    (key("Tab"), t_to("tui.hint.toggle", &locale)),
                    (key("Enter"), t_to("tui.hint.confirm", &locale)),
                    (key("Esc"), t_to("tui.hint.cancel", &locale)),
                ]
            } else {
                vec![
                    (key("Enter"), t_to("tui.hint.confirm", &locale)),
                    (key("Esc"), t_to("tui.hint.cancel", &locale)),
                ]
            }
        }
        AppMode::EditKey { .. } | AppMode::AddNode { .. } => vec![
            (key("Enter"), t_to("tui.hint.confirm", &locale)),
            (key("Esc"), t_to("tui.hint.cancel", &locale)),
        ],
        AppMode::Search { .. } => vec![
            (key("Enter"), t_to("tui.hint.next_match", &locale)),
            (key("Esc"), t_to("tui.hint.exit", &locale)),
        ],
        AppMode::Help => vec![(key("Esc"), t_to("tui.hint.close", &locale))],
        AppMode::ConfirmQuit { .. } => vec![
            (key("Y"), t_to("tui.hint.save_quit", &locale)),
            (key("N"), t_to("tui.hint.no_save_quit", &locale)),
            (
                key("C") + " / " + &key("Esc"),
                t_to("tui.hint.cancel", &locale),
            ),
        ],
        AppMode::ConfirmSave { .. } => vec![
            (key("Enter"), t_to("tui.hint.save", &locale)),
            (key("Esc"), t_to("tui.hint.cancel", &locale)),
        ],
        AppMode::ContextMenu { .. } => vec![
            (key("↑↓"), t_to("tui.hint.select", &locale)),
            (key("Enter"), t_to("tui.hint.execute", &locale)),
            (key("Esc"), t_to("tui.hint.exit", &locale)),
        ],
    };

    // 构建提示条内容：快捷键已由 key() 函数添加 [ ] 括号
    let mut spans: Vec<Span> = Vec::new();
    for (i, (key, desc)) in hints.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default().fg(Color::White)));
        }
        spans.push(Span::styled(
            key.clone(), // key 已包含 [ ] 括号
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(
            format!(" {desc}"),
            Style::default().fg(Color::White),
        ));
    }

    let bar =
        Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::Black).fg(Color::White));
    frame.render_widget(bar, area);
}

// ── 编辑覆盖层 ───────────────────────────────────────────────────────────────

fn render_edit_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let AppMode::Edit {
        path,
        value_type,
        buffer,
        cursor_pos,
        detected_type,
        parse_error,
    } = &app.mode
    else {
        return;
    };

    // 覆盖层位置：底部 3 行
    let overlay_height = 3u16;
    if area.height < overlay_height + 2 {
        return;
    }
    let overlay_area = Rect {
        x: area.x + 1,
        y: area.y + area.height - overlay_height - 1,
        width: area.width.saturating_sub(2),
        height: overlay_height,
    };

    frame.render_widget(Clear, overlay_area);

    let locale = get_locale();
    let display_buf = format!("{buffer} ");

    // 根据检测结果决定边框颜色
    let (border_color, type_info) = if parse_error.is_some() {
        // 有解析错误，显示为字符串
        (Color::DarkGray, t_to("tui.status.string_as_str", &locale))
    } else if let Some(detected) = detected_type {
        if detected == "empty" {
            (Color::Yellow, "empty".to_string())
        } else if detected == value_type {
            // 类型匹配
            (Color::Green, format!("✓ {detected}"))
        } else {
            // 类型不匹配
            (
                Color::Yellow,
                t_to("tui.overlay.type_mismatch", &locale)
                    .replace("{0}", detected.as_str())
                    .replace("{1}", value_type.as_str()),
            )
        }
    } else {
        (Color::Yellow, value_type.clone())
    };

    let title = format!(
        " {} {} - {} [{}] ",
        t_to("tui.overlay.edit", &locale),
        value_type,
        path,
        type_info,
    );

    let para = Paragraph::new(display_buf)
        .block(
            Block::default()
                .title(Span::styled(title, Style::default().fg(border_color)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(para, overlay_area);

    // 设置光标位置（+1 是边框偏移）
    let cursor_x = overlay_area.x + 1 + (*cursor_pos as u16).min(overlay_area.width - 3);
    let cursor_y = overlay_area.y + 1;
    frame.set_cursor_position((cursor_x, cursor_y));
}

fn render_edit_key_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let AppMode::EditKey {
        path,
        buffer,
        cursor_pos,
        ..
    } = &app.mode
    else {
        return;
    };

    // 覆盖层位置：底部 3 行
    let overlay_height = 3u16;
    if area.height < overlay_height + 2 {
        return;
    }
    let overlay_area = Rect {
        x: area.x + 1,
        y: area.y + area.height - overlay_height - 1,
        width: area.width.saturating_sub(2),
        height: overlay_height,
    };

    frame.render_widget(Clear, overlay_area);

    let locale = get_locale();
    let display_buf = format!("{buffer} ");
    let title = format!(" {} {} ", t_to("tui.overlay.rename_key", &locale), path,);

    let para = Paragraph::new(display_buf)
        .block(
            Block::default()
                .title(Span::styled(title, Style::default().fg(Color::Cyan)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(para, overlay_area);

    // 设置光标位置（+1 是边框偏移）
    let cursor_x = overlay_area.x + 1 + (*cursor_pos as u16).min(overlay_area.width - 3);
    let cursor_y = overlay_area.y + 1;
    frame.set_cursor_position((cursor_x, cursor_y));
}

// ── 搜索覆盖层 ─────────────────────────────────────────────────────────────

fn render_search_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let AppMode::Search { query, cursor_pos } = &app.mode else {
        return;
    };

    let overlay_height = 3u16;
    if area.height < overlay_height + 2 {
        return;
    }
    let overlay_area = Rect {
        x: area.x + 1,
        y: area.y + area.height - overlay_height - 1,
        width: area.width.saturating_sub(2),
        height: overlay_height,
    };

    frame.render_widget(Clear, overlay_area);

    let locale = get_locale();
    let display_buf = format!("/ {query} ");
    let title = t_to("tui.overlay.search", &locale);

    let para = Paragraph::new(display_buf)
        .block(
            Block::default()
                .title(Span::styled(title, Style::default().fg(Color::Cyan)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(para, overlay_area);

    // 光标位置（+3 是因为前面有 "/ "）
    let cursor_x = overlay_area.x + 1 + 2 + (*cursor_pos as u16).min(overlay_area.width - 5);
    let cursor_y = overlay_area.y + 1;
    frame.set_cursor_position((cursor_x, cursor_y));
}

// ── 添加节点覆盖层 ─────────────────────────────────────────────────────────

fn render_add_node_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let AppMode::AddNode {
        parent_path,
        is_array: _,
        key_buffer,
        key_cursor,
        selecting_type,
        type_selected,
    } = &app.mode
    else {
        return;
    };

    let locale = get_locale();

    // 阶段1：输入 key
    if !*selecting_type {
        let overlay_height = 3u16;
        if area.height < overlay_height + 2 {
            return;
        }
        let overlay_area = Rect {
            x: area.x + 1,
            y: area.y + area.height - overlay_height - 1,
            width: area.width.saturating_sub(2),
            height: overlay_height,
        };

        frame.render_widget(Clear, overlay_area);

        let display_buf = format!(" {key_buffer} ");
        let title = format!(
            " {} {} ",
            t_to("tui.overlay.add_field", &locale),
            parent_path,
        );

        let para = Paragraph::new(display_buf)
            .block(
                Block::default()
                    .title(Span::styled(title, Style::default().fg(Color::Green)))
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green)),
            )
            .style(Style::default().fg(Color::White));

        frame.render_widget(para, overlay_area);

        // 光标位置
        let cursor_x = overlay_area.x + 1 + (*key_cursor as u16).min(overlay_area.width - 3);
        let cursor_y = overlay_area.y + 1;
        frame.set_cursor_position((cursor_x, cursor_y));
        return;
    }

    // 阶段2：类型选择
    let overlay_height = 6u16;
    if area.height < overlay_height + 2 {
        return;
    }
    let overlay_width = 36u16;
    let overlay_area = Rect {
        x: area.x + (area.width - overlay_width) / 2,
        y: area.y + (area.height - overlay_height) / 2,
        width: overlay_width,
        height: overlay_height,
    };

    frame.render_widget(Clear, overlay_area);

    // 类型选项
    #[allow(clippy::useless_vec)]
    let type_options = vec![("null", "null (默认)"), ("{}", "空对象"), ("[]", "空数组")];

    let title = format!(" {} ", t_to("tui.overlay.select_type", &locale));

    let block = Block::default()
        .title(Span::styled(title, Style::default().fg(Color::Green)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let mut lines = Vec::new();
    for (i, (symbol, label)) in type_options.iter().enumerate() {
        let is_selected = i == *type_selected;
        let prefix = if is_selected { "▶ " } else { "  " };
        let style = if is_selected {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let locale_label = if *label == "null (默认)" {
            t_to("tui.overlay.type_null", &locale)
        } else if *label == "空对象" {
            t_to("tui.overlay.type_object", &locale)
        } else {
            t_to("tui.overlay.type_array", &locale)
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{prefix}[{symbol}]"), style),
            Span::styled(
                format!(" {locale_label}"),
                Style::default().fg(Color::White),
            ),
        ]));
    }

    let para = Paragraph::new(lines)
        .block(block)
        .style(Style::default().fg(Color::White));

    frame.render_widget(para, overlay_area);
}

// ── 退出确认覆盖层 ───────────────────────────────────────────────────────────

fn render_confirm_quit_overlay(frame: &mut Frame, area: Rect) {
    let locale = get_locale();
    let overlay_height = 7u16;
    let overlay_width = 48u16;

    if area.height < overlay_height + 2 || area.width < overlay_width + 2 {
        return;
    }

    let overlay_area = Rect {
        x: area.x + (area.width - overlay_width) / 2,
        y: area.y + (area.height - overlay_height) / 2,
        width: overlay_width,
        height: overlay_height,
    };

    frame.render_widget(Clear, overlay_area);

    // i18n 文本
    let title = t_to("tui.overlay.confirm_quit", &locale);
    let file_modified = t_to("tui.overlay.file_modified", &locale);
    let save_quit = t_to("tui.overlay.save_and_quit", &locale);
    let quit_no_save = t_to("tui.overlay.quit_no_save", &locale);
    let cancel = t_to("tui.overlay.cancel", &locale);

    let msg = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {file_modified}"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " [ Y ] ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("{save_quit}   "), Style::default().fg(Color::White)),
            Span::styled(
                " [ N ] ",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("{quit_no_save}   "),
                Style::default().fg(Color::White),
            ),
            Span::styled(
                " [ C ] ",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(cancel, Style::default().fg(Color::White)),
        ]),
    ];

    let para = Paragraph::new(msg)
        .block(
            Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .style(Style::default().fg(Color::White));

    // 保存对话框区域供鼠标处理使用
    frame.render_widget(para, overlay_area);
}

// ── 帮助面板覆盖层 ───────────────────────────────────────────────────────────

/// 固定的快捷键列宽度（字符数），用于表格对齐
const KEY_COL_WIDTH: usize = 14;

fn render_help_panel(frame: &mut Frame, area: Rect) {
    let locale = get_locale();

    // i18n 文本
    let title = t_to("tui.help.title", &locale);
    let help_title = t_to("tui.help.help_title", &locale);
    let nav = t_to("tui.help.nav", &locale);
    let edit = t_to("tui.help.edit", &locale);
    let file = t_to("tui.help.file", &locale);
    let close_help = t_to("tui.help.close_help", &locale);

    let save = t_to("tui.help.save", &locale);
    let undo = t_to("tui.help.undo", &locale);
    let redo = t_to("tui.help.redo", &locale);
    let quit = t_to("tui.help.quit", &locale);

    let move_up_down = t_to("tui.help.move_up_down", &locale);
    let collapse_expand = t_to("tui.help.collapse_expand", &locale);
    let toggle_expand = t_to("tui.help.toggle_expand", &locale);
    let quick_scroll = t_to("tui.help.quick_scroll", &locale);
    let jump_begin_end = t_to("tui.help.jump_begin_end", &locale);
    let edit_value = t_to("tui.help.edit_value", &locale);
    let new_node = t_to("tui.help.new_node", &locale);
    let delete_node = t_to("tui.help.delete_node", &locale);
    let toggle_bool = t_to("tui.help.toggle_bool", &locale);
    let search = t_to("tui.help.search", &locale);
    let menu = t_to("tui.hint.menu", &locale);

    let overlay_width = 52u16;
    let overlay_height = 24u16;

    if area.height < overlay_height + 2 || area.width < overlay_width + 2 {
        return;
    }

    let overlay_area = Rect {
        x: area.x + (area.width - overlay_width) / 2,
        y: area.y + (area.height - overlay_height) / 2,
        width: overlay_width,
        height: overlay_height,
    };

    frame.render_widget(Clear, overlay_area);

    // 获取修饰键文本
    let ctrl = if cfg!(target_os = "macos") {
        "⌘"
    } else {
        "Ctrl"
    };

    // 辅助函数：创建带颜色的快捷键 span
    let key_span = |k: &str| -> Span<'static> {
        Span::styled(
            format!("[{k}]"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    };

    // 辅助函数：创建组合键 span
    let combo_span = |c: &str, k: &str| -> Span<'static> {
        Span::styled(
            format!("[{c}]+[{}]", k.to_uppercase()),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
    };

    // 辅助函数：创建表格行，固定快捷键列宽度
    let help_row = |key: Span<'static>, desc: String| -> Line<'static> {
        let key_text = extract_text(&key);
        let padding = KEY_COL_WIDTH.saturating_sub(key_text.len());
        Line::from(vec![
            Span::raw("  "),
            key,
            Span::raw(" ".repeat(padding)),
            Span::styled(desc, Style::default().fg(Color::White)),
        ])
    };

    // 辅助函数：创建普通快捷键行
    let key_row = |k: &str, desc: &str| -> Line<'static> {
        help_row(key_span(k), desc.to_string())
    };

    // 辅助函数：创建组合键行
    let combo_row = |c: &str, k: &str, desc: &str| -> Line<'static> {
        help_row(combo_span(c, k), desc.to_string())
    };

    let help_content: Vec<Line> = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!("  {help_title}"),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
                .add_modifier(Modifier::UNDERLINED),
        )),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {nav}"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        key_row("↑/↓", &move_up_down),
        key_row("←/→", &collapse_expand),
        key_row("Space", &toggle_expand),
        key_row("PgUp/PgDn", &quick_scroll),
        key_row("Home/End", &jump_begin_end),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {edit}"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        key_row("Enter", &edit_value),
        key_row("N", &new_node),
        key_row("Del", &delete_node),
        key_row("Tab", &toggle_bool),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {file}"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        key_row("/", &search),
        key_row("F2", &menu),
        combo_row(ctrl, "S", &save),
        combo_row(ctrl, "Z", &undo),
        combo_row(ctrl, "Y", &redo),
        combo_row(ctrl, "Q", &quit),
        Line::from(""),
        Line::from(Span::styled(
            close_help,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
    ];

    let para = Paragraph::new(help_content)
        .block(
            Block::default()
                .title(format!(" {title} "))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .style(Style::default().fg(Color::White))
        .wrap(Wrap { trim: true });

    frame.render_widget(para, overlay_area);
}

/// 从 Span 中提取文本内容（用于计算宽度）
fn extract_text(span: &Span<'_>) -> String {
    span.content.clone().into_owned()
}

// ── 保存预览覆盖层 ───────────────────────────────────────────────────────────

fn render_save_preview(frame: &mut Frame, app: &App, area: Rect) {
    let AppMode::ConfirmSave { original_content } = &app.mode else {
        return;
    };

    let locale = get_locale();
    let new_content = app.get_new_content();

    // 计算 diff 统计
    let old_lines = original_content.lines().count();
    let new_lines = new_content.lines().count();
    #[allow(clippy::cast_possible_wrap)]
    let line_diff = new_lines as i64 - old_lines as i64;
    let lines_text = t_to("tui.status.lines", &locale);
    let diff_info = match line_diff.cmp(&0) {
        std::cmp::Ordering::Greater => format!("+{line_diff}{lines_text}"),
        std::cmp::Ordering::Less => format!("{line_diff}{lines_text}"),
        std::cmp::Ordering::Equal => t_to("tui.status.no_changes", &locale),
    };

    // 覆盖层大小
    let overlay_height = 10u16;
    let overlay_width = 60u16;
    if area.height < overlay_height + 2 || area.width < overlay_width + 2 {
        return;
    }
    let overlay_area = Rect {
        x: area.x + (area.width - overlay_width) / 2,
        y: area.y + (area.height - overlay_height) / 2,
        width: overlay_width,
        height: overlay_height,
    };

    frame.render_widget(Clear, overlay_area);

    // 构建消息
    let file_name = app.file_path.file_name().map_or_else(
        || "unknown".to_string(),
        |s| s.to_string_lossy().to_string(),
    );

    let save_confirm = t_to("tui.status.save_confirm", &locale);
    let change = t_to("tui.status.change", &locale);
    let old_label = t_to("tui.status.old_lines", &locale);
    let new_label = t_to("tui.status.new_lines", &locale);
    let save_hint = t_to("tui.overlay.save_hint", &locale);

    let msg = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("  {save_confirm}"),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(&file_name, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(format!("  {change}"), Style::default().fg(Color::DarkGray)),
            Span::styled(
                &diff_info,
                Style::default().fg(match line_diff.cmp(&0) {
                    std::cmp::Ordering::Greater => Color::Green,
                    std::cmp::Ordering::Less => Color::Red,
                    std::cmp::Ordering::Equal => Color::DarkGray,
                }),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                format!("  {old_label}"),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("{old_lines}{lines_text}"),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("  {new_label}"),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(
                format!("{new_lines}{lines_text}"),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            format!("  {save_hint}"),
            Style::default().fg(Color::Yellow),
        )),
    ];

    let title = t_to("tui.status.save_preview", &locale);
    let para = Paragraph::new(msg).block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );

    frame.render_widget(para, overlay_area);
}

// ── 右键菜单覆盖层 ───────────────────────────────────────────────────────────

fn render_context_menu(frame: &mut Frame, app: &App, area: Rect) {
    let AppMode::ContextMenu {
        selected,
        mouse_x,
        mouse_y,
        ..
    } = &app.mode
    else {
        return;
    };

    let locale = get_locale();
    let actions = ContextAction::all();
    // 菜单宽度增加以容纳快捷键提示
    let menu_width = 34u16;
    let menu_height = actions.len() as u16 + 2;

    // 菜单位置：鼠标点击位置（减去一些偏移让菜单在点击位置下方/旁边）
    let menu_x = (*mouse_x)
        .saturating_sub(2)
        .min(area.width.saturating_sub(menu_width + 2));
    let menu_y = (*mouse_y).min(area.height.saturating_sub(menu_height + 2));

    if area.height < menu_y + menu_height + 2 || area.width < menu_x + menu_width + 2 {
        return;
    }

    let overlay_area = Rect {
        x: area.x + menu_x,
        y: area.y + menu_y,
        width: menu_width,
        height: menu_height,
    };

    // 清除区域并填充不透明黑色背景
    frame.render_widget(Clear, overlay_area);
    let bg = Paragraph::new(" ".repeat(overlay_area.width as usize))
        .style(Style::default().bg(Color::Black));
    // 逐行渲染背景以确保完全覆盖
    for y in 0..overlay_area.height {
        let row_area = Rect {
            x: overlay_area.x,
            y: overlay_area.y + y,
            width: overlay_area.width,
            height: 1,
        };
        frame.render_widget(&bg, row_area);
    }

    // 悬停效果优先于键盘选中
    let hover_row = app.menu_hover_row;

    let items: Vec<ListItem> = actions
        .iter()
        .enumerate()
        .map(|(i, action)| {
            let is_hovered = hover_row == Some(i);
            let is_selected = hover_row.is_none() && i == *selected;

            let shortcut = action.shortcut();

            let style = if is_hovered {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else if is_selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            // 构建菜单项：快捷键用黄色突出，标签用白色
            let label = action.label();

            // 快捷键部分用黄色，标签部分用当前样式
            let spans = vec![
                Span::styled(
                    format!("[{shortcut}]"),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!(" {label}"), style),
            ];

            ListItem::new(Line::from(spans))
        })
        .collect();

    let actions_label = t_to("tui.confirm.actions", &locale);
    let menu = List::new(items)
        .block(
            Block::default()
                .title(actions_label)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue))
                .style(Style::default().bg(Color::Black)),
        )
        .style(Style::default().bg(Color::Black))
        .highlight_style(Style::default());

    frame.render_widget(menu, overlay_area);
}
