# CLI 参考手册：je

## 命令总览

```
je <文件> [子命令] [参数] [选项]
```

无子命令时进入 TUI 模式；有子命令时执行命令后退出。

---

## 全局选项

| 选项 | 说明 |
|------|------|
| `--json` | 所有输出包装为 JSON 格式 `{"ok":...,"value":...}` |
| `--compact` | 值输出为压缩 JSON（不美化） |
| `--indent <n>` | 美化输出时的缩进空格数，默认 2 |
| `--no-color` | 禁用颜色输出（TUI 模式同样生效） |
| `-h, --help` | 显示帮助 |
| `-V, --version` | 显示版本号 |

---

## 读取命令

### `get <路径>`

获取指定路径处的值。

```bash
je config.json get .name
# 输出：Alice

je config.json get .servers[0].host
# 输出：localhost

je config.json get .missing
# stderr: 路径未找到: .missing
# exit: 2
```

输出规则：
- 字符串：裸输出，不含引号（方便 shell 使用）
- 数字/布尔/null：裸输出
- 对象/数组：美化 JSON 输出
- 加 `--json` 后字符串也会带引号，统一为 JSON 类型

---

### `keys <路径>`

列出对象的所有 key，或数组的所有索引。

```bash
je config.json keys .
# 输出（每行一个）：
# name
# age
# servers

je config.json keys .servers
# 输出：
# 0
# 1
# 2
```

---

### `len <路径>`

返回数组长度或对象 key 数量。

```bash
je config.json len .servers
# 输出：3

je config.json len .
# 输出：5
```

---

### `type <路径>`

返回路径处值的类型。

```bash
je config.json type .name
# 输出：string

je config.json type .servers
# 输出：array

je config.json type .count
# 输出：number
```

可能的输出：`string`、`number`、`boolean`、`null`、`object`、`array`

---

### `exists <路径>`

检查路径是否存在。

```bash
je config.json exists .name
# exit 0（存在）

je config.json exists .missing
# exit 2（不存在）
```

无 stdout 输出，仅通过退出码区分。适合 shell 脚本的 `if` 判断：

```bash
if je config.json exists .mcpServers.github; then
    echo "github MCP 已配置"
fi
```

---

### `schema`

推断并输出文件的结构（不含实际值）。

```bash
je config.json schema
# 输出：
# {
#   name: string,
#   age: number,
#   servers: [{host: string, port: number}],
#   enabled: boolean
# }
```

适合 Agent 在读取具体值前先了解文件结构，减少后续操作的 token 消耗。

---

### `check`

校验 JSON 格式是否合法。

```bash
je config.json check
# 合法时：无输出，exit 0

je broken.json check
# stderr: 第 12 行，第 5 列：尾部逗号
# stderr: 第 34 行，第 1 列：未终止的字符串
# exit 1
```

---

## 写入命令

### `set <路径> <值>`

设置指定路径的值。路径不存在时自动创建中间层。

```bash
je config.json set .name "Bob"
# 输出：ok

je config.json set .server.host "127.0.0.1"
# 若 .server 不存在，自动创建对象
# 输出：ok

je config.json set .tags[0] "rust"
# 输出：ok

# 值可以是任意 JSON 类型
je config.json set .count 42
je config.json set .enabled true
je config.json set .data null
je config.json set .config '{"timeout": 30}'
```

---

### `del <路径>`

删除指定路径的 key 或数组元素。

```bash
je config.json del .name
# 输出：ok

je config.json del .servers[1]
# 删除数组中的第 2 个元素
# 输出：ok

je config.json del .missing
# stderr: 路径未找到: .missing
# exit 2
```

---

### `add <路径> <值>`

向数组末尾追加元素，或向对象合并新字段。

```bash
# 追加到数组
je config.json add .tags "golang"
# 输出：ok

# 合并到对象（已存在的 key 会被覆盖）
je config.json add .server '{"timeout": 30, "retry": 3}'
# 输出：ok
```

---

### `mv <源路径> <目标路径>`

移动或重命名 key。

```bash
je config.json mv .oldName .newName
# 输出：ok

je config.json mv .config.debug .config.debugMode
# 输出：ok
```

---

### `patch <json>`

一次性批量操作，减少 Agent 的调用次数（节省 token）。

patch 格式遵循标准 JSON Patch（RFC 6902），支持以下操作：
`add`、`remove`、`replace`、`move`、`copy`、`test`

```bash
je config.json patch '[
  {"op": "replace", "path": ".name",    "value": "Bob"},
  {"op": "replace", "path": ".version", "value": 2},
  {"op": "remove",  "path": ".legacy"},
  {"op": "add",     "path": ".tags/-",  "value": "new"}
]'
# 输出：patched 4 ops
```

注：数组末尾追加使用 `path/-` 后缀（RFC 6902 规范）。

所有操作原子执行：要么全部成功，要么全部回滚。

---

## 格式化/修复命令

### `fmt`

原地美化格式化 JSON 文件。

```bash
je config.json fmt
# 输出：ok

je config.json fmt --indent 4
# 使用 4 空格缩进
```

---

### `fix`

自动检测并修复常见 JSON 格式错误，然后格式化。

```bash
je broken.json fix
# 输出：fixed 3 errors
#   第 12 行：移除尾部逗号
#   第 18 行：单引号替换为双引号
#   第 25 行：给 key 加引号

je broken.json fix --dry-run
# 预览将修复的内容，不实际写入文件
# exit 0（有可修复的错误）或 exit 1（有无法修复的错误）
```

---

### `minify`

原地压缩 JSON，移除所有空白。

```bash
je config.json minify
# 输出：ok
```

---

## 工具命令

### `diff <另一个文件>`

对比两个 JSON 文件的结构差异。

```bash
je old.json diff new.json
# 输出（类 diff 格式）：
# - .name: "Alice"
# + .name: "Bob"
# + .version: 2
# - .legacy: true
```

---

### `tui`

显式启动 TUI 模式（等同于不带子命令）。

```bash
je config.json tui
```

---

## Agent 使用示例

以下示例展示 Agent 如何以最小 token 消耗完成常见任务：

### 场景：修改 Claude Code 的 MCP 配置

```bash
# 1. 先了解文件结构（不读取全部内容）
je ~/.claude/settings.json schema
# 输出：{mcpServers: {[name]: {command: string, args: [string], env: {}}}, defaultMode: string}

# 2. 检查某个 MCP server 是否已存在
je ~/.claude/settings.json exists .mcpServers.github
# exit 0 → 已存在

# 3. 只读取需要的那个值
je ~/.claude/settings.json get .mcpServers.github.command
# 输出：/usr/local/bin/gh-mcp

# 4. 更新单个字段
je ~/.claude/settings.json set .mcpServers.github.env.TOKEN "ghp_xxxx"
# 输出：ok

# 5. 批量更新多个字段（一次调用，RFC 6902）
je ~/.claude/settings.json patch '[
  {"op": "replace", "path": ".defaultMode",                  "value": "acceptEdits"},
  {"op": "add",     "path": ".mcpServers.github.enabled",    "value": true}
]'
# 输出：patched 2 ops
```

整个流程：Agent 只消耗了结构摘要 + 单个值的 token，而非整个 JSON 文件的内容。

---

## 退出码参考

| 退出码 | 含义 | 典型场景 |
|--------|------|----------|
| 0 | 成功 | 所有正常操作 |
| 1 | 通用错误 | JSON 无效、写入权限不足、`fix` 有无法修复的错误 |
| 2 | 路径未找到 | `get`/`del`/`exists` 找不到路径 |
| 3 | 类型不匹配 | 对非数组路径执行 `add` 追加操作 |
