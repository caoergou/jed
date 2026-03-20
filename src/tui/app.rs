use std::collections::HashSet;
use std::path::PathBuf;

use ratatui::widgets::ListState;

use crate::engine::{
    FormatOptions, JsonValue, add as engine_add, delete as engine_delete, format_pretty, get,
    parse_lenient, rename_key, set as engine_set,
};

use super::tree::{TreeLine, flatten};

/// TUI 的交互模式。
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    /// 普通导航模式。
    Normal,
    /// 正在编辑某个值。
    Edit {
        /// 被编辑的节点路径。
        path: String,
        /// 当前值的类型（用于显示）。
        value_type: String,
        /// 编辑缓冲区内容。
        buffer: String,
        /// 光标在缓冲区中的字节位置。
        cursor_pos: usize,
    },
    /// 正在编辑某个键名。
    EditKey {
        /// 被编辑的节点路径。
        path: String,
        /// 原始键名（不含引号）。
        old_key: String,
        /// 编辑缓冲区内容。
        buffer: String,
        /// 光标在缓冲区中的字节位置。
        cursor_pos: usize,
    },
    /// 等待确认剥离注释。
    ConfirmStripComments,
    /// 搜索模式。
    Search {
        /// 搜索查询字符串。
        query: String,
        /// 光标在查询字符串中的位置。
        cursor_pos: usize,
    },
    /// 添加节点模式。
    AddNode {
        /// 父节点的路径。
        parent_path: String,
        /// 父节点是否为数组。
        is_array: bool,
        /// key 缓冲区（数组模式不使用）。
        key_buffer: String,
        /// 光标在 key 缓冲区的位置。
        key_cursor: usize,
    },
    /// 右键菜单模式。
    ContextMenu {
        /// 菜单位置（行号）。
        row: usize,
        /// 菜单项索引。
        selected: usize,
        /// 鼠标点击位置 X。
        mouse_x: u16,
        /// 鼠标点击位置 Y。
        mouse_y: u16,
    },
}

/// 右键菜单操作项。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextAction {
    Edit,
    AddChild,
    AddSibling,
    Delete,
    CopyKey,
    CopyValue,
    CopyPath,
    ExpandAll,
    CollapseAll,
}

impl ContextAction {
    pub fn all() -> &'static [ContextAction] {
        &[
            ContextAction::Edit,
            ContextAction::AddChild,
            ContextAction::AddSibling,
            ContextAction::Delete,
            ContextAction::CopyKey,
            ContextAction::CopyValue,
            ContextAction::CopyPath,
            ContextAction::ExpandAll,
            ContextAction::CollapseAll,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            ContextAction::Edit => "编辑",
            ContextAction::AddChild => "添加子级",
            ContextAction::AddSibling => "添加兄弟",
            ContextAction::Delete => "删除",
            ContextAction::CopyKey => "复制 Key",
            ContextAction::CopyValue => "复制 Value",
            ContextAction::CopyPath => "复制路径",
            ContextAction::ExpandAll => "展开全部",
            ContextAction::CollapseAll => "折叠全部",
        }
    }
}

/// 状态消息的级别。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    Warn,
    Error,
}

/// 应用整体状态。
pub struct App {
    /// 文档树。
    pub doc: JsonValue,
    /// 当前文件路径。
    pub file_path: PathBuf,
    /// 是否已修改。
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

    // 鼠标双击支持
    pub last_click_time: Option<std::time::Instant>,
    pub last_click_row: Option<usize>,
    // 右键菜单悬停支持
    pub menu_hover_row: Option<usize>,
}

impl App {
    /// 从文件路径创建 App，完成初始解析。
    pub fn from_file(path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("无法读取 '{}': {e}", path.display()))?;

        let has_comments = content.contains("//") || content.contains("/*");
        let output = parse_lenient(&content).map_err(|e| format!("解析失败: {e}"))?;

        // 默认展开根节点
        let mut expanded = HashSet::new();
        if matches!(output.value, JsonValue::Object(_) | JsonValue::Array(_)) {
            expanded.insert(".".into());
        }

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
            last_click_time: None,
            last_click_row: None,
            menu_hover_row: None,
        })
    }

    /// 生成当前的树形行列表。
    pub fn tree_lines(&self) -> Vec<TreeLine> {
        flatten(&self.doc, &self.expanded)
    }

    /// 当前选中的树行（如果存在）。
    #[allow(dead_code)]
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

    /// 切换节点展开/折叠状态（双击使用）。
    pub fn expand_or_toggle(&mut self) {
        let lines = self.tree_lines();
        let Some(line) = lines.get(self.cursor) else {
            return;
        };
        if line.path.starts_with("__close__") || !line.has_children {
            return;
        }
        if line.is_expanded {
            self.expanded.remove(&line.path);
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
    #[allow(clippy::cast_possible_truncation)]
    pub fn start_edit(&mut self) {
        let lines = self.tree_lines();
        let Some(line) = lines.get(self.cursor) else {
            return;
        };
        if line.path.starts_with("__close__") || line.has_children {
            self.set_status(
                "只能编辑基本类型的值（string/number/boolean/null）",
                StatusLevel::Warn,
            );
            return;
        }

        // 获取当前值和类型
        let (current_val, value_type) = match get(&self.doc, &line.path) {
            Ok(v) => {
                let t = v.type_name().to_string();
                let s = match v {
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
                };
                (s, t)
            }
            Err(_) => return,
        };

        let len = current_val.len();
        self.mode = AppMode::Edit {
            path: line.path.clone(),
            value_type,
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
        if let Err(e) = engine_set(&mut self.doc, &path, new_val) {
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

    /// 进入编辑键名模式，仅对对象中的 key 有效（不支持数组索引）。
    pub fn start_edit_key(&mut self) {
        let lines = self.tree_lines();
        let Some(line) = lines.get(self.cursor) else {
            return;
        };

        // 不能编辑 __close__ 行
        if line.path.starts_with("__close__") {
            return;
        }

        // 不能编辑根节点
        if line.path == "." {
            self.set_status("不能重命名根节点", StatusLevel::Warn);
            return;
        }

        // 从路径中提取当前的 key
        // 路径格式例如: .parent.key 或 .parent[0]
        let old_key = line
            .display_key
            .trim_start_matches('"')
            .trim_end_matches('"')
            .to_string();

        // 检查是否是数组索引（不能重命名）
        if old_key.starts_with('[') && old_key.ends_with(']') {
            self.set_status("数组索引不能重命名", StatusLevel::Warn);
            return;
        }

        let len = old_key.len();
        self.mode = AppMode::EditKey {
            path: line.path.clone(),
            old_key: old_key.clone(),
            buffer: old_key,
            cursor_pos: len,
        };
    }

    /// 确认编辑键名。
    pub fn confirm_edit_key(&mut self) {
        // 先提取需要的数据，避免 borrow 冲突
        let (path, old_key, buffer) = match &self.mode {
            AppMode::EditKey {
                path,
                old_key,
                buffer,
                ..
            } => (path.clone(), old_key.clone(), buffer.clone()),
            _ => return,
        };

        let new_key = buffer.trim();
        if new_key.is_empty() {
            self.set_status("key 不能为空", StatusLevel::Warn);
            return;
        }

        if new_key == old_key.as_str() {
            // key 没有变化，取消编辑
            self.mode = AppMode::Normal;
            return;
        }

        self.snapshot();
        if let Err(e) = rename_key(&mut self.doc, &path, new_key) {
            self.set_status(&format!("重命名失败：{e}"), StatusLevel::Error);
        } else {
            self.modified = true;
            // 更新展开状态的路径（如果路径被展开）
            let new_path = if let Some(parent_end) = path.rfind('.') {
                format!("{}.{}", &path[..parent_end], new_key)
            } else {
                format!(".{}", new_key)
            };

            // 如果原路径在展开集合中，更新为新路径
            if self.expanded.contains(&path) {
                self.expanded.remove(&path);
                self.expanded.insert(new_path);
            }

            self.set_status("已重命名", StatusLevel::Info);
        }
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
        match engine_delete(&mut self.doc, &path) {
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

    // ── 右键菜单 ─────────────────────────────────────────────────────────────

    /// 进入右键菜单模式。
    pub fn show_context_menu(&mut self, mouse_x: u16, mouse_y: u16) {
        self.mode = AppMode::ContextMenu {
            row: self.cursor,
            selected: 0,
            mouse_x,
            mouse_y,
        };
    }

    /// 退出右键菜单模式。
    pub fn close_context_menu(&mut self) {
        self.mode = AppMode::Normal;
        self.menu_hover_row = None;
    }

    /// 执行右键菜单操作。
    pub fn execute_context_action(&mut self, action: ContextAction) {
        let lines = self.tree_lines();
        let Some(line) = lines.get(self.cursor) else {
            self.mode = AppMode::Normal;
            return;
        };

        match action {
            ContextAction::Edit => {
                self.mode = AppMode::Normal;
                self.start_edit();
            }
            ContextAction::AddChild => {
                self.mode = AppMode::Normal;
                self.start_add_node();
            }
            ContextAction::AddSibling => {
                // 添加兄弟节点：先获取父节点路径
                self.mode = AppMode::Normal;
                if let Some(parent) = line.path.rfind('.') {
                    let parent_path = &line.path[..parent];
                    if !parent_path.is_empty() {
                        // TODO: 实现添加兄弟节点
                        self.set_status("添加兄弟节点功能开发中", StatusLevel::Info);
                    }
                }
            }
            ContextAction::Delete => {
                self.delete_current();
                self.mode = AppMode::Normal;
                self.menu_hover_row = None;
            }
            ContextAction::CopyKey => {
                // 复制 key（如果是对象键）
                let key = &line.display_key;
                if !key.is_empty() {
                    if let Err(e) = self.copy_to_clipboard(key) {
                        self.set_status(&format!("复制失败：{e}"), StatusLevel::Error);
                    } else {
                        self.set_status(&format!("已复制 key: {key}"), StatusLevel::Info);
                    }
                } else {
                    self.set_status("当前节点没有 key", StatusLevel::Warn);
                }
                self.mode = AppMode::Normal;
            }
            ContextAction::CopyValue => {
                // 复制 value
                let value = &line.value_preview;
                if !value.is_empty() {
                    if let Err(e) = self.copy_to_clipboard(value) {
                        self.set_status(&format!("复制失败：{e}"), StatusLevel::Error);
                    } else {
                        self.set_status("已复制 value", StatusLevel::Info);
                    }
                } else {
                    self.set_status("当前节点没有 value", StatusLevel::Warn);
                }
                self.mode = AppMode::Normal;
            }
            ContextAction::CopyPath => {
                // 复制 JSONPath
                let path = format!("$.{}", line.path.strip_prefix('.').unwrap_or(&line.path));
                if let Err(e) = self.copy_to_clipboard(&path) {
                    self.set_status(&format!("复制失败：{e}"), StatusLevel::Error);
                } else {
                    self.set_status(&format!("已复制路径: {path}"), StatusLevel::Info);
                }
                self.mode = AppMode::Normal;
            }
            ContextAction::ExpandAll => {
                self.expand_all();
                self.set_status("已展开全部节点", StatusLevel::Info);
                self.mode = AppMode::Normal;
            }
            ContextAction::CollapseAll => {
                self.collapse_all();
                self.set_status("已折叠全部节点", StatusLevel::Info);
                self.mode = AppMode::Normal;
            }
        }
    }

    /// 复制文本到系统剪贴板（跨平台）。
    fn copy_to_clipboard(&self, text: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut clipboard = arboard::Clipboard::new()?;
        clipboard.set_text(text)?;
        Ok(())
    }

    /// 展开全部节点。
    pub fn expand_all(&mut self) {
        fn collect_paths(value: &JsonValue, path: &str, paths: &mut Vec<String>) {
            match value {
                JsonValue::Object(obj) => {
                    for (k, v) in obj {
                        let new_path = if path.is_empty() {
                            k.clone()
                        } else {
                            format!("{}.{}", path, k)
                        };
                        if matches!(v, JsonValue::Object(_) | JsonValue::Array(_)) {
                            paths.push(new_path.clone());
                            collect_paths(v, &new_path, paths);
                        }
                    }
                }
                JsonValue::Array(arr) => {
                    for (i, v) in arr.iter().enumerate() {
                        let new_path = format!("{}[{}]", path, i);
                        if matches!(v, JsonValue::Object(_) | JsonValue::Array(_)) {
                            paths.push(new_path.clone());
                            collect_paths(v, &new_path, paths);
                        }
                    }
                }
                _ => {}
            }
        }
        let mut paths = Vec::new();
        collect_paths(&self.doc, "", &mut paths);
        for p in paths {
            self.expanded.insert(p);
        }
    }

    /// 折叠全部节点。
    pub fn collapse_all(&mut self) {
        self.expanded.clear();
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

    // ── 搜索 ──────────────────────────────────────────────────────────────────

    /// 进入搜索模式。
    pub fn start_search(&mut self) {
        self.mode = AppMode::Search {
            query: String::new(),
            cursor_pos: 0,
        };
    }

    /// 跳转到下一个匹配项。
    pub fn search_next(&mut self) {
        let AppMode::Search { query, .. } = &self.mode else {
            return;
        };
        if query.is_empty() {
            return;
        }

        let lines = self.tree_lines();
        let q = query.to_lowercase();

        // 从当前位置往后找
        for (i, line) in lines.iter().enumerate().skip(self.cursor + 1) {
            if line.display_key.to_lowercase().contains(&q)
                || line.value_preview.to_lowercase().contains(&q)
            {
                self.cursor = i;
                self.list_state.select(Some(i));
                return;
            }
        }

        // 循环到开头继续找
        for (i, line) in lines.iter().enumerate().take(self.cursor + 1) {
            if line.display_key.to_lowercase().contains(&q)
                || line.value_preview.to_lowercase().contains(&q)
            {
                self.cursor = i;
                self.list_state.select(Some(i));
                return;
            }
        }
    }

    /// 取消搜索。
    pub fn cancel_search(&mut self) {
        self.mode = AppMode::Normal;
    }

    // ── 添加节点 ─────────────────────────────────────────────────────────────

    /// 进入添加节点模式。
    pub fn start_add_node(&mut self) {
        let lines = self.tree_lines();
        let Some(line) = lines.get(self.cursor) else {
            return;
        };

        // 确定父节点路径
        let parent_path = if line.has_children && line.is_expanded {
            line.path.clone()
        } else {
            parent_path(&line.path)
        };

        // 判断父节点类型
        let is_array = matches!(get(&self.doc, &parent_path), Ok(JsonValue::Array(_)));

        // 数组模式：直接添加 null 元素，不需要弹窗
        if is_array {
            self.snapshot();
            // 用 add 追加到数组末尾
            if let Err(e) = engine_add(&mut self.doc, &parent_path, JsonValue::Null) {
                self.set_status(&format!("添加失败: {e}"), StatusLevel::Error);
            } else {
                self.set_status("已添加空元素", StatusLevel::Info);
                // 展开父节点
                self.expanded.insert(parent_path.clone());
            }
            return;
        }

        // 对象模式：弹出输入框，只输入 key
        self.mode = AppMode::AddNode {
            parent_path,
            is_array,
            key_buffer: String::new(),
            key_cursor: 0,
        };
    }

    /// 确认添加节点（对象模式）。
    pub fn confirm_add_node(&mut self) {
        let AppMode::AddNode {
            parent_path,
            is_array: _,
            key_buffer,
            key_cursor: _,
        } = &self.mode.clone()
        else {
            return;
        };

        // 对象模式：key 不能为空
        if key_buffer.is_empty() {
            self.set_status("需要输入字段名", StatusLevel::Error);
            return;
        }

        self.snapshot();

        // 构建目标路径
        let target_path = if parent_path == "." {
            format!(".{key_buffer}")
        } else {
            format!("{parent_path}.{key_buffer}")
        };

        // 添加值默认为 null
        if let Err(e) = engine_set(&mut self.doc, &target_path, JsonValue::Null) {
            self.set_status(&format!("添加失败: {e}"), StatusLevel::Error);
            return;
        }

        self.modified = true;
        self.mode = AppMode::Normal;

        // 展开父节点
        self.expanded.insert(parent_path.clone());

        // 找到新添加的节点并选中
        let lines = self.tree_lines();
        if let Some(idx) = lines.iter().position(|l| l.path == target_path) {
            self.cursor = idx;
            self.list_state.select(Some(idx));
        }

        self.set_status("已添加", StatusLevel::Info);
    }

    /// 取消添加节点。
    pub fn cancel_add_node(&mut self) {
        self.mode = AppMode::Normal;
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
        lines.get(self.cursor).map_or_else(
            || ".".into(),
            |l| {
                if l.path.starts_with("__close__") {
                    l.path.trim_start_matches("__close__").to_string()
                } else {
                    l.path.clone()
                }
            },
        )
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
            return if parent.is_empty() {
                ".".into()
            } else {
                parent.into()
            };
        }
    }
    ".".into()
}
