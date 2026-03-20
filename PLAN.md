# je — JSON 编辑器：项目计划

## 项目概述

`je` 是一个双接口 JSON 工具，同时服务于**人类用户**和 **AI Agent**。

- **人类**：完整 TUI 界面，树形导航、内联编辑、语法高亮
- **Agent**：最小化 CLI 模式，基于路径的操作，极低 token 消耗

同一个二进制文件，两种交互模式，共用同一套核心引擎。

---

## 核心目标

| 目标 | 说明 |
|------|------|
| 人类友好 | TUI 树形视图，键盘/鼠标导航，语法高亮 |
| Agent 友好 | 路径寻址 CLI 命令，最小化 stdout，无渲染噪音 |
| 减少 token 消耗 | Agent 只读写所需的部分，而非整个文件 |
| 容错解析 | 自动检测并修复常见 JSON 格式错误 |
| 跨平台 | 单一二进制，支持 Linux、macOS、Windows |
| 零外部依赖 | 不依赖 `jq` 或其他运行时 |

---

## 两种模式，一个工具

```bash
je settings.json                  # TUI 模式（人类使用）
je settings.json get .key         # 命令模式（Agent/脚本使用）
```

### 命令模式对 Agent 的价值

| 传统方式（Agent 读写 JSON） | je 命令模式 |
|--------------------------|------------|
| Read 整个文件进入 context | `get .key` → 只返回目标值 |
| 修改后 Write 整个文件 | `set .key val` → 返回 `ok` |
| Agent 手动解析 JSON | 路径寻址自动导航 |
| 格式错误需 Agent 自行修复 | `fix` 命令自动修复并报告 |
| 多次往返确认 | `patch` 一次完成批量操作 |

---

## 功能规格

### TUI 模式

通过 `je <文件>` 启动（无子命令）。

- 树形视图，支持展开/折叠（`l`/`h` 或方向键）
- 语法高亮（key、字符串、数字、布尔、null 不同颜色）
- 内联值编辑（在选中节点按 `e` 进入编辑模式）
- 添加 key/元素（`a`），删除（`d`），移动（`m`）
- 搜索/过滤（`/`）
- 撤销/重做（`u` / `ctrl+r`）
- 保存（`ctrl+s`）
- 校验错误以红色标记内联展示
- 底部状态栏：当前路径、文件修改状态、错误数量
- 支持 JSON、JSONC（带注释）、JSON5、宽松模式

### 命令模式（Agent 友好）

所有命令格式：`je <文件> <子命令> [参数] [选项]`

#### 读取操作

```bash
je file.json get <路径>           # 获取路径处的值
je file.json keys <路径>          # 列出对象的所有 key
je file.json len <路径>           # 数组/对象的长度
je file.json type <路径>          # 路径处值的类型
je file.json exists <路径>        # 路径存在则 exit 0，不存在则 exit 2
je file.json schema               # 推断结构（紧凑，不含实际值）
je file.json check                # 校验，错误输出到 stderr
```

#### 写入操作

```bash
je file.json set <路径> <值>      # 设置值（路径不存在则自动创建）
je file.json del <路径>           # 删除 key 或数组元素
je file.json add <路径> <值>      # 追加到数组，或合并到对象
je file.json patch '<json>'       # 一次性批量操作
je file.json mv <源路径> <目标路径> # 移动/重命名 key
```

#### 格式化/修复

```bash
je file.json fmt                  # 格式化（美化输出），原地修改
je file.json fix                  # 自动修复格式错误，然后格式化
je file.json fix --dry-run        # 预览将修复的内容，不实际写入
je file.json minify               # 压缩为最小 JSON，原地修改
```

#### 工具命令

```bash
je file.json diff <other.json>    # 对比两个 JSON 文件的结构差异
je file.json tui                  # 显式启动 TUI 模式
```

---

## 路径语法

采用 jq 风格的路径语法，TUI 状态栏与 CLI 保持一致：

```
.                          # 根节点
.key                       # 对象字段
.key.nested                # 嵌套字段
.array[0]                  # 数组索引
.array[-1]                 # 最后一个元素
.key.array[2].field        # 深层路径
```

---

## 输出设计（Agent token 效率）

### 默认输出（命令模式）

- `get`：只输出裸值，不含包装，不 dump 整个文件
- `set`/`del`/`add`：成功时输出 `ok`，仅此而已
- `fix`：输出摘要行，例如 `fixed 3 errors`
- `check`：成功时无输出（exit 0），错误输出到 stderr
- `schema`：输出紧凑结构，不含实际值
- `keys`：每行一个 key，无装饰

### `--json` 选项（结构化输出）

所有命令支持 `--json`，返回机器可读的 JSON：

```json
{"ok": true, "value": "..."}
{"ok": false, "error": "路径未找到: .foo.bar"}
```

### 退出码

| 退出码 | 含义 |
|--------|------|
| 0 | 成功 |
| 1 | 通用错误（JSON 无效、写入失败等） |
| 2 | 路径未找到 |
| 3 | 类型不匹配 |

---

## 错误检测与自动修复

`je fix` 处理最常见的 JSON 格式错误：

| 错误类型 | 示例 | 修复方式 |
|----------|------|----------|
| 尾部逗号 | `{"a": 1,}` | 移除尾部逗号 |
| 单引号 | `{'key': 'val'}` | 替换为双引号 |
| 未加引号的 key | `{key: "val"}` | 给 key 加引号 |
| 缺少逗号 | `{"a": 1 "b": 2}` | 插入逗号 |
| 注释 | `// 注释\n{"a":1}` | 剥离注释 |
| BOM 头 | 文件开头的 BOM | 剥离 BOM |
| Python 字面量 | `True`、`False`、`None` | 替换为 JSON 等价值 |

无法自动修复的错误，报告行号和列号，以 exit 1 退出。

---

## 技术选型

| 组件 | 选型 | 理由 |
|------|------|------|
| 语言 | Rust | 跨平台单二进制，性能好，内存安全 |
| TUI 框架 | `ratatui` | Rust 最活跃的 TUI 库 |
| JSON 解析 | `serde_json` | 标准，性能好 |
| 宽松解析 | `json5` crate 或自研 | 支持 JSONC、尾逗号、注释 |
| CLI 参数 | `clap` | 标准，支持 derive 宏 |
| 路径求值 | 自研 | 保持依赖最小，控制行为 |
| 撤销/重做 | 自研内存栈 | 足够简单，无需外部库 |

---

## 项目目录结构

```
je/
├── Cargo.toml
├── README.md
├── PLAN.md               # 本文件
├── ARCHITECTURE.md       # 技术架构详解
├── CLI_SPEC.md           # 完整 CLI 参考手册
├── src/
│   ├── main.rs           # 入口：判断 TUI 模式 vs 命令模式
│   ├── cli.rs            # clap 参数定义
│   │
│   ├── engine/           # 核心逻辑（无 I/O，完全可测试）
│   │   ├── mod.rs
│   │   ├── parser.rs     # 宽松 JSON 解析器
│   │   ├── path.rs       # 路径语法解析与求值
│   │   ├── edit.rs       # get/set/del/add/mv 操作
│   │   ├── format.rs     # 美化输出与压缩
│   │   ├── fix.rs        # 错误检测与自动修复
│   │   └── schema.rs     # 结构推断
│   │
│   ├── command/          # 命令模式处理器
│   │   ├── mod.rs
│   │   ├── read.rs       # get, keys, len, type, exists, schema, check
│   │   ├── write.rs      # set, del, add, patch, mv
│   │   ├── repair.rs     # fmt, fix, minify
│   │   └── diff.rs
│   │
│   └── tui/              # TUI 模式
│       ├── mod.rs
│       ├── app.rs        # 应用状态
│       ├── tree.rs       # 树形 widget
│       ├── editor.rs     # 内联值编辑器 widget
│       ├── search.rs     # 搜索/过滤浮层
│       └── keybinds.rs   # 键位映射
│
└── tests/
    ├── engine/           # 引擎单元测试
    ├── command/          # 命令集成测试
    └── fixtures/         # 测试用 JSON 文件
```

---

## 实施阶段

### 第一阶段：核心引擎（无 I/O）

产出：
- 宽松 JSON 解析器（支持 JSONC、尾逗号、单引号、注释）
- 路径解析器与求值器
- 格式化器（美化/压缩）
- 自动修复逻辑
- 结构推断器
- 以上所有模块的完整单元测试

本阶段产出一个独立可测试的库，无任何 I/O 依赖。

### 第二阶段：命令模式

产出：
- 基于 `clap` 的 CLI，包含所有子命令
- 最小化 stdout 输出
- `--json` 结构化输出选项
- 退出码规范
- 针对 fixture 文件的集成测试

**第二阶段结束后，工具即可被 Agent 完整使用。**

### 第三阶段：TUI 模式

产出：
- 树形视图 widget（展开/折叠）
- 语法高亮
- 状态栏（路径、修改状态、错误数）
- 基础内联编辑（修改基本类型的值）
- 保存/放弃流程
- 撤销/重做

### 第四阶段：打磨完善

产出：
- TUI 中添加/删除/移动节点
- 搜索/过滤浮层
- 鼠标支持
- Diff 视图
- Shell 补全（bash、zsh、fish）
- 分发包：Homebrew、`cargo install`、`.deb`/`.rpm`

---

## 不做的事（Non-Goals）

- 不是通用文本编辑器
- 不做 JSON Schema 验证（结构推断有，但不做完整校验）
- 不做 jq 表达式（只做路径寻址，不做完整查询语言）
- 无网络/HTTP 功能
- 无插件系统

---

## 已确认的设计决策

1. **路径语法**：使用 `.key` jq 风格（例如 `.servers[0].host`），对开发者最熟悉。

2. **patch 格式**：使用标准 JSON Patch（RFC 6902），支持 `add`/`remove`/`replace`/`move`/`copy`/`test` 操作。虽然比简单映射稍冗长，但标准化有利于 Agent 生成正确的格式。

3. **`get` 输出格式**：字符串带引号返回（`"hello"`），保持 JSON 类型一致性，Agent 管道中可直接判断类型。

4. **JSONC 注释处理**：
   - **MVP 阶段（Phase 1-3）**：保存时剥离注释。但在 TUI 模式下保存前弹出确认提示，命令模式需显式传 `--strip-comments` 才会丢弃注释，否则报错退出，防止静默数据丢失。
   - **Phase 4（未来）**：实现注释保留（基于 CST），标注为实验性功能。
   - **原因**：注释归属（删除/插入/移动 key 时注释跟谁走）存在天然歧义，强行实现会引入难以复现的 bug，分阶段处理更稳妥。

5. **写入安全**：使用原子写入（写临时文件 → fsync → 重命名），防止进程中断导致文件损坏。
