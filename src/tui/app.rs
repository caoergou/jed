use std::{collections::HashSet, path::PathBuf};

use ratatui::widgets::ListState;

use crate::engine::{
    delete, format_pretty, parse_lenient, set, FormatOptions, JsonValue,
};

use super::tree::{flatten, TreeLine};

/// TUI 的交互模式。
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    /// 普通导航模式。
    Normal,
    /// 正在编辑某个值。
    Edit {
        /// 被编辑的节点路径。
        path: String,
        /// 编辑缓冲区内容。
        buffer: String,
        /// 光标在缓冲区中的字节位置。
        cursor_pos: usize,
    },
    /// 等待确认剥离注释。
    ConfirmStripComments,
}

/// 状态消息的级别。
#[derive(Debug, Clone)]
pub enum StatusLevel {
    Info,
    Warn,
    Error,
}

/// 应用整体状态。
pub struct App {
    pub doc: JsonValue,
    pub file_path: PathBuf,
    pub modified: bool,
    /// 是否含有注释（JSONC 格式），保存前需确认。
    pub has_comments: bool,

    /// 当前选中行的索引（相对于 flat tree）。
    pub cursor: usize,
    /// ratatui ListState，用于追踪滚动位置。
    pub list_state: ListState,

    /// 已展开节点的路径集合。
    pub expanded: HashSet<String>,

    /// 撤销栈（保存文档快照）。
    pub undo_stack: Vec<JsonValue>,
    /// 重做栈。
    pub redo_stack: Vec<JsonValue>,

    pub mode: AppMode,
    pub status: Option<(String, StatusLevel)>,
    pub should_quit: bool,
}

impl App {
    /// 从文件路径创建 App，完成初始解析。
    pub fn from_file(path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("无法读取 '{}': {e}", path.display()))?;

        let has_comments = content.contains("//") || content.contains("/*");
        let output = parse_lenient(&content)
            .map_err(|e| format!("解析失败: {e}"))?;

        // 默认展开根节点
        let mut expanded = HashSet::new();
        expanded.insert(".".into());

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Ok(Self {
            doc: output.value,
            file_path: path,
            modified: false,
            has_comments,
            cursor: 0,
            list_state,
            expanded,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            mode: AppMode::Normal,
            status: None,
            should_quit: false,
        })
    }

    /// 生成当前的树形行列表。
    pub fn tree_lines(&self) -> Vec<TreeLine> {
        flatten(&self.doc, &self.expanded)
    }

    /// 当前选中的树行（如果存在）。
    pub fn current_line<'a>(&self, lines: &'a [TreeLine]) -> Option<&'a TreeLine> {
        lines.get(self.cursor)
    }

    // ── 导航 ──────────────────────────────────────────────────────────────────

    pub fn move_down(&mut self) {
        let len = self.tree_lines().len();
        if self.cursor + 1 < len {
            self.cursor += 1;
            self.list_state.select(Some(self.cursor));
        }
    }

    pub fn move_up(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            self.list_state.select(Some(self.cursor));
        }
    }

    /// 展开当前节点，若已展开则移入第一个子节点。
    pub fn expand_or_enter(&mut self) {
        let lines = self.tree_lines();
        let Some(line) = lines.get(self.cursor) else {
            return;
        };
        if line.path.starts_with("__close__") {
            return;
        }
        if !line.has_children {
            return;
        }
        if line.is_expanded {
            // 已展开：移入第一个子节点
            if self.cursor + 1 < lines.len() {
                self.cursor += 1;
                self.list_state.select(Some(self.cursor));
            }
        } else {
            self.expanded.insert(line.path.clone());
        }
    }

    /// 折叠当前节点，若已折叠则移至父节点。
    pub fn collapse_or_go_parent(&mut self) {
        let lines = self.tree_lines();
        let Some(line) = lines.get(self.cursor) else {
            return;
        };

        // 若当前是闭括号行，先跳到对应的开括号行
        let path = if line.path.starts_with("__close__") {
            line.path.trim_start_matches("__close__").to_string()
        } else {
            line.path.clone()
        };

        if self.expanded.contains(&path) {
            self.expanded.remove(&path);
            // 光标跳回该节点的开括号行
            let new_lines = self.tree_lines();
            if let Some(pos) = new_lines.iter().position(|l| l.path == path) {
                self.cursor = pos;
                self.list_state.select(Some(pos));
            }
        } else {
            // 已折叠：移至父节点
            let parent = parent_path(&path);
            let new_lines = self.tree_lines();
            if let Some(pos) = new_lines.iter().position(|l| l.path == parent) {
                self.cursor = pos;
                self.list_state.select(Some(pos));
            }
        }
    }

    // ── 编辑 ──────────────────────────────────────────────────────────────────

    /// 进入编辑模式，仅对基本类型节点有效。
    pub fn start_edit(&mut self) {
        let lines = self.tree_lines();
        let Some(line) = lines.get(self.cursor) else {
            return;
        };
        if line.path.starts_with("__close__") || line.has_children {
            self.set_status("只能编辑基本类型的值（string/number/boolean/null）", StatusLevel::Warn);
            return;
        }
        let current_val = match crate::engine::get(&self.doc, &line.path) {
            Ok(v) => match v {
                JsonValue::String(s) => s.clone(),
                JsonValue::Bool(b) => b.to_string(),
                JsonValue::Number(n) => {
                    if n.fract() == 0.0 && n.abs() < 1e15 {
                        format!("{}", *n as i64)
                    } else {
                        format!("{n}")
                    }
                }
                JsonValue::Null => "null".into(),
                _ => return,
            },
            Err(_) => return,
        };
        let len = current_val.len();
        self.mode = AppMode::Edit {
            path: line.path.clone(),
            buffer: current_val,
            cursor_pos: len,
        };
    }

    /// 确认编辑，将缓冲区解析为 JSON 值并写入文档。
    pub fn confirm_edit(&mut self) {
        let AppMode::Edit { path, buffer, .. } = &self.mode else {
            return;
        };
        let path = path.clone();
        let raw = buffer.clone();

        // 尝试解析为 JSON，失败则视为字符串
        let new_val = match serde_json::from_str::<serde_json::Value>(&raw) {
            Ok(v) => JsonValue::from(v),
            Err(_) => JsonValue::String(raw),
        };

        self.snapshot();
        if let Err(e) = set(&mut self.doc, &path, new_val) {
            self.set_status(&format!("编辑失败：{e}"), StatusLevel::Error);
        } else {
            self.modified = true;
            self.set_status("已更新", StatusLevel::Info);
        }
        self.mode = AppMode::Normal;
    }

    /// 取消编辑。
    pub fn cancel_edit(&mut self) {
        self.mode = AppMode::Normal;
    }

    // ── 删除 ──────────────────────────────────────────────────────────────────

    pub fn delete_current(&mut self) {
        let lines = self.tree_lines();
        let Some(line) = lines.get(self.cursor) else {
            return;
        };
        if line.path == "." || line.path.starts_with("__close__") {
            self.set_status("不能删除根节点", StatusLevel::Warn);
            return;
        }
        let path = line.path.clone();
        self.snapshot();
        match delete(&mut self.doc, &path) {
            Ok(_) => {
                self.modified = true;
                // 光标不超出新范围
                let new_len = self.tree_lines().len();
                if self.cursor >= new_len && self.cursor > 0 {
                    self.cursor = new_len - 1;
                    self.list_state.select(Some(self.cursor));
                }
                self.set_status(&format!("已删除 {path}"), StatusLevel::Info);
            }
            Err(e) => self.set_status(&format!("删除失败：{e}"), StatusLevel::Error),
        }
    }

    // ── 撤销/重做 ─────────────────────────────────────────────────────────────

    fn snapshot(&mut self) {
        self.undo_stack.push(self.doc.clone());
        self.redo_stack.clear();
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.doc.clone());
            self.doc = prev;
            self.modified = true;
            self.clamp_cursor();
            self.set_status("已撤销", StatusLevel::Info);
        } else {
            self.set_status("没有可撤销的操作", StatusLevel::Warn);
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.doc.clone());
            self.doc = next;
            self.modified = true;
            self.clamp_cursor();
            self.set_status("已重做", StatusLevel::Info);
        } else {
            self.set_status("没有可重做的操作", StatusLevel::Warn);
        }
    }

    // ── 保存 ──────────────────────────────────────────────────────────────────

    /// 尝试保存。若文件含注释则先进入确认模式。
    pub fn try_save(&mut self) {
        if self.has_comments {
            self.mode = AppMode::ConfirmStripComments;
            return;
        }
        self.do_save();
    }

    /// 确认剥离注释后保存。
    pub fn confirm_save_strip_comments(&mut self) {
        self.has_comments = false;
        self.do_save();
        self.mode = AppMode::Normal;
    }

    fn do_save(&mut self) {
        let content = format_pretty(&self.doc, &FormatOptions::default());
        match crate::command::write_file_atomic(&self.file_path, &content) {
            Ok(()) => {
                self.modified = false;
                self.set_status("已保存", StatusLevel::Info);
            }
            Err(e) => self.set_status(&format!("保存失败：{e}"), StatusLevel::Error),
        }
    }

    // ── 辅助 ──────────────────────────────────────────────────────────────────

    pub fn set_status(&mut self, msg: &str, level: StatusLevel) {
        self.status = Some((msg.to_string(), level));
    }

    fn clamp_cursor(&mut self) {
        let len = self.tree_lines().len();
        if self.cursor >= len && len > 0 {
            self.cursor = len - 1;
            self.list_state.select(Some(self.cursor));
        }
    }

    /// 获取当前选中节点的路径字符串，用于状态栏显示。
    pub fn current_path(&self) -> String {
        let lines = self.tree_lines();
        lines
            .get(self.cursor)
            .map(|l| {
                if l.path.starts_with("__close__") {
                    l.path.trim_start_matches("__close__").to_string()
                } else {
                    l.path.clone()
                }
            })
            .unwrap_or_else(|| ".".into())
    }
}

/// 计算路径的父路径。
fn parent_path(path: &str) -> String {
    if path == "." {
        return ".".into();
    }
    // 从末尾找到最后一个 '.' 或 '['
    let bytes = path.as_bytes();
    for i in (1..bytes.len()).rev() {
        if bytes[i] == b'.' || bytes[i] == b'[' {
            let parent = &path[..i];
            return if parent.is_empty() { ".".into() } else { parent.into() };
        }
    }
    ".".into()
}
