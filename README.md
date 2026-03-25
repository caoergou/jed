# Jzen — JSON Config Editor

[中文版本](./README-zh.md)

JSON editor with **TUI for humans** and **CLI for AI agents**.

[![CI](https://github.com/caoergou/jzen/actions/workflows/ci.yml/badge.svg)](https://github.com/caoergou/jzen/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

---

## Why Jzen?

### Problem 1: Claude Code MCP Config

Traditional: load entire file → find field → rewrite full file → high token cost

With Jzen:
```bash
jzen schema ~/.claude/settings.json        # structure only
jzen get .mcpServers.github.command ~      # single value
jzen set .mcpServers.github.env.TOKEN '"xxx"' ~
jzen patch '[{"op":"replace",path:".defaultMode",value:"Command"}]' ~
```
**Token savings: 90%+**

### Problem 2: OpenClaw Agent Config

Traditional: open full file → manual edit → risk format errors

With Jzen:
```bash
jzen tree ~/.config/openclaw/agent.json
jzen set .model.provider '"openai"' ~/.config/openclaw/agent.json
jzen fix --strip-comments ~/.config/openclaw/agent.json
```
**Atomic writes, auto-repair**

---

## Quick Start

```bash
# TUI (human)
jzen config.json

# CLI (agent)
jzen get .name config.json
jzen set .name '"Bob"' config.json
jzen fix --strip-comments config.json
```

---

## Install

```bash
# One-liner (auto-installs completions)
curl -fsSL https://github.com/caoergou/jzen/releases/latest/download/install.sh | sh

# Or Homebrew
brew install caoergou/jzen/jzen
```

---

## Commands

| Command | Description |
|---------|-------------|
| `get .key f.json` | Read value |
| `set .key val f.json` | Set value |
| `del .key f.json` | Delete |
| `add .arr val f.json` | Append to array |
| `patch '[...]' f.json` | Batch (atomic) |
| `schema f.json` | Structure only |
| `tree f.json` | Visual tree |
| `fix f.json` | Auto-repair JSON |
| `fmt f.json` | Pretty-print |
| `convert yaml f.json` | To YAML/TOML |

Path: `.key`, `.arr[0]`, `.arr[-1]`, `.a.b.c`

---

## Agent Skill

```bash
npx skills add caoergou/jzen
```

---

## TUI Keys

| Key | Action |
|-----|--------|
| `↑/↓` | Navigate |
| `Enter` | Edit |
| `N` | Add node |
| `Delete` | Delete |
| `Ctrl+S` | Save |
| `q` | Quit |

---

## License

MIT