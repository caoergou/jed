use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use super::app::{App, AppMode, StatusLevel};
use super::tree::TreeLine;

/// 每帧的主渲染入口。
pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let lines = app.tree_lines();

    // 布局：树形主区域 + 底部状态栏
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    render_tree(frame, app, chunks[0], &lines);
    render_statusbar(frame, app, chunks[1], &lines);

    // 编辑覆盖层
    if matches!(app.mode, AppMode::Edit { .. }) {
        render_edit_overlay(frame, app, area);
    }

    // 确认剥离注释覆盖层
    if matches!(app.mode, AppMode::ConfirmStripComments) {
        render_confirm_overlay(frame, area);
    }
}

// ── 树形视图 ─────────────────────────────────────────────────────────────────

fn render_tree(frame: &mut Frame, app: &mut App, area: Rect, lines: &[TreeLine]) {
    let modified_marker = if app.modified { " [*]" } else { "" };
    let title = format!(
        " je: {}{modified_marker} ",
        app.file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
    );

    let items: Vec<ListItem> = lines.iter().map(make_list_item).collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(Span::styled(title, Style::default().add_modifier(Modifier::BOLD)))
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

fn make_list_item(line: &TreeLine) -> ListItem<'_> {
    let indent = "  ".repeat(line.depth);

    // 展开/折叠指示符
    let indicator = if line.path.starts_with("__close__") {
        "  "
    } else if line.has_children {
        if line.is_expanded { "▼ " } else { "▶ " }
    } else {
        "  "
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
        "closing" => Color::White,
        _ => Color::White, // object / array 的开括号
    };

    let value_span = Span::styled(
        line.value_preview.clone(),
        Style::default().fg(value_color),
    );

    ListItem::new(Line::from(vec![
        Span::raw(format!("{indent}{indicator}")),
        key_span,
        value_span,
    ]))
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
        Line::from(vec![
            Span::styled(format!(" {path} "), Style::default().fg(Color::DarkGray)),
            Span::styled("│", Style::default().fg(Color::DarkGray)),
            Span::styled(format!(" {msg} "), Style::default().fg(color)),
        ])
    } else {
        let hints = match &app.mode {
            AppMode::Normal => " j/k:移动  h/l:折叠/展开  e:编辑  d:删除  u:撤销  ctrl+s:保存  q:退出",
            AppMode::Edit { .. } => " Enter:确认  Esc:取消",
            AppMode::ConfirmStripComments => " y:确认剥离注释并保存  n:取消",
        };
        Line::from(vec![
            Span::styled(format!(" {path} "), Style::default().fg(Color::DarkGray)),
            Span::styled("│", Style::default().fg(Color::DarkGray)),
            Span::styled(hints, Style::default().fg(Color::DarkGray)),
        ])
    };

    let bar = Paragraph::new(status_text)
        .style(Style::default().bg(Color::Black));
    frame.render_widget(bar, area);
}

// ── 编辑覆盖层 ───────────────────────────────────────────────────────────────

fn render_edit_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let AppMode::Edit {
        path,
        buffer,
        cursor_pos,
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

    let display_buf = format!("{buffer} ");
    let title = format!(" 编辑 {path} ");

    let para = Paragraph::new(display_buf)
        .block(
            Block::default()
                .title(Span::styled(title, Style::default().fg(Color::Yellow)))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        )
        .style(Style::default().fg(Color::White));

    frame.render_widget(para, overlay_area);

    // 设置光标位置（+1 是边框偏移）
    let cursor_x = overlay_area.x + 1 + (*cursor_pos as u16).min(overlay_area.width - 3);
    let cursor_y = overlay_area.y + 1;
    frame.set_cursor_position((cursor_x, cursor_y));
}

// ── 确认覆盖层 ───────────────────────────────────────────────────────────────

fn render_confirm_overlay(frame: &mut Frame, area: Rect) {
    let overlay_height = 5u16;
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

    let msg = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  此文件含有注释（JSONC 格式）。",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            "  保存后注释将被移除，是否继续？",
            Style::default().fg(Color::Yellow),
        )),
        Line::from(Span::styled(
            "  y 确认  /  n 取消",
            Style::default().fg(Color::White),
        )),
    ];

    let para = Paragraph::new(msg)
        .block(
            Block::default()
                .title(" 注意 ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        );

    frame.render_widget(para, overlay_area);
}
