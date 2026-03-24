use std::collections::HashSet;
use std::path::PathBuf;

use ratatui::widgets::ListState;

use crate::engine::{
    FormatOptions, JsonValue, add as engine_add, delete as engine_delete, format_pretty, get,
    insert as engine_insert, parse_lenient, rename_key, set as engine_set,
};
use crate::i18n::{get_locale, t_to};

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
        /// 实时检测的类型（用于反馈）。
        detected_type: Option<String>,
        /// 是否有解析错误。
        parse_error: Option<String>,
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
    /// 帮助面板。
    Help,
    /// 退出确认（文件已修改时）。
    ConfirmQuit {
        /// 上一次按键是否是 Escape（用于检测连续按两次）
        last_was_escape: bool,
    },
    /// 保存前预览 diff
    ConfirmSave {
        /// 原始内容（保存前的状态）
        original_content: String,
    },
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
        /// 当前阶段：false = 输入 key，true = 选择类型
        selecting_type: bool,
        /// 选中的类型索引：0=null, 1=object, 2=array
        type_selected: usize,
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
        ]
    }

    #[allow(clippy::trivially_copy_pass_by_ref)]
    pub fn label(&self) -> String {
        let locale = get_locale();
        match self {
            ContextAction::Edit => t_to("tui.action.edit", &locale),
            ContextAction::AddChild => t_to("tui.action.add_child", &locale),
            ContextAction::AddSibling => t_to("tui.action.add_sibling", &locale),
            ContextAction::Delete => t_to("tui.action.delete", &locale),
            ContextAction::CopyKey => t_to("tui.action.copy_key", &locale),
            ContextAction::CopyValue => t_to("tui.action.copy_value", &locale),
            ContextAction::CopyPath => t_to("tui.action.copy_path", &locale),
        }
    }

    /// 获取操作对应的快捷键（单个字符）
    pub fn shortcut(self) -> char {
        match self {
            ContextAction::Edit => 'e',
            ContextAction::AddChild => 'a',
            ContextAction::AddSibling => 's',
            ContextAction::Delete => 'd',
            ContextAction::CopyKey => 'c',
            ContextAction::CopyValue => 'v',
            ContextAction::CopyPath => 'p',
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
    // 退出确认：追踪上次按键是否是 Escape（用于检测连续按两次）
    pub last_escape_time: Option<std::time::Instant>,
}

impl App {
    /// 从文件路径创建 App，完成初始解析。
    pub fn from_file(path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let locale = get_locale();
        let content = std::fs::read_to_string(&path).map_err(|e| {
            t_to("err.read_failed", &locale)
                .replace("{0}", &path.display().to_string())
                .replace("{1}", &e.to_string())
        })?;

        let has_comments = content.contains("//") || content.contains("/*");
        let output = parse_lenient(&content).map_err(|e| {
            t_to("err.parse_failed", &locale)
                .replace("{0}", &path.display().to_string())
                .replace("{1}", &e.to_string())
        })?;

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
            last_escape_time: None,
        })
    }

    /// 生成当前的树形行列表。
    pub fn tree_lines(&self) -> Vec<TreeLine> {
        flatten(&self.doc, &self.expanded)
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
                &t_to("tui.status.edit_value_only", &get_locale()),
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

        // 检测初始值的类型
        let (detected_type, parse_error) = Self::detect_value_type(&current_val);

        let len = current_val.len();
        self.mode = AppMode::Edit {
            path: line.path.clone(),
            value_type,
            buffer: current_val,
            cursor_pos: len,
            detected_type,
            parse_error,
        };
    }

    /// 实时检测输入值的类型
    fn detect_value_type(input: &str) -> (Option<String>, Option<String>) {
        let locale = get_locale();
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return (Some("empty".to_string()), None);
        }

        // 尝试解析为 JSON
        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(v) => {
                let type_name = match v {
                    serde_json::Value::String(_) => "string",
                    serde_json::Value::Number(_) => "number",
                    serde_json::Value::Bool(_) => "boolean",
                    serde_json::Value::Null => "null",
                    serde_json::Value::Array(_) => "array",
                    serde_json::Value::Object(_) => "object",
                };
                (Some(type_name.to_string()), None)
            }
            Err(e) => {
                // 解析失败，作为字符串处理
                (
                    Some(t_to("tui.status.string_unquoted", &locale)),
                    Some(e.to_string()),
                )
            }
        }
    }

    /// 更新编辑缓冲区的实时校验状态
    pub fn update_edit_validation(&mut self) {
        let AppMode::Edit {
            buffer,
            detected_type,
            parse_error,
            ..
        } = &mut self.mode
        else {
            return;
        };
        let (new_type, new_error) = Self::detect_value_type(buffer);
        *detected_type = new_type;
        *parse_error = new_error;
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
            self.set_status(
                &t_to("err.edit_failed", &get_locale()).replace("{0}", &e.to_string()),
                StatusLevel::Error,
            );
        } else {
            self.modified = true;
            self.set_status(&t_to("status.updated", &get_locale()), StatusLevel::Info);
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
            self.set_status(
                &t_to("tui.status.cannot_rename_root", &get_locale()),
                StatusLevel::Warn,
            );
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
            self.set_status(
                &t_to("tui.status.cannot_rename_index", &get_locale()),
                StatusLevel::Warn,
            );
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
            self.set_status(
                &t_to("tui.status.key_empty", &get_locale()),
                StatusLevel::Warn,
            );
            return;
        }

        if new_key == old_key.as_str() {
            // key 没有变化，取消编辑
            self.mode = AppMode::Normal;
            return;
        }

        self.snapshot();
        if let Err(e) = rename_key(&mut self.doc, &path, new_key) {
            self.set_status(
                &t_to("err.rename_failed", &get_locale()).replace("{0}", &e.to_string()),
                StatusLevel::Error,
            );
        } else {
            self.modified = true;
            // 更新展开状态的路径（如果路径被展开）
            let new_path = if let Some(parent_end) = path.rfind('.') {
                format!("{}.{}", &path[..parent_end], new_key)
            } else {
                format!(".{new_key}")
            };

            // 如果原路径在展开集合中，更新为新路径
            if self.expanded.contains(&path) {
                self.expanded.remove(&path);
                self.expanded.insert(new_path);
            }

            self.set_status(&t_to("status.renamed", &get_locale()), StatusLevel::Info);
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
            self.set_status(
                &t_to("tui.status.cannot_delete_root", &get_locale()),
                StatusLevel::Warn,
            );
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
                self.set_status(&t_to("status.deleted", &get_locale()), StatusLevel::Info);
            }
            Err(e) => self.set_status(
                &t_to("err.delete_failed", &get_locale()).replace("{0}", &e.to_string()),
                StatusLevel::Error,
            ),
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
        match action {
            ContextAction::Edit => self.context_edit(),
            ContextAction::AddChild => self.context_add_child(),
            ContextAction::AddSibling => self.context_add_sibling(),
            ContextAction::Delete => self.context_delete(),
            ContextAction::CopyKey => self.context_copy_key(),
            ContextAction::CopyValue => self.context_copy_value(),
            ContextAction::CopyPath => self.context_copy_path(),
        }
    }

    fn context_edit(&mut self) {
        self.mode = AppMode::Normal;
        self.start_edit();
    }

    fn context_add_child(&mut self) {
        self.mode = AppMode::Normal;
        self.start_add_node();
    }

    fn context_delete(&mut self) {
        self.delete_current();
        self.mode = AppMode::Normal;
        self.menu_hover_row = None;
    }

    fn context_copy_key(&mut self) {
        let lines = self.tree_lines();
        let Some(line) = lines.get(self.cursor) else {
            self.mode = AppMode::Normal;
            return;
        };
        let key = &line.display_key;
        if key.is_empty() {
            self.set_status(&t_to("tui.status.no_key", &get_locale()), StatusLevel::Warn);
        } else if let Err(e) = Self::copy_to_clipboard(key) {
            self.set_status(
                &t_to("tui.status.copy_failed", &get_locale()).replace("{0}", &e.to_string()),
                StatusLevel::Error,
            );
        } else {
            self.set_status(
                &t_to("tui.status.copied_key", &get_locale()).replace("{0}", key),
                StatusLevel::Info,
            );
        }
        self.mode = AppMode::Normal;
    }

    fn context_copy_value(&mut self) {
        let lines = self.tree_lines();
        let Some(line) = lines.get(self.cursor) else {
            self.mode = AppMode::Normal;
            return;
        };
        let value = &line.value_preview;
        if value.is_empty() {
            self.set_status(
                &t_to("tui.status.no_value", &get_locale()),
                StatusLevel::Warn,
            );
        } else if let Err(e) = Self::copy_to_clipboard(value) {
            self.set_status(
                &t_to("tui.status.copy_failed", &get_locale()).replace("{0}", &e.to_string()),
                StatusLevel::Error,
            );
        } else {
            self.set_status(
                &t_to("tui.status.copied_value", &get_locale()),
                StatusLevel::Info,
            );
        }
        self.mode = AppMode::Normal;
    }

    fn context_copy_path(&mut self) {
        let lines = self.tree_lines();
        let Some(line) = lines.get(self.cursor) else {
            self.mode = AppMode::Normal;
            return;
        };
        let path = format!("$.{}", line.path.strip_prefix('.').unwrap_or(&line.path));
        if let Err(e) = Self::copy_to_clipboard(&path) {
            self.set_status(
                &t_to("tui.status.copy_failed", &get_locale()).replace("{0}", &e.to_string()),
                StatusLevel::Error,
            );
        } else {
            self.set_status(
                &t_to("tui.status.copied_path", &get_locale()).replace("{0}", &path),
                StatusLevel::Info,
            );
        }
        self.mode = AppMode::Normal;
    }

    fn context_add_sibling(&mut self) {
        self.mode = AppMode::Normal;
        let lines = self.tree_lines();
        let Some(line) = lines.get(self.cursor) else {
            return;
        };

        // 不能给根节点添加兄弟
        if line.path == "." {
            self.set_status(
                &t_to("tui.status.cannot_delete_root", &get_locale()),
                StatusLevel::Warn,
            );
            return;
        }

        // 获取父节点路径
        let parent = parent_path(&line.path);

        // 判断父节点类型
        match get(&self.doc, &parent) {
            Ok(JsonValue::Array(_)) => {
                // 数组：在当前索引后面插入 null
                let current_index = extract_array_index(&line.path);
                self.snapshot();
                if let Err(e) =
                    engine_insert(&mut self.doc, &parent, current_index + 1, JsonValue::Null)
                {
                    self.set_status(
                        &t_to("err.add_failed", &get_locale()).replace("{0}", &e.to_string()),
                        StatusLevel::Error,
                    );
                } else {
                    self.modified = true;
                    self.expanded.insert(parent.clone());
                    self.set_status(
                        &t_to("tui.status.added_null", &get_locale()),
                        StatusLevel::Info,
                    );
                    // 移动光标到新插入的节点
                    let new_lines = self.tree_lines();
                    let target_path = format!("{}[{}]", parent, current_index + 1);
                    if let Some(idx) = new_lines.iter().position(|l| l.path == target_path) {
                        self.cursor = idx;
                        self.list_state.select(Some(idx));
                    }
                }
            }
            Ok(JsonValue::Object(_)) => {
                // 对象：进入 AddNode 模式，在父节点添加新字段
                self.mode = AppMode::AddNode {
                    parent_path: parent,
                    is_array: false,
                    key_buffer: String::new(),
                    key_cursor: 0,
                    selecting_type: false,
                    type_selected: 0,
                };
            }
            _ => {
                self.set_status(
                    &t_to("tui.status.cannot_delete_root", &get_locale()),
                    StatusLevel::Warn,
                );
            }
        }
    }

    /// 复制文本到系统剪贴板（跨平台）。
    fn copy_to_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error>> {
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
                        let new_path = if path == "." {
                            format!(".{k}")
                        } else {
                            format!("{path}.{k}")
                        };
                        if matches!(v, JsonValue::Object(_) | JsonValue::Array(_)) {
                            paths.push(new_path.clone());
                            collect_paths(v, &new_path, paths);
                        }
                    }
                }
                JsonValue::Array(arr) => {
                    for (i, v) in arr.iter().enumerate() {
                        let new_path = format!("{path}[{i}]");
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
        // 根节点也需要展开
        paths.push(".".to_string());
        collect_paths(&self.doc, ".", &mut paths);
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
            self.set_status(&t_to("tui.status.undone", &get_locale()), StatusLevel::Info);
        } else {
            self.set_status(
                &t_to("tui.status.no_undo", &get_locale()),
                StatusLevel::Warn,
            );
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.doc.clone());
            self.doc = next;
            self.modified = true;
            self.clamp_cursor();
            self.set_status(&t_to("tui.status.redone", &get_locale()), StatusLevel::Info);
        } else {
            self.set_status(
                &t_to("tui.status.no_redo", &get_locale()),
                StatusLevel::Warn,
            );
        }
    }

    // ── 保存 ──────────────────────────────────────────────────────────────────

    /// 尝试保存。先显示 diff 预览。
    pub fn try_save(&mut self) {
        // 读取当前文件内容用于 diff
        let original_content = std::fs::read_to_string(&self.file_path).unwrap_or_default();
        let new_content = format_pretty(&self.doc, &FormatOptions::default());

        // 如果内容相同，直接提示无需保存
        if original_content == new_content {
            self.set_status(
                &t_to("tui.status.no_changes", &get_locale()),
                StatusLevel::Info,
            );
            return;
        }

        // 进入 diff 预览模式
        self.mode = AppMode::ConfirmSave { original_content };
    }

    /// 确认保存（从 diff 预览）。
    pub fn confirm_save(&mut self) {
        self.has_comments = false;
        self.do_save();
        self.mode = AppMode::Normal;
    }

    /// 取消保存。
    pub fn cancel_save(&mut self) {
        self.mode = AppMode::Normal;
        self.set_status(
            &t_to("tui.status.cancel_save", &get_locale()),
            StatusLevel::Info,
        );
    }

    pub fn do_save(&mut self) {
        let content = format_pretty(&self.doc, &FormatOptions::default());
        match crate::command::write_file_atomic(&self.file_path, &content) {
            Ok(()) => {
                self.modified = false;
                self.set_status(&t_to("status.saved", &get_locale()), StatusLevel::Info);
            }
            Err(e) => self.set_status(
                &t_to("err.save_failed", &get_locale()).replace("{0}", &e.to_string()),
                StatusLevel::Error,
            ),
        }
    }

    /// 获取新内容（用于 diff 预览）。
    pub fn get_new_content(&self) -> String {
        format_pretty(&self.doc, &FormatOptions::default())
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

        // 获取当前选中节点的值类型
        let value_type = line.value_type;
        let current_path = &line.path;

        // 判断当前选中的是否是对象或数组（即使是空的）
        let is_object = value_type == "object";
        let is_array = value_type == "array";

        // 确定父节点路径
        // 如果选中的是容器类型（对象/数组），则在内部添加；否则在父节点添加
        let parent_path = if (is_object || is_array) && line.is_expanded {
            current_path.clone()
        } else {
            parent_path(current_path)
        };

        // 重新判断父节点类型
        let parent_is_array = matches!(get(&self.doc, &parent_path), Ok(JsonValue::Array(_)));

        // 数组模式：直接添加 null 元素，不需要弹窗
        if parent_is_array {
            self.snapshot();
            // 用 add 追加到数组末尾
            if let Err(e) = engine_add(&mut self.doc, &parent_path, JsonValue::Null) {
                self.set_status(
                    &t_to("err.add_failed", &get_locale()).replace("{0}", &e.to_string()),
                    StatusLevel::Error,
                );
            } else {
                self.set_status(
                    &t_to("tui.status.added_null", &get_locale()),
                    StatusLevel::Info,
                );
                // 展开父节点
                self.expanded.insert(parent_path.clone());
            }
            return;
        }

        // 对象模式：弹出输入框，只输入 key
        self.mode = AppMode::AddNode {
            parent_path,
            is_array: parent_is_array,
            key_buffer: String::new(),
            key_cursor: 0,
            selecting_type: false,
            type_selected: 0,
        };
    }

    /// 确认添加节点（对象模式）。
    pub fn confirm_add_node(&mut self) {
        let AppMode::AddNode {
            parent_path,
            is_array: _,
            key_buffer,
            key_cursor: _,
            selecting_type,
            type_selected,
        } = &self.mode.clone()
        else {
            return;
        };

        // 阶段1：还在输入 key
        if !*selecting_type {
            // 对象模式：key 不能为空
            if key_buffer.is_empty() {
                self.set_status(
                    &t_to("tui.status.need_field_name", &get_locale()),
                    StatusLevel::Error,
                );
                return;
            }
            // 进入类型选择阶段
            if let AppMode::AddNode { selecting_type, .. } = &mut self.mode {
                *selecting_type = true;
            }
            return;
        }

        // 阶段2：类型选择阶段，确认添加
        self.snapshot();

        // 根据选中的类型创建值
        #[allow(clippy::default_trait_access)]
        let new_value = match type_selected {
            1 => JsonValue::Object(Default::default()), // {}
            2 => JsonValue::Array(Default::default()),  // []
            _ => JsonValue::Null,                       // null (默认)
        };

        // 构建目标路径
        let target_path = if parent_path == "." {
            format!(".{key_buffer}")
        } else {
            format!("{parent_path}.{key_buffer}")
        };

        if let Err(e) = engine_set(&mut self.doc, &target_path, new_value) {
            self.set_status(
                &t_to("err.add_failed", &get_locale()).replace("{0}", &e.to_string()),
                StatusLevel::Error,
            );
            return;
        }

        self.modified = true;
        self.mode = AppMode::Normal;

        // 如果是对象或数组，展开它
        if *type_selected != 0 {
            self.expanded.insert(target_path.clone());
        }

        // 展开父节点
        self.expanded.insert(parent_path.clone());

        // 找到新添加的节点并选中
        let lines = self.tree_lines();
        if let Some(idx) = lines.iter().position(|l| l.path == target_path) {
            self.cursor = idx;
            self.list_state.select(Some(idx));
        }

        self.set_status(&t_to("status.added", &get_locale()), StatusLevel::Info);
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

/// 从路径中提取数组索引（如 `.arr[3]` 返回 3）。
/// 如果路径不以数组索引结尾，返回 0。
fn extract_array_index(path: &str) -> usize {
    // 查找最后一个 [ 和 ]
    let open = path.rfind('[');
    let close = path.rfind(']');
    if let (Some(open_idx), Some(close_idx)) = (open, close)
        && close_idx > open_idx
    {
        let index_str = &path[open_idx + 1..close_idx];
        return index_str.parse().unwrap_or(0);
    }
    0
}
