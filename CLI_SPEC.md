# CLI 参考手册：jed

## 命令总览

```
jed [文件] [子命令] [参数] [选项]
jed [子命令] [参数] [文件] [选项]
```

无子命令时进入 TUI 模式；有子命令时执行命令后退出。
文件参数可位于子命令前（`jed file.json get .key`）或后（`jed get .key file.json`），两种写法等价。
省略文件时从 stdin 读取（仅读取类命令支持管道）。

---

## 全局选项

| 选项 | 说明 |
|------|------|
| `--json` | 所有输出包装为 JSON 格式 `{"ok":...,"value":...}` |
| `--lang <lang>` | 输出语言：`en`、`zh-CN`、`zh-TW` |
| `--quiet` | 静默模式，抑制提示性输出 |
| `-h, --help` | 显示帮助 |
| `-V, --version` | 显示版本号 |

---

## 读取命令

### `get <路径> [文件]`

获取指定路径处的值。

```bash
jed get .name config.json
# 输出：Alice

jed get .servers[0].host config.json
# 输出：localhost

jed get .missing config.json
# stderr: 路径未找到: .missing
# exit: 2
```

输出规则：
- 字符串：裸输出，不含引号（方便 shell 使用）
- 数字/布尔/null：裸输出
- 对象/数组：美化 JSON 输出
- 加 `--json` 后统一为 JSON 格式包装

---

### `keys [路径] [文件]`

列出对象的所有 key，或数组的所有索引。

```bash
jed keys . config.json
# 输出（每行一个）：
# name
# age
# servers

jed keys .servers config.json
# 输出：
# 0
# 1
# 2
```

---

### `len [路径] [文件]`

返回数组长度或对象 key 数量。

```bash
jed len .servers config.json
# 输出：3
```

---

### `type [路径] [文件]`

返回路径处值的类型。

```bash
jed type .name config.json
# 输出：string

jed type .servers config.json
# 输出：array
```

可能的输出：`string`、`number`、`boolean`、`null`、`object`、`array`

---

### `exists <路径> [文件]`

检查路径是否存在。无 stdout 输出，仅通过退出码区分。

```bash
jed exists .name config.json
# exit 0（存在）

jed exists .missing config.json
# exit 2（不存在）
```

适合 shell 脚本的 `if` 判断：

```bash
if jed exists .mcpServers.github ~/.claude/settings.json; then
    echo "github MCP 已配置"
fi
```

---

### `schema [文件]`

推断并输出文件的结构（不含实际值，只有类型和形状）。

```bash
jed schema config.json
# 输出：
# {
#   name: string,
#   age: number,
#   servers: [{host: string, port: number}],
#   enabled: boolean
# }
```

加 `--json` 可输出标准 JSON Schema 格式：

```bash
jed --json schema config.json
```

---

### `check [文件]`

校验 JSON 格式是否合法。

```bash
jed check config.json
# 合法时：无输出，exit 0

jed check broken.json
# stderr: 第 12 行，第 5 列：尾部逗号
# exit 1
```

---

### `tree [文件]`

以树形结构展示 JSON，便于快速了解文件结构。

```bash
jed tree config.json
# 输出：
# {
#   name: Alice
#   servers: ...
# }

jed tree -e config.json           # 展开所有节点
jed tree -p .servers config.json  # 只展示指定路径下的子树
```

| 选项 | 说明 |
|------|------|
| `-e, --expand-all` | 展开所有嵌套节点 |
| `-p, --path <路径>` | 只展示指定路径下的内容 |

---

### `query <过滤表达式> [文件]`

使用路径表达式过滤/查询 JSON，功能与 `get` 等价，语义更明确。

```bash
jed query '.users[0]' data.json
jed query .database.host config.json
```

---

### `diff <另一个文件> [文件]`

对比两个 JSON 文件的结构差异。

```bash
jed diff new.json old.json
# 输出（类 diff 格式）：
# - .name: "Alice"
# + .name: "Bob"
# + .version: 2
# - .legacy: true
```

---

## 写入命令

### `set <路径> <值> [文件]`

设置指定路径的值。路径不存在时自动创建中间层。

```bash
jed set .name '"Bob"' config.json
# 输出：ok

jed set .server.host '"127.0.0.1"' config.json
# 若 .server 不存在，自动创建对象

# 值可以是任意 JSON 类型
jed set .count 42 config.json
jed set .enabled true config.json
jed set .data null config.json
jed set .config '{"timeout": 30}' config.json
```

---

### `del <路径> [文件]`

删除指定路径的 key 或数组元素。

```bash
jed del .name config.json
# 输出：ok

jed del .servers[1] config.json
# 删除数组中的第 2 个元素
```

---

### `add [路径] <值> [文件]`

向数组末尾追加元素，或向对象合并新字段。

```bash
# 追加到数组
jed add .tags '"golang"' config.json
# 输出：ok

# 合并到对象（已存在的 key 会被覆盖）
jed add .server '{"timeout": 30, "retry": 3}' config.json
```

---

### `mv <源路径> <目标路径> [文件]`

移动或重命名 key。

```bash
jed mv .oldName .newName config.json
# 输出：ok
```

---

### `patch <json> [文件]`

一次性批量操作，减少 Agent 的调用次数。格式遵循 JSON Patch（RFC 6902），所有操作原子执行。

支持操作：`add`、`remove`、`replace`、`move`、`copy`、`test`

```bash
jed patch '[
  {"op": "replace", "path": ".name",    "value": "Bob"},
  {"op": "replace", "path": ".version", "value": 2},
  {"op": "remove",  "path": ".legacy"},
  {"op": "add",     "path": ".tags/-",  "value": "new"}
]' config.json
# 输出：patched 4 ops
```

注：数组末尾追加使用 `path/-` 后缀（RFC 6902 规范）。

---

## 格式化/修复命令

### `fmt [文件]`

原地美化格式化 JSON 文件。

```bash
jed fmt config.json
# 输出：ok

jed fmt --indent 4 config.json
# 使用 4 空格缩进
```

---

### `fix [文件]`

自动检测并修复常见 JSON 格式错误，然后格式化。

```bash
jed fix broken.json
# 输出：fixed 3 errors
#   第 12 行：移除尾部逗号
#   第 18 行：单引号替换为双引号

jed fix --dry-run broken.json
# 预览将修复的内容，不实际写入文件

jed fix --strip-comments file.jsonc
# 修复并剥离注释（不加此选项时，注释会导致报错退出）
```

---

### `minify [文件]`

原地压缩 JSON，移除所有空白。

```bash
jed minify config.json
# 输出：ok
```

---

## 转换命令

### `convert <格式> [文件]`

将 JSON 转换为其他格式输出（不修改原文件）。

```bash
jed convert yaml config.json
# 输出 YAML 格式内容到 stdout
```

当前支持的格式：`yaml`

> TOML 转换尚未实现。

---

### `validate <schema文件> [文件]`

根据 JSON Schema 文件校验数据文件。

```bash
jed validate schema.json data.json
# 校验通过：输出 {"valid": true, ...}
# 校验失败：输出缺少的必填字段，exit 1
```

> 当前仅检查 `required` 字段的存在性，完整 JSON Schema 验证尚未实现。

---

## 发现命令

### `commands`

列出所有可用命令及其说明。

```bash
jed commands
jed --json commands   # 输出 JSON 格式
```

---

### `explain <命令名>`

显示指定命令的详细帮助和示例。

```bash
jed explain set
jed explain patch
```

---

### `completions <shell>`

生成 Shell 补全脚本。

```bash
# Bash
jed completions bash > ~/.bash_completion.d/jed

# Zsh
jed completions zsh > ~/.zsh/completions/_jed

# Fish
jed completions fish > ~/.config/fish/completions/jed.fish
```

支持的 shell：`bash`、`zsh`、`fish`、`powershell`、`elvish`

---

## Agent 使用示例

### 场景：修改 Claude Code 的 MCP 配置

```bash
# 1. 先了解文件结构（不读取全部内容）
jed schema ~/.claude/settings.json
# 输出：{mcpServers: {[name]: {command: string, args: [string], env: {}}}, defaultMode: string}

# 2. 检查某个 MCP server 是否已存在
jed exists .mcpServers.github ~/.claude/settings.json
# exit 0 → 已存在

# 3. 只读取需要的那个值
jed get .mcpServers.github.command ~/.claude/settings.json
# 输出：/usr/local/bin/gh-mcp

# 4. 更新单个字段
jed set .mcpServers.github.env.TOKEN '"ghp_xxxx"' ~/.claude/settings.json
# 输出：ok

# 5. 批量更新多个字段（一次调用）
jed patch '[
  {"op": "replace", "path": ".defaultMode",                  "value": "acceptEdits"},
  {"op": "add",     "path": ".mcpServers.github.enabled",    "value": true}
]' ~/.claude/settings.json
# 输出：patched 2 ops
```

---

## 退出码参考

| 退出码 | 含义 | 典型场景 |
|--------|------|----------|
| 0 | 成功 | 所有正常操作 |
| 1 | 通用错误 | JSON 无效、写入权限不足 |
| 2 | 路径未找到 | `get`/`del`/`exists` 找不到路径 |
| 3 | 类型不匹配 | 对非数组路径执行 `add` 追加操作 |
