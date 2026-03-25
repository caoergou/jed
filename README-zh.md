# Jzen — JSON 配置编辑器

[English Version](./README.md)

JSON 编辑器，**TUI 面向人类**，**CLI 面向 AI Agent**。

[![CI](https://github.com/caoergou/jzen/actions/workflows/ci.yml/badge.svg)](https://github.com/caoergou/jzen/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

---

## 为什么需要 Jzen？

### 问题：编辑 JSON 配置

传统方式：加载完整文件 → 查找字段 → 重写整个文件

使用 Jzen：
```bash
jzen schema config.json                    # 仅结构（不含值）
jzen get .database.host config.json       # 单个值
jzen set .database.port 5432 config.json  # 原子更新
jzen patch '[{"op":"add","path":".tags","value":["prod"]}]' config.json
```

**vs jq**: jq 是查询语言，jzen 是编辑器。jq 读取全部 → 过滤 → 输出；jzen 只读取你查询的内容。

---

## 快速开始

```bash
# TUI（人类）
jzen config.json

# CLI（Agent）
jzen get .name config.json
jzen set .name '"Bob"' config.json
jzen fix --strip-comments config.json
```

---

## 安装

```bash
# 一行命令（自动安装补全）
curl -fsSL https://github.com/caoergou/jzen/releases/latest/download/install.sh | sh

# 或 Homebrew
brew install caoergou/jzen/jzen
```

---

## 命令

| 命令 | 描述 |
|------|------|
| `get .key f.json` | 读取值 |
| `set .key val f.json` | 设置值 |
| `del .key f.json` | 删除 |
| `add .arr val f.json` | 追加到数组 |
| `patch '[...]' f.json` | 批量（原子） |
| `schema f.json` | 仅结构 |
| `tree f.json` | 可视化树 |
| `fix f.json` | 自动修复 JSON |
| `fmt f.json` | 美化 |
| `convert yaml f.json` | 转换为 YAML/TOML |

路径: `.key`, `.arr[0]`, `.arr[-1]`, `.a.b.c`

---

## Agent Skill

```bash
npx skills add caoergou/jzen
```

---

## TUI 按键

| 按键 | 操作 |
|------|------|
| `↑/↓` | 导航 |
| `Enter` | 编辑 |
| `N` | 添加节点 |
| `Delete` | 删除 |
| `Ctrl+S` | 保存 |
| `q` | 退出 |

---

## 许可证

MIT