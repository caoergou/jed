# Jzen — JSON Config Editor

Edit JSON configs without the struggle — **interactive TUI for humans**, **agent-optimized CLI for AI**.

- **Humans**: Visual tree navigation, inline editing, syntax highlighting, auto-repair
- **AI Agents**: Minimal-token output, crash-safe writes, batch operations

Same binary. Two modes. One engine.

---

## Why Jzen?

### Problem 1: Editing Claude Code MCP Configuration

When you need to modify Claude Code's `settings.json` for MCP servers, the traditional approach forces you to:

- Load the entire config file into your context window (often 100+ lines)
- Manually locate the field you need to change
- Rewrite the entire file after modification
- **Result**: High token consumption, error-prone manual editing

With Jzen, you only pay for what you change:

```bash
# Inspect structure without reading values
jzen schema ~/.claude/settings.json

# Get only the specific value you need
jzen get .mcpServers.github.command ~/.claude/settings.json

# Update a single field atomically
jzen set .mcpServers.github.env.TOKEN '"ghp_xxxx"' ~/.claude/settings.json

# Batch update in one call (minimal round-trips)
jzen patch '[
  {"op": "replace", "path": ".defaultMode", "value": "acceptEdits"},
  {"op": "add", "path": ".mcpServers.github.enabled", "value": true}
]' ~/.claude/settings.json
```

**Token savings**: 90%+ reduction — you read only what you query, not the entire file.

---

### Problem 2: Editing OpenClaw Agent Configuration

OpenClaw stores agent behavior in JSON config. Traditional tools require:

- Opening the entire file to understand its structure
- Manually editing and saving the full file
- Risking format errors that break the agent

Jzen makes this seamless:

```bash
# Visualize structure at a glance
jzen tree ~/.config/openclaw/agent.json

# Update model configuration
jzen set .model.provider '"openai"' ~/.config/openclaw/agent.json
jzen set .model.name '"gpt-4o"' ~/.config/openclaw/agent.json

# Add new MCP server
jzen set .mcpServers.github '{
  "command": "npx",
  "args": ["-y", "@modelcontextprotocol/server-github"]
}' ~/.config/openclaw/agent.json

# Auto-fix common JSON errors
jzen fix --strip-comments ~/.config/openclaw/agent.json
```

**Benefits**: Atomic writes, crash-safe operation, automatic format repair.

---

## Comparison: Traditional vs Jzen

| Task | Traditional Approach | Jzen |
|------|----------------------|------|
| Read config structure | Load entire file | `schema` → type-only output |
| Read specific value | Parse full JSON | `get .key` → single value |
| Modify one field | Rewrite entire file | `set .key val` → atomic |
| Multiple changes | Multiple round-trips | `patch` → single call |
| Repair JSON errors | Manual fixing | `fix` → auto-repair |
| Token cost | Full file in context | Only queried values |

[![CI](https://github.com/caoergou/jzen/actions/workflows/ci.yml/badge.svg)](https://github.com/caoergou/jzen/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

---

## Quick Start

```bash
# TUI mode (human)
jzen config.json

# Command mode (agent / script)
jzen get .name config.json
jzen set .name '"Bob"' config.json
jzen fix --strip-comments config.json

# Both argument orders are supported
jzen config.json get .name
jzen get .name config.json
```

---

## Installation

### Package managers (recommended)

```bash
# macOS / Linux (Homebrew)
brew install caoergou/jzen/jzen

# Debian / Ubuntu
sudo dpkg -i jzen_*.deb

# Fedora / RHEL / CentOS
sudo rpm -i jzen-*.rpm
```

### Install script

```bash
# Linux / macOS — auto-detects platform, shell, and installs with completions
curl -fsSL https://github.com/caoergou/jzen/releases/latest/download/install.sh | sh

# Skip auto-installing shell completions
SKIP_COMPLETIONS=1 curl -fsSL https://github.com/caoergou/jzen/releases/latest/download/install.sh | sh

# Custom install directory
INSTALL_DIR=~/.local/bin curl -fsSL https://github.com/caoergou/jzen/releases/latest/download/install.sh | sh
```

The install script will:
1. Download the correct binary for your platform
2. Detect your shell (bash/zsh/fish)
3. Auto-install shell completions to the appropriate location
4. Print completion instructions if manual setup is needed

### Pre-built binaries

Download from the [Releases](https://github.com/caoergou/jzen/releases) page:

| Platform | Binary |
|----------|--------|
| Linux x86_64 | `jzen-linux-x86_64` |
| Linux aarch64 | `jzen-linux-aarch64` |
| macOS x86_64 | `jzen-macos-x86_64` |
| macOS Apple Silicon | `jzen-macos-aarch64` |
| Windows x86_64 | `jzen-windows-x86_64.exe` |

Place the binary somewhere on your `$PATH`.

### From crates.io

```bash
cargo install jzen
```

### From source (requires Rust)

```bash
cargo install --git https://github.com/caoergou/jzen
```

---

## Agent Skill

Install the jzen skill to enable AI agents to edit JSON with minimal token usage:

```bash
# Install for Claude Code, OpenClaw, Codex, etc.
npx skills add caoergou/jzen

# Or install a specific skill from this repo
npx skills add caoergou/jzen --skill jzen
```

After installation, the agent will automatically use jzen for JSON operations, reducing token consumption by 90%+.

---

## TUI Mode

Launch by passing only a filename:

```bash
jzen settings.json
```

| Key | Action |
|-----|--------|
| `↑/↓` | Move up/down |
| `←` | Collapse / go to parent |
| `→` / `Space` | Expand / toggle |
| `Enter` | Edit leaf node / expand container |
| `N` / `Insert` | Add new node |
| `Delete` | Delete current node |
| `Ctrl+S` | Save |
| `Ctrl+F` / `/` | Search |
| `Ctrl+Z` | Undo |
| `Ctrl+Y` | Redo |
| `F1` | Help |
| `q` | Quit (prompts if unsaved) |

---

## Command Mode

Designed for **AI agents** to read and write JSON with minimal token usage.

### Read

```bash
jzen get .key file.json              # get value at path
jzen get '.servers[0].host' file.json
jzen keys . file.json                # list all top-level keys
jzen len .tags file.json             # array / object length
jzen type .count file.json           # type name: string|number|boolean|null|object|array
jzen exists .key file.json           # exit 0=exists, 2=not found
jzen schema file.json                # infer structure (no values)
jzen check file.json                 # validate; errors to stderr
```

### Write

```bash
jzen set .name '"Bob"' file.json     # set value
jzen del .legacy file.json           # delete key
jzen add .tags '"go"' file.json      # append to array
jzen mv .oldKey .newKey file.json    # rename key

# Batch (JSON Patch RFC 6902) — one call, atomic
jzen patch '[
  {"op": "replace", "path": ".name",    "value": "Bob"},
  {"op": "add",     "path": ".tags/-",  "value": "go"},
  {"op": "remove",  "path": ".legacy"}
]' file.json
```

### Format / Repair

```bash
jzen fmt file.json                   # pretty-print in place
jzen fix --strip-comments file.json  # auto-fix JSONC, trailing commas, etc.
jzen fix --dry-run file.json         # preview repairs without writing
jzen minify file.json                # compact JSON
jzen diff old.json new.json          # structural diff
```

### Inspect / Convert

```bash
jzen tree file.json                  # display as indented tree
jzen tree -e file.json               # expand all nodes
jzen tree -p .servers file.json      # tree view of a sub-path
jzen query '.users[0]' file.json     # alias for get, with path-filter semantics
jzen validate schema.json file.json  # validate against JSON Schema
jzen convert yaml file.json          # convert to YAML
jzen convert toml file.json          # convert to TOML
```

### Discovery

```bash
jzen commands                        # list all available commands
jzen explain get                     # detailed help for a specific command
jzen completions bash                # generate shell completion script
jzen completions zsh
jzen completions fish
```

### Global options

| Option | Description |
|--------|-------------|
| `--json` | Wrap all output as `{"ok":...,"value":...}` |
| `--lang <lang>` | Output language: `en`, `zh-CN`, `zh-TW` |
| `--quiet` | Suppress informational output |
| `-h, --help` | Show help |
| `-V, --version` | Show version |

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Path not found |
| 3 | Type mismatch |

---

## Why Jzen for AI Agents?

| Traditional approach | jzen command mode |
|----------------------|-----------------|
| Read full file into context | `get .key` → only the target value |
| Rewrite full file after change | `set .key val` → returns `ok` |
| Agent parses JSON manually | Path addressing handles navigation |
| Agent retries on format errors | `fix` repairs errors automatically |
| Multiple round-trips | `patch` applies batch changes in one call |

### Example: configure Claude Code MCP server

```bash
# 1. Check file structure without reading values
jzen schema ~/.claude/settings.json

# 2. Check if a server exists
jzen exists .mcpServers.github ~/.claude/settings.json

# 3. Read only the specific value needed
jzen get .mcpServers.github.command ~/.claude/settings.json

# 4. Update a single field
jzen set .mcpServers.github.env.TOKEN '"ghp_xxxx"' ~/.claude/settings.json

# 5. Batch update (one call)
jzen patch '[
  {"op": "replace", "path": ".defaultMode", "value": "acceptEdits"},
  {"op": "add",     "path": ".mcpServers.github.enabled", "value": true}
]' ~/.claude/settings.json
```

### Example: configure OpenClaw agent

```bash
# 1. Inspect the config structure
jzen tree ~/.config/openclaw/agent.json

# 2. Update the model configuration
jzen set .model.provider '"openai"' ~/.config/openclaw/agent.json
jzen set .model.name '"gpt-4o"' ~/.config/openclaw/agent.json

# 3. Add a new MCP server
jzen set .mcpServers.github '{
  "command": "npx",
  "args": ["-y", "@modelcontextprotocol/server-github"]
}' ~/.config/openclaw/agent.json

# 4. Fix and format before saving
jzen fix --strip-comments ~/.config/openclaw/agent.json
```

---

## Auto-fix Capabilities

`jzen fix` repairs the most common JSON format errors found in the wild:

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

## Stdin / Pipe Support

All read commands accept JSON from stdin when no file argument is given:

```bash
cat config.json | jzen get .name
echo '{"a":1}' | jzen schema
```

---

## Building from Source

```bash
git clone https://github.com/caoergou/jzen
cd jzen
cargo build --release
./target/release/jzen --version
```

---

---

## Shell Completions

Enable tab-completion for `jzen` commands and options.

### Bash

```bash
# Write to bash-completion directory (recommended)
jzen completions bash > ~/.local/share/bash-completion/completions/jzen

# Or add to a custom directory and source it
jzen completions bash > ~/.bash_completion.d/jzen
echo 'source ~/.bash_completion.d/jzen' >> ~/.bashrc
```

### Zsh

```bash
# Write to fpath directory
mkdir -p ~/.zfunc
jzen completions zsh > ~/.zfunc/_jzen

# Add to .zshrc (before any compinit call):
# fpath=(~/.zfunc $fpath)

# Reload shell
exec zsh
```

### Other Shells

Fish, PowerShell, and Elvish are also supported. See [CLI_SPEC.md](CLI_SPEC.md#completions-shell) for details.

---

## Roadmap

### v1.x — ✅ Complete (Polished & Distributed)
- [x] Shell completions (bash/zsh/fish/powershell/elvish)
- [x] `diff --json` structured output mode
- [x] TOML conversion (`jzen convert toml`)
- [x] Full JSON Schema validation (`type`, `required`, `properties`, `minimum`, `maximum`, `minLength`, `maxLength`, `minItems`, `maxItems`, `items`, `enum`)
- [x] Package manager distribution: Homebrew, apt/deb, rpm
- [x] YAML conversion (`jzen convert yaml`)
- [x] File watching in TUI mode

### v2.x — Power Features (In Progress)
- [ ] Interactive shell mode (`jzen shell`) — persistent REPL for batch edits without reopening files
- [ ] JSONC comment preservation on save (CST-based; currently stripped on write)
- [ ] TUI mouse support
- [ ] Large file optimization (virtual scroll for files > 1 MB)

### v3.x — Long Term
- [ ] Multi-file tabs in TUI
- [ ] JSON Pointer (RFC 6901) as alternative path syntax

---

## License

MIT
