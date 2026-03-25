# Jzen — JSON 配置编辑器

[English Version](./README.md)

面向人类的 TUI 编辑器，面向 AI Agent 的 CLI 工具。

[![CI](https://github.com/caoergou/jzen/actions/workflows/ci.yml/badge.svg)](https://github.com/caoergou/jzen/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

---

## Jzen 的优势

传统方式：读取完整文件 → 手动定位 → 重写整个文件

使用 Jzen：
```bash
jzen schema config.json                    # 仅返回结构体，不含具体值
jzen get .database.host config.json       # 读取单个值
jzen set .database.port 5432 config.json  # 原子写入
jzen patch '[{"op":"add","path":".tags","value":["prod"]}]' config.json
```

**与 jq 的区别**：jq 是查询语言，Jzen 是编辑器。jq 读取全部内容 → 过滤 → 输出；Jzen 只返回你查询的那部分。

---

## 快速上手

```bash
# TUI 模式（人类用户）
jzen config.json

# CLI 模式（AI Agent）
jzen get .name config.json
jzen set .name '"Bob"' config.json
jzen fix --strip-comments config.json
```

---

## 安装

```bash
# 一行命令安装（自动配置 shell 补全）
curl -fsSL https://github.com/caoergou/jzen/releases/latest/download/install.sh | sh

# 或使用 Homebrew
brew install caoergou/jzen/jzen
```

---

## 命令一览

| 命令 | 用途 |
|------|------|
| `get .key f.json` | 读取指定路径的值 |
| `set .key val f.json` | 设置值 |
| `del .key f.json` | 删除键 |
| `add .arr val f.json` | 追加到数组 |
| `patch '[...]' f.json` | 批量原子操作 |
| `schema f.json` | 返回结构体（不含值） |
| `tree f.json` | 树形可视化 |
| `fix f.json` | 自动修复 JSON 错误 |
| `fmt f.json` | 格式化 |
| `convert yaml f.json` | 转换为 YAML/TOML |

路径语法：`.key`、`.arr[0]`、`.arr[-1]`、`.a.b.c`

---

## Agent 技能

```bash
npx skills add caoergou/jzen
```

---

## TUI 快捷键

| 按键 | 功能 |
|------|------|
| `↑/↓` | 上下移动 |
| `Enter` | 编辑 |
| `N` | 添加节点 |
| `Delete` | 删除 |
| `Ctrl+S` | 保存 |
| `q` | 退出 |

---

## 许可证

MIT