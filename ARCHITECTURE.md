# 技术架构：jzen JSON 编辑器

## 系统总览

```
┌────────────────────────────────────────────────────────┐
│                      jzen 二进制                          │
│                                                        │
│   main.rs：检测模式（TUI vs 命令）                      │
│        │                     │                         │
│   ┌────▼──────┐        ┌─────▼──────┐                 │
│   │  TUI 模式  │        │  命令模式   │                 │
│   │  （人类）  │        │  （Agent） │                 │
│   └────┬──────┘        └─────┬──────┘                 │
│        └──────────┬──────────┘                         │
│                   ▼                                    │
│   ┌───────────────────────────────┐                    │
│   │          核心引擎              │                    │
│   │  parser / path / edit /       │                    │
│   │  fix / format / schema        │                    │
│   └───────────────┬───────────────┘                    │
│                   ▼                                    │
│          ┌────────────────┐                            │
│          │   文件 I/O      │                            │
│          │  （原子读写）   │                            │
│          └────────────────┘                            │
└────────────────────────────────────────────────────────┘
```

---

## 核心引擎

引擎操作**内存文档树**（`JsonValue`），无任何 I/O 依赖，完全可单元测试。

### 文档模型

```rust
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(JsonNumber),          // 保留整数与浮点的区别
    String(String),
    Array(Vec<JsonValue>),
    Object(IndexMap<String, JsonValue>),  // 保留插入顺序
}
```

对象使用 `IndexMap` 而非 `HashMap`，以保留 key 顺序——这对保存时最小化 diff 很重要。

---

### 解析器（`engine/parser.rs`）

两种解析模式：

**严格模式**：使用标准 `serde_json` 反序列化，用于已知格式正确的文件。

**宽松模式**：自定义 tokenizer，容忍以下情况：
- 对象和数组中的尾部逗号
- `//` 和 `/* */` 注释
- 单引号字符串
- 未加引号的 key（简单标识符）
- Python 字面量（`True` → `true`，`False` → `false`，`None` → `null`）
- 文件开头的 BOM

宽松模式同时返回解析结果**和**已做修复的列表，供 `fix --dry-run` 报告使用。

---

### 路径求值器（`engine/path.rs`）

解析并求值路径表达式：

```
PathExpr  = Segment*
Segment   = '.' Key
           | '[' Index ']'
           | '[' '-' Index ']'   // 从末尾计数

Key       = 标识符 | 带引号字符串
Index     = 整数
```

核心接口：

```rust
// 读取
pub fn get(doc: &JsonValue, path: &str) -> Result<&JsonValue, PathError>
pub fn exists(doc: &JsonValue, path: &str) -> bool

// 写入
pub fn set(doc: &mut JsonValue, path: &str, value: JsonValue) -> Result<(), PathError>
pub fn del(doc: &mut JsonValue, path: &str) -> Result<JsonValue, PathError>
```

`set` 会自动创建中间层的对象/数组（类似 `mkdir -p`）。

---

### 格式化器（`engine/format.rs`）

美化输出，可配置：
- 缩进大小（默认 2 空格）
- 末尾换行（默认有）
- 是否排序 key（默认否，保留插入顺序）

压缩：输出无空白的紧凑 JSON。

---

### 自动修复器（`engine/fix.rs`）

运行宽松解析器，收集修复列表，写回格式化后的输出。

```rust
pub struct FixResult {
    pub fixed: bool,
    pub repairs: Vec<Repair>,    // 已修改的内容
    pub errors: Vec<ParseError>, // 无法修复的内容
}

pub struct Repair {
    pub line: usize,
    pub col: usize,
    pub description: String,    // 例如："移除尾部逗号"
}
```

---

### 结构推断器（`engine/schema.rs`）

生成紧凑的结构摘要（不含实际值，只有类型和形状）：

```
输入：  {"name": "Alice", "age": 30, "tags": ["rust", "cli"]}
输出：  {name: string, age: number, tags: [string]}
```

对于异构数组，使用联合类型：`[string | number]`。

---

## 命令模式

### 分发逻辑（`command/mod.rs`）

核心命令（get/set/fix 等）在 `dispatch()` 中处理；tree、query、validate、convert 等扩展命令在 `main.rs` 中直接调用对应的 `run_*` 函数，不经过 `dispatch()`。

```rust
// command/mod.rs — 核心命令
match cmd {
    Command::Get { path, .. }  => read::cmd_get(file, &path, ctx),
    Command::Set { path, .. }  => write::cmd_set(file, &path, &value, ctx),
    Command::Fix { .. }        => repair::cmd_fix(file, dry_run, strip_comments, ctx),
    // ...
}

// main.rs — 扩展命令
Some(Command::Tree { .. })     => command::run_tree(&f, expand_all, path, json),
Some(Command::Validate { .. }) => command::run_validate(&f, schema, json),
Some(Command::Convert { .. })  => command::run_convert(&f, format, json),
```

### 输出层（`output.rs`）

所有命令通过统一的 `Ctx` 结构输出结果：

```rust
pub struct Ctx {
    pub cmd: &'static str,
    pub json: bool,      // 是否使用 --json 包装格式
}
```

输出方法：
- `ctx.print_value(v)` — 输出 JsonValue（字符串裸输出，对象/数组美化）
- `ctx.print_raw(v)` — 直接输出 serde_json::Value
- `ctx.print_error(msg, fix, actions)` — 错误输出，含修复建议和推荐命令
- `ctx.print_raw_with_actions(v, actions)` — 带推荐后续操作的输出（Agent 友好）

`--json` 模式下所有输出统一包装为：
```json
{"ok": true/false, "value": ..., "error": "...", "actions": [...]}
```

### i18n（`i18n.rs`）

通过 `JZEN_LANG` 环境变量（或 `--lang` 选项）控制输出语言。支持 `en`、`zh-CN`、`zh-TW`。
翻译字符串以静态映射存储，零运行时依赖。

### stdin 支持（`main.rs`）

当文件参数为 `-`（默认值）时，从 stdin 读取内容，写入 `tempfile` 临时文件，再传给命令处理器。临时文件通过 `keep()` 保留到进程退出。

### 原子文件写入

所有写操作流程：
1. 写入 `<文件>.tmp`
2. `fsync` 刷盘
3. 重命名为 `<文件>`（POSIX 下原子操作；Windows 下尽力保证）

防止进程在写入中途被杀死导致文件损坏。

---

## TUI 模式

### 应用状态（`tui/app.rs`）

```rust
pub struct App {
    pub doc: JsonValue,           // 当前文档
    pub file_path: PathBuf,
    pub modified: bool,
    pub cursor: Path,             // 当前选中的节点路径
    pub expanded: HashSet<Path>,  // 已展开的节点集合
    pub undo_stack: Vec<Snapshot>,
    pub redo_stack: Vec<Snapshot>,
    pub mode: AppMode,
    pub errors: Vec<ParseError>,  // 校验错误
}

pub enum AppMode {
    Normal,
    Edit { original: JsonValue },
    Search { query: String, matches: Vec<Path> },
}
```

### 渲染流程

每帧渲染：
1. 由 `doc` + `expanded` 集合生成可见节点的扁平列表
2. 渲染树形 widget（左侧面板，约 60% 宽度）
3. 渲染详情面板（右侧面板，约 40% 宽度）：当前值、类型、路径
4. 渲染状态栏（底部一行）：路径、修改标记、错误数量
5. 若处于编辑模式：在当前节点上渲染内联编辑器浮层
6. 若处于搜索模式：在底部渲染搜索栏

### 树形 Widget（`tui/tree.rs`）

将文档树转换为扁平的 `TreeLine` 列表用于渲染：

```rust
pub struct TreeLine {
    pub depth: usize,
    pub key: Option<String>,      // 对象字段名，数组元素为 None
    pub index: Option<usize>,     // 数组元素的索引
    pub value_preview: String,    // 截断的单行预览
    pub value_type: ValueType,
    pub is_expanded: bool,
    pub has_children: bool,
    pub has_error: bool,
    pub path: Path,
}
```

### 键位映射（`tui/keybinds.rs`）

| 按键 | 模式 | 操作 |
|------|------|------|
| `j` / `↓` | 普通 | 光标下移 |
| `k` / `↑` | 普通 | 光标上移 |
| `l` / `→` / `Enter` | 普通 | 展开节点 |
| `h` / `←` | 普通 | 折叠节点 |
| `e` | 普通 | 编辑当前值 |
| `a` | 普通 | 添加 key/元素 |
| `d` | 普通 | 删除当前节点 |
| `u` | 普通 | 撤销 |
| `ctrl+r` | 普通 | 重做 |
| `ctrl+s` | 普通 | 保存 |
| `/` | 普通 | 进入搜索 |
| `Esc` | 编辑/搜索 | 取消 |
| `Enter` | 编辑 | 确认编辑 |
| `q` | 普通 | 退出（有修改时提示确认） |

---

## 跨平台注意事项

| 问题 | 处理方式 |
|------|----------|
| 终端颜色 | `ratatui` 使用 `crossterm` 后端，自动处理 Windows Console API |
| 文件路径 | 全程使用 `std::path::PathBuf`，不用裸字符串 |
| 原子写入 | POSIX：`rename(2)`；Windows：`MoveFileExW` 带 `REPLACE_EXISTING` |
| 换行符 | 读取时规范化为 LF，写入时可配置保留原有风格 |
| 二进制分发 | CI 交叉编译：`x86_64-pc-windows-gnu`、`aarch64-apple-darwin` 等 |

---

## 测试策略

### 单元测试（按模块）

- `engine/parser`：往返测试、宽松模式 fixture
- `engine/path`：合法路径、非法路径、边界情况（空数组、unicode key）
- `engine/edit`：`set` 自动创建中间层、`del` 返回被删除的值
- `engine/fix`：每种错误类型都有 before/after fixture

### 集成测试（`tests/command/`）

每个子命令针对 fixture 文件测试：
- 正常情况
- 路径未找到 → exit code 2
- 类型不匹配 → exit code 3
- 无效 JSON → 错误正确报告

### TUI 测试

使用 `ratatui` 的 `TestBackend` 进行渲染快照测试。

---

## 性能目标

| 文件大小 | TUI 启动 | `get` 命令 | `fix` 命令 |
|----------|----------|-----------|-----------|
| < 100 KB | < 50ms   | < 5ms     | < 10ms    |
| 1 MB     | < 200ms  | < 20ms    | < 100ms   |
| 10 MB    | < 1s     | < 50ms    | < 500ms   |

> 大文件（> 1MB）的 TUI 虚拟滚动尚未实现，列为 Roadmap v2.x 目标。
