#![allow(clippy::unnested_or_patterns, clippy::too_many_lines)]
#![allow(
    clippy::cast_lossless,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap
)]

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEventKind};
use std::time::{Duration, Instant};

use super::app::{App, AppMode, ContextAction};

// 双击时间间隔（毫秒）
const DOUBLE_CLICK_MS: u64 = 500;

/// 处理终端事件，更新 App 状态。
pub fn handle_event(app: &mut App, event: &Event) {
    match event {
        Event::Key(key) => handle_key(app, *key),
        Event::Mouse(mouse) => handle_mouse(app, *mouse),
        _ => {}
    }
}

fn handle_key(app: &mut App, key: KeyEvent) {
    match &app.mode.clone() {
        AppMode::Normal => handle_normal(app, key),
        AppMode::Edit { .. } => handle_edit(app, key),
        AppMode::EditKey { .. } => handle_edit_key(app, key),
        AppMode::Help => handle_help(app, key),
        AppMode::ConfirmQuit { .. } => handle_confirm_quit(app, key),
        AppMode::ConfirmSave { .. } => handle_confirm_save(app, key),
        AppMode::Search { .. } => handle_search(app, key),
        AppMode::AddNode { .. } => handle_add_node(app, key),
        AppMode::ContextMenu { .. } => handle_context_menu(app, key),
    }
}

// ── 普通模式 ─────────────────────────────────────────────────────────────────
// 新交互方案：纯方向键，去掉 vim 风格

fn handle_normal(app: &mut App, key: KeyEvent) {
    // 先清除上次状态消息
    app.status = None;

    match (key.code, key.modifiers) {
        // 导航：只用方向键
        (KeyCode::Up, _) => app.move_up(),
        (KeyCode::Down, _) => app.move_down(),
        (KeyCode::Left, _) => app.collapse_or_go_parent(),
        (KeyCode::Right, _) => app.expand_or_enter(),

        // PageUp/PageDown: 快速滚动（大幅移动）
        (KeyCode::PageUp, _) => {
            let scroll_amount = 10;
            app.cursor = app.cursor.saturating_sub(scroll_amount);
            app.list_state.select(Some(app.cursor));
        }
        (KeyCode::PageDown, _) => {
            let scroll_amount = 10;
            let lines = app.tree_lines();
            app.cursor = (app.cursor + scroll_amount).min(lines.len().saturating_sub(1));
            app.list_state.select(Some(app.cursor));
        }

        // Home/End: 跳到首尾
        (KeyCode::Home, _) => {
            app.cursor = 0;
            app.list_state.select(Some(0));
        }
        (KeyCode::End, _) => {
            let lines = app.tree_lines();
            if !lines.is_empty() {
                app.cursor = lines.len() - 1;
                app.list_state.select(Some(app.cursor));
            }
        }

        // Space 切换展开/折叠
        (KeyCode::Char(' '), _) => app.expand_or_toggle(),

        // Enter: 叶子节点进入编辑，容器节点展开
        (KeyCode::Enter, _) => {
            let lines = app.tree_lines();
            if let Some(line) = lines.get(app.cursor) {
                if line.has_children && !line.is_expanded {
                    app.expanded.insert(line.path.clone());
                } else if !line.has_children {
                    app.start_edit();
                }
            }
        }

        // 删除
        (KeyCode::Delete, _) => app.delete_current(),

        // 新建节点
        (KeyCode::Insert, _) | (KeyCode::Char('n'), _) => app.start_add_node(),

        // 撤销 / 重做
        (KeyCode::Char('z'), KeyModifiers::CONTROL) => app.undo(),
        (KeyCode::Char('y'), KeyModifiers::CONTROL) => app.redo(),

        // 保存
        (KeyCode::Char('s'), KeyModifiers::CONTROL) => app.try_save(),

        // 搜索
        (KeyCode::Char('f'), KeyModifiers::CONTROL) | (KeyCode::Char('/'), _) => app.start_search(),

        // 右键菜单
        (KeyCode::F(2), _) => {
            app.show_context_menu(5, (app.cursor as u16) + 1);
        }

        // 帮助面板
        (KeyCode::F(1), _) => {
            if matches!(app.mode, AppMode::Help) {
                app.mode = AppMode::Normal;
            } else {
                app.mode = AppMode::Help;
            }
        }

        // 退出：Esc 两次，或未修改时直接 Esc
        (KeyCode::Esc, _) => {
            if app.modified {
                // 已修改：检查是否是连续按两次 Esc
                let now = Instant::now();
                let is_double_escape = app
                    .last_escape_time
                    .is_some_and(|last| now.duration_since(last) < Duration::from_millis(500));

                if is_double_escape {
                    // 连续按两次，强制退出
                    app.should_quit = true;
                } else {
                    // 第一次按，或超时了，显示确认对话框
                    app.mode = AppMode::ConfirmQuit {
                        last_was_escape: true,
                    };
                    app.last_escape_time = Some(now);
                }
            } else {
                // 未修改时直接退出
                app.should_quit = true;
            }
        }

        // Ctrl+Q 仍然支持，作为退出的快捷方式
        (KeyCode::Char('q'), KeyModifiers::CONTROL) => {
            if app.modified {
                app.mode = AppMode::ConfirmQuit {
                    last_was_escape: false,
                };
            } else {
                app.should_quit = true;
            }
        }

        _ => {}
    }
}

// ── 编辑模式 ─────────────────────────────────────────────────────────────────

fn handle_edit(app: &mut App, key: KeyEvent) {
    let AppMode::Edit {
        buffer,
        cursor_pos,
        value_type,
        ..
    } = &mut app.mode
    else {
        return;
    };

    match key.code {
        KeyCode::Enter => {
            app.confirm_edit();
        }
        KeyCode::Esc => {
            app.cancel_edit();
        }
        // Tab 切换布尔值
        KeyCode::Tab => {
            if *value_type == "boolean" {
                let current = buffer.trim();
                *buffer = if current == "true" {
                    "false".to_string()
                } else {
                    "true".to_string()
                };
                *cursor_pos = buffer.len();
                app.update_edit_validation();
            }
        }
        KeyCode::Char(c) => {
            buffer.insert(*cursor_pos, c);
            *cursor_pos += c.len_utf8();
            app.update_edit_validation();
        }
        KeyCode::Backspace => {
            if *cursor_pos > 0 {
                let prev = prev_char_boundary(buffer, *cursor_pos);
                buffer.drain(prev..*cursor_pos);
                *cursor_pos = prev;
                app.update_edit_validation();
            }
        }
        KeyCode::Delete => {
            if *cursor_pos < buffer.len() {
                let next = next_char_boundary(buffer, *cursor_pos);
                buffer.drain(*cursor_pos..next);
                app.update_edit_validation();
            }
        }
        KeyCode::Left => {
            if *cursor_pos > 0 {
                *cursor_pos = prev_char_boundary(buffer, *cursor_pos);
            }
        }
        KeyCode::Right => {
            if *cursor_pos < buffer.len() {
                *cursor_pos = next_char_boundary(buffer, *cursor_pos);
            }
        }
        KeyCode::Home => {
            *cursor_pos = 0;
        }
        KeyCode::End => {
            *cursor_pos = buffer.len();
        }
        _ => {}
    }
}

fn handle_edit_key(app: &mut App, key: KeyEvent) {
    let AppMode::EditKey {
        buffer, cursor_pos, ..
    } = &mut app.mode
    else {
        return;
    };

    match key.code {
        KeyCode::Enter => {
            app.confirm_edit_key();
        }
        KeyCode::Esc => {
            app.cancel_edit();
        }
        KeyCode::Char(c) => {
            buffer.insert(*cursor_pos, c);
            *cursor_pos += c.len_utf8();
        }
        KeyCode::Backspace => {
            if *cursor_pos > 0 {
                let prev = prev_char_boundary(buffer, *cursor_pos);
                buffer.drain(prev..*cursor_pos);
                *cursor_pos = prev;
            }
        }
        KeyCode::Delete => {
            if *cursor_pos < buffer.len() {
                let next = next_char_boundary(buffer, *cursor_pos);
                buffer.drain(*cursor_pos..next);
            }
        }
        KeyCode::Left => {
            if *cursor_pos > 0 {
                *cursor_pos = prev_char_boundary(buffer, *cursor_pos);
            }
        }
        KeyCode::Right => {
            if *cursor_pos < buffer.len() {
                *cursor_pos = next_char_boundary(buffer, *cursor_pos);
            }
        }
        KeyCode::Home => {
            *cursor_pos = 0;
        }
        KeyCode::End => {
            *cursor_pos = buffer.len();
        }
        _ => {}
    }
}

// ── 帮助模式 ─────────────────────────────────────────────────────────────────

fn handle_help(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc | KeyCode::Enter | KeyCode::F(1) => {
            app.mode = AppMode::Normal;
        }
        _ => {}
    }
}

// ── 退出确认模式 ─────────────────────────────────────────────────────────────

fn handle_confirm_quit(app: &mut App, key: KeyEvent) {
    match key.code {
        // Y: 保存并退出
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            app.do_save();
            app.should_quit = true;
        }
        // N: 不保存直接退出
        KeyCode::Char('n') | KeyCode::Char('N') => {
            app.should_quit = true;
        }
        // C / Esc: 取消
        KeyCode::Char('c') | KeyCode::Char('C') | KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.last_escape_time = None;
        }
        _ => {}
    }
}

// ── 保存预览模式 ──────────────────────────────────────────────────────────────

fn handle_confirm_save(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.confirm_save();
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.cancel_save();
        }
        _ => {}
    }
}

// ── 搜索模式 ────────────────────────────────────────────────────────────────

fn handle_search(app: &mut App, key: KeyEvent) {
    let AppMode::Search { query, cursor_pos } = &mut app.mode else {
        return;
    };

    match key.code {
        KeyCode::Enter => {
            app.search_next();
        }
        KeyCode::Esc => {
            app.cancel_search();
        }
        KeyCode::Char(c) => {
            query.insert(*cursor_pos, c);
            *cursor_pos += c.len_utf8();
        }
        KeyCode::Backspace => {
            if *cursor_pos > 0 {
                let prev = prev_char_boundary(query, *cursor_pos);
                query.drain(prev..*cursor_pos);
                *cursor_pos = prev;
            }
        }
        KeyCode::Delete => {
            if *cursor_pos < query.len() {
                let next = next_char_boundary(query, *cursor_pos);
                query.drain(*cursor_pos..next);
            }
        }
        KeyCode::Left => {
            if *cursor_pos > 0 {
                *cursor_pos = prev_char_boundary(query, *cursor_pos);
            }
        }
        KeyCode::Right => {
            if *cursor_pos < query.len() {
                *cursor_pos = next_char_boundary(query, *cursor_pos);
            }
        }
        KeyCode::Home => {
            *cursor_pos = 0;
        }
        KeyCode::End => {
            *cursor_pos = query.len();
        }
        _ => {}
    }
}

// ── 添加节点模式 ─────────────────────────────────────────────────────────────

fn handle_add_node(app: &mut App, key: KeyEvent) {
    let AppMode::AddNode {
        parent_path: _,
        is_array: _,
        key_buffer,
        key_cursor,
    } = &mut app.mode
    else {
        return;
    };

    match key.code {
        KeyCode::Enter => {
            app.confirm_add_node();
        }
        KeyCode::Esc => {
            app.cancel_add_node();
        }
        KeyCode::Char(c) => {
            key_buffer.insert(*key_cursor, c);
            *key_cursor += c.len_utf8();
        }
        KeyCode::Backspace => {
            if *key_cursor > 0 {
                let prev = prev_char_boundary(key_buffer, *key_cursor);
                key_buffer.drain(prev..*key_cursor);
                *key_cursor = prev;
            }
        }
        KeyCode::Delete => {
            if *key_cursor < key_buffer.len() {
                let next = next_char_boundary(key_buffer, *key_cursor);
                key_buffer.drain(*key_cursor..next);
            }
        }
        KeyCode::Left => {
            if *key_cursor > 0 {
                *key_cursor = prev_char_boundary(key_buffer, *key_cursor);
            }
        }
        KeyCode::Right => {
            if *key_cursor < key_buffer.len() {
                *key_cursor = next_char_boundary(key_buffer, *key_cursor);
            }
        }
        KeyCode::Home => {
            *key_cursor = 0;
        }
        KeyCode::End => {
            *key_cursor = key_buffer.len();
        }
        _ => {}
    }
}

// ── 右键菜单模式 ───────────────────────────────────────────────────────────

fn handle_context_menu(app: &mut App, key: KeyEvent) {
    let AppMode::ContextMenu { selected, .. } = &mut app.mode else {
        return;
    };

    let actions = ContextAction::all();
    let max = actions.len();

    match key.code {
        KeyCode::Esc => {
            app.close_context_menu();
        }
        KeyCode::Enter => {
            let action = actions[*selected];
            app.execute_context_action(action);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if *selected > 0 {
                *selected -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if *selected + 1 < max {
                *selected += 1;
            }
        }
        // 快捷键直接执行
        KeyCode::Char('e') => app.execute_context_action(ContextAction::Edit),
        KeyCode::Char('a') => app.execute_context_action(ContextAction::AddChild),
        KeyCode::Char('d') => app.execute_context_action(ContextAction::Delete),
        KeyCode::Char('c') => app.execute_context_action(ContextAction::CopyKey),
        KeyCode::Char('v') => app.execute_context_action(ContextAction::CopyValue),
        KeyCode::Char('p') => app.execute_context_action(ContextAction::CopyPath),
        KeyCode::Char('*') => app.execute_context_action(ContextAction::ExpandAll),
        KeyCode::Char('-') => app.execute_context_action(ContextAction::CollapseAll),
        _ => {}
    }
}

// ── 鼠标处理 ─────────────────────────────────────────────────────────────────

fn handle_mouse(app: &mut App, event: crossterm::event::MouseEvent) {
    // 退出确认对话框的鼠标点击
    if let AppMode::ConfirmQuit { .. } = &app.mode {
        if event.kind == crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left)
        {
            // 对话框位置在屏幕中央，宽度48，高度7
            // 简化处理：根据列判断点击了哪个按钮
            // [Y] 在列 10-20, [N] 在列 22-32, [C] 在列 36-44
            if event.row > 0 && event.row < 20 {
                let col = event.column as i32;
                if (10..20).contains(&col) {
                    // [Y] 保存并退出
                    app.do_save();
                    app.should_quit = true;
                } else if (22..32).contains(&col) {
                    // [N] 不保存退出
                    app.should_quit = true;
                } else if (36..44).contains(&col) {
                    // [C] 取消
                    app.mode = AppMode::Normal;
                }
            }
        }
        return;
    }

    // 保存预览对话框的鼠标点击
    if let AppMode::ConfirmSave { .. } = &app.mode {
        if event.kind == crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left)
        {
            // 保存预览对话框位置在屏幕中央，宽度60，高度10
            // 提示行内容: "  [ Enter / Y ] Save  [ Esc / N ] Cancel"
            // 简化处理：[Y/Save] 在列 13-22, [N/Cancel] 在列 26-35
            if event.row > 0 && event.row < 20 {
                let col = event.column as i32;
                if (13..22).contains(&col) {
                    // [Y/Enter] 保存
                    app.confirm_save();
                } else if (26..35).contains(&col) {
                    // [N/Esc] 取消
                    app.cancel_save();
                }
            }
        }
        return;
    }

    // 如果在右键菜单模式下，处理菜单内的鼠标移动和点击
    if let AppMode::ContextMenu {
        mouse_x, mouse_y, ..
    } = &app.mode
    {
        let actions = ContextAction::all();
        let menu_width = 28i32;
        let menu_height = actions.len() as i32 + 2;

        // 先保存坐标，避免重复借用
        let saved_mouse_x = i32::from(*mouse_x);
        let saved_mouse_y = i32::from(*mouse_y);

        let menu_x = saved_mouse_x - 2;
        let menu_y = saved_mouse_y;

        let click_x = i32::from(event.column);
        let click_y = i32::from(event.row);

        // 检查鼠标是否在菜单区域内，更新悬停状态
        let in_menu_area = click_x >= menu_x
            && click_x < menu_x + menu_width
            && click_y >= menu_y
            && click_y < menu_y + menu_height;

        if in_menu_area {
            let diff = click_y - menu_y - 1;
            let item_index = if diff < 0 {
                0
            } else {
                usize::try_from(diff).unwrap_or(0)
            };
            if item_index < actions.len() {
                app.menu_hover_row = Some(item_index);
            }
        } else {
            app.menu_hover_row = None;
            // 鼠标移出菜单区域时关闭菜单
            app.close_context_menu();
        }

        // 处理左键点击
        if event.kind == crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left)
        {
            if in_menu_area {
                // 计算点击了哪一项（减去标题行）
                let diff = click_y - menu_y - 1;
                let item_index = if diff < 0 {
                    0
                } else {
                    usize::try_from(diff).unwrap_or(0)
                };
                if item_index < actions.len() {
                    let action = actions[item_index];
                    app.execute_context_action(action);
                    return;
                }
            }
            // 点击菜单外，关闭菜单
            app.close_context_menu();
            return;
        }
        // 右键点击菜单外也关闭
        if event.kind
            == crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Right)
        {
            app.close_context_menu();
            return;
        }
    }

    // 普通模式下的树形视图鼠标处理
    let lines = app.tree_lines();
    if lines.is_empty() {
        return;
    }

    // 计算点击的是哪一行（树从 y=1 开始，每行高 1）
    let row = event.row as usize;
    let item_row = row.saturating_sub(1);

    if item_row >= lines.len() {
        return;
    }

    // 处理右键点击：显示菜单
    if event.kind == crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Right) {
        app.cursor = item_row;
        app.list_state.select(Some(item_row));
        app.show_context_menu(event.column, event.row);
        return;
    }

    // 只处理左键单击和滚轮事件
    match event.kind {
        MouseEventKind::ScrollUp => {
            let scroll_amount = 5;
            app.cursor = app.cursor.saturating_sub(scroll_amount);
            app.list_state.select(Some(app.cursor));
            return;
        }
        MouseEventKind::ScrollDown => {
            let scroll_amount = 5;
            let lines = app.tree_lines();
            app.cursor = (app.cursor + scroll_amount).min(lines.len().saturating_sub(1));
            app.list_state.select(Some(app.cursor));
            return;
        }
        MouseEventKind::Down(MouseButton::Left) => {}
        _ => return,
    }

    // 计算点击的是哪一行（树从 y=1 开始，每行高 1）
    let row = event.row as usize;
    let item_row = row.saturating_sub(1);

    if item_row >= lines.len() {
        return;
    }

    let line = &lines[item_row];

    // 检测点击是否在展开/折叠区域（前几个字符）
    // 每行格式: [indent][indicator][key]: [value]
    // indent = 2空格 * depth, indicator = 2字符
    let toggle_width = 2 + line.depth * 2; // indicator区域宽度
    let click_col = event.column as usize;

    // 检测双击：同一行 + 快速点击
    let now = Instant::now();
    let is_double_click =
        app.last_click_time
            .zip(app.last_click_row)
            .is_some_and(|(time, prev_row)| {
                let elapsed =
                    u64::try_from(now.duration_since(time).as_millis()).unwrap_or(u64::MAX);
                prev_row == item_row && elapsed < DOUBLE_CLICK_MS
            });

    // 如果点击在展开/折叠区域，且节点有子节点，则切换展开/折叠
    if click_col < toggle_width && line.has_children && !is_double_click {
        app.cursor = item_row;
        app.list_state.select(Some(item_row));
        app.expand_or_toggle();
        app.last_click_time = None;
        app.last_click_row = None;
        return;
    }

    if is_double_click {
        // 双击：进入编辑模式
        app.cursor = item_row;
        app.list_state.select(Some(item_row));

        // 判断点击的是键还是值区域
        // 每行格式: [indent][indicator][key]: [value]
        let key_region_end = toggle_width + line.display_key.len() + 2; // +2 for ": "

        // 键区域不为空（数组索引不能编辑 key）
        let is_key_editable = !line.display_key.is_empty() && !line.display_key.starts_with('[');

        if click_col < key_region_end && is_key_editable {
            // 双击键：编辑键名
            app.start_edit_key();
        } else {
            // 双击值：编辑值
            app.start_edit();
        }

        // 重置双击状态
        app.last_click_time = None;
        app.last_click_row = None;
    } else {
        // 单击：选中节点
        app.cursor = item_row;
        app.list_state.select(Some(item_row));
        // 记录点击时间供下次双击检测
        app.last_click_time = Some(now);
        app.last_click_row = Some(item_row);
    }
}

// ── UTF-8 辅助 ───────────────────────────────────────────────────────────────

fn prev_char_boundary(s: &str, pos: usize) -> usize {
    let mut p = pos;
    while p > 0 {
        p -= 1;
        if s.is_char_boundary(p) {
            return p;
        }
    }
    0
}

fn next_char_boundary(s: &str, pos: usize) -> usize {
    let mut p = pos + 1;
    while p <= s.len() {
        if s.is_char_boundary(p) {
            return p;
        }
        p += 1;
    }
    s.len()
}
