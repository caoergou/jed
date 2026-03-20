use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use super::app::{App, AppMode, StatusLevel};

/// 处理终端事件，更新 App 状态。
pub fn handle_event(app: &mut App, event: Event) {
    match event {
        Event::Key(key) => handle_key(app, key),
        Event::Resize(_, _) => {} // ratatui 自动处理终端缩放
        _ => {}
    }
}

fn handle_key(app: &mut App, key: KeyEvent) {
    match &app.mode.clone() {
        AppMode::Normal => handle_normal(app, key),
        AppMode::Edit { .. } => handle_edit(app, key),
        AppMode::ConfirmStripComments => handle_confirm(app, key),
    }
}

// ── 普通模式 ─────────────────────────────────────────────────────────────────

fn handle_normal(app: &mut App, key: KeyEvent) {
    // 先清除上次状态消息
    app.status = None;

    match (key.code, key.modifiers) {
        // 移动
        (KeyCode::Char('j'), _) | (KeyCode::Down, _) => app.move_down(),
        (KeyCode::Char('k'), _) | (KeyCode::Up, _) => app.move_up(),

        // 展开 / 折叠
        (KeyCode::Char('l'), _) | (KeyCode::Right, _) | (KeyCode::Enter, _) => {
            app.expand_or_enter();
        }
        (KeyCode::Char('h'), _) | (KeyCode::Left, _) => {
            app.collapse_or_go_parent();
        }

        // 编辑
        (KeyCode::Char('e'), _) => app.start_edit(),

        // 删除
        (KeyCode::Char('d'), _) => app.delete_current(),

        // 撤销 / 重做
        (KeyCode::Char('u'), _) => app.undo(),
        (KeyCode::Char('r'), KeyModifiers::CONTROL) => app.redo(),

        // 保存
        (KeyCode::Char('s'), KeyModifiers::CONTROL) => app.try_save(),

        // 退出
        (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
            if app.modified {
                app.set_status(
                    "文件已修改未保存。再按 q 强制退出，或 ctrl+s 保存",
                    StatusLevel::Warn,
                );
                // 标记为 pending quit，下次再按 q 则真正退出
                // 通过检查 status 是否含 "强制" 来判断
            } else {
                app.should_quit = true;
            }
        }
        (KeyCode::Char('Q'), _) => {
            // 强制退出，不保存
            app.should_quit = true;
        }

        _ => {}
    }
}

// ── 编辑模式 ─────────────────────────────────────────────────────────────────

fn handle_edit(app: &mut App, key: KeyEvent) {
    let AppMode::Edit {
        buffer,
        cursor_pos,
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
        KeyCode::Char(c) => {
            buffer.insert(*cursor_pos, c);
            *cursor_pos += c.len_utf8();
        }
        KeyCode::Backspace => {
            if *cursor_pos > 0 {
                // 找到前一个字符的起始位置
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

// ── 确认模式 ─────────────────────────────────────────────────────────────────

fn handle_confirm(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.confirm_save_strip_comments();
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.mode = AppMode::Normal;
            app.set_status("已取消保存", StatusLevel::Info);
        }
        _ => {}
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
