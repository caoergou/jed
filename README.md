# je — JSON Editor

A dual-interface JSON tool built for **humans and AI agents** alike.

- **Humans** get a full TUI with tree navigation, inline editing, and syntax highlighting.
- **AI Agents** get path-based CLI commands with minimal token output.

Same binary. Two modes. One engine.

[![CI](https://github.com/EricCaoGH/je/actions/workflows/ci.yml/badge.svg)](https://github.com/EricCaoGH/je/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

---

## Quick Start

```bash
# TUI mode (human)
je config.json

# Command mode (agent / script)
je config.json get .name
je config.json set .name '"Bob"'
je config.json fix --strip-comments
```

---

## Installation

### From source (requires Rust)

```bash
cargo install --git https://github.com/EricCaoGH/je
```

### Pre-built binaries

Download from the [Releases](https://github.com/EricCaoGH/je/releases) page:

| Platform | Binary |
|----------|--------|
| Linux x86_64 | `je-linux-x86_64` |
| Linux aarch64 | `je-linux-aarch64` |
| macOS x86_64 | `je-macos-x86_64` |
| macOS Apple Silicon | `je-macos-aarch64` |
| Windows x86_64 | `je-windows-x86_64.exe` |

---

## TUI Mode

Launch by passing only a filename:

```bash
je settings.json
```

| Key | Action |
|-----|--------|
| `j` / `↓` | Move cursor down |
| `k` / `↑` | Move cursor up |
| `l` / `→` | Expand node |
| `h` / `←` | Collapse node |
| `e` | Edit current value |
| `d` | Delete current node |
| `u` | Undo |
| `ctrl+r` | Redo |
| `ctrl+s` | Save |
| `q` | Quit |

---

## Command Mode

Designed for **AI agents** to read and write JSON with minimal token usage.

### Read

```bash
je file.json get .key              # get value at path
je file.json get '.servers[0].host'
je file.json keys .                # list all top-level keys
je file.json len .tags             # array / object length
je file.json type .count           # type name: string|number|boolean|null|object|array
je file.json exists .key           # exit 0=exists, 2=not found
je file.json schema                # infer structure (no values)
je file.json check                 # validate; errors to stderr
```

### Write

```bash
je file.json set .name '"Bob"'     # set value
je file.json del .legacy           # delete key
je file.json add .tags '"go"'      # append to array
je file.json mv .oldKey .newKey    # rename key

# Batch (JSON Patch RFC 6902) — one call, atomic
je file.json patch '[
  {"op": "replace", "path": ".name",    "value": "Bob"},
  {"op": "add",     "path": ".tags/-",  "value": "go"},
  {"op": "remove",  "path": ".legacy"}
]'
```

### Format / Repair

```bash
je file.json fmt                   # pretty-print in place
je file.json fix --strip-comments  # auto-fix JSONC, trailing commas, etc.
je file.json fix --dry-run         # preview repairs without writing
je file.json minify                # compact JSON
je file.json diff other.json       # structural diff
```

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Path not found |
| 3 | Type mismatch |

---

## Why je for AI Agents?

| Traditional approach | je command mode |
|----------------------|-----------------|
| Read full file into context | `get .key` → only the target value |
| Rewrite full file after change | `set .key val` → returns `ok` |
| Agent parses JSON manually | Path addressing handles navigation |
| Agent retries on format errors | `fix` repairs errors automatically |
| Multiple round-trips | `patch` applies batch changes in one call |

### Example: configure Claude Code MCP server

```bash
# 1. Check file structure without reading values
je ~/.claude/settings.json schema

# 2. Check if a server exists
je ~/.claude/settings.json exists .mcpServers.github

# 3. Read only the specific value needed
je ~/.claude/settings.json get .mcpServers.github.command

# 4. Update a single field
je ~/.claude/settings.json set .mcpServers.github.env.TOKEN '"ghp_xxxx"'

# 5. Batch update (one call)
je ~/.claude/settings.json patch '[
  {"op": "replace", "path": ".defaultMode", "value": "acceptEdits"},
  {"op": "add",     "path": ".mcpServers.github.enabled", "value": true}
]'
```

---

## Auto-fix Capabilities

`je fix` repairs the most common JSON format errors found in the wild:

| Error | Example | Fix |
|-------|---------|-----|
| Trailing comma | `{"a": 1,}` | Remove |
| Single quotes | `{'key': 'val'}` | Replace with double quotes |
| Unquoted keys | `{key: "val"}` | Add quotes |
| Missing comma | `{"a": 1 "b": 2}` | Insert comma |
| Line comments | `// comment` | Strip |
| Block comments | `/* comment */` | Strip |
| Python literals | `True`, `False`, `None` | Replace with JSON equivalents |
| BOM | Leading `\uFEFF` | Strip |

---

## Path Syntax

Uses jq-inspired path syntax:

```
.                      # root
.key                   # object field
.key.nested            # nested field
.array[0]              # array index
.array[-1]             # last element
.key.array[2].field    # deep path
```

---

## Building from Source

```bash
git clone https://github.com/EricCaoGH/je
cd je
cargo build --release
./target/release/je --version
```

---

## License

MIT
