# Jed — JSON Config Editor

Edit JSON configs without the struggle — **interactive TUI for humans**, **agent-optimized CLI for AI**.

- **Humans**: Visual tree navigation, inline editing, syntax highlighting, auto-repair
- **AI Agents**: Minimal-token output, crash-safe writes, batch operations

Same binary. Two modes. One engine.

[![CI](https://github.com/caoergou/jed/actions/workflows/ci.yml/badge.svg)](https://github.com/caoergou/jed/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

---

## Quick Start

```bash
# TUI mode (human)
jed config.json

# Command mode (agent / script)
jed get .name config.json
jed set .name '"Bob"' config.json
jed fix --strip-comments config.json

# Both argument orders are supported
jed config.json get .name
jed get .name config.json
```

---

## Installation

### Pre-built binaries (recommended)

Download from the [Releases](https://github.com/caoergou/jed/releases) page, or use the install script:

```bash
# Linux / macOS — auto-detects platform and installs to /usr/local/bin
curl -fsSL https://github.com/caoergou/jed/releases/latest/download/install.sh | sh
```

### From crates.io

```bash
cargo install jed-json
```

### From source (requires Rust)

```bash
cargo install --git https://github.com/caoergou/jed
```

| Platform | Binary |
|----------|--------|
| Linux x86_64 | `jed-linux-x86_64` |
| Linux aarch64 | `jed-linux-aarch64` |
| macOS x86_64 | `jed-macos-x86_64` |
| macOS Apple Silicon | `jed-macos-aarch64` |
| Windows x86_64 | `jed-windows-x86_64.exe` |

Place the binary somewhere on your `$PATH` and rename it to `jed`.

---

## TUI Mode

Launch by passing only a filename:

```bash
jed settings.json
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
jed get .key file.json              # get value at path
jed get '.servers[0].host' file.json
jed keys . file.json                # list all top-level keys
jed len .tags file.json             # array / object length
jed type .count file.json           # type name: string|number|boolean|null|object|array
jed exists .key file.json           # exit 0=exists, 2=not found
jed schema file.json                # infer structure (no values)
jed check file.json                 # validate; errors to stderr
```

### Write

```bash
jed set .name '"Bob"' file.json     # set value
jed del .legacy file.json           # delete key
jed add .tags '"go"' file.json      # append to array
jed mv .oldKey .newKey file.json    # rename key

# Batch (JSON Patch RFC 6902) — one call, atomic
jed patch '[
  {"op": "replace", "path": ".name",    "value": "Bob"},
  {"op": "add",     "path": ".tags/-",  "value": "go"},
  {"op": "remove",  "path": ".legacy"}
]' file.json
```

### Format / Repair

```bash
jed fmt file.json                   # pretty-print in place
jed fix --strip-comments file.json  # auto-fix JSONC, trailing commas, etc.
jed fix --dry-run file.json         # preview repairs without writing
jed minify file.json                # compact JSON
jed diff old.json new.json          # structural diff
```

### Inspect / Convert

```bash
jed tree file.json                  # display as indented tree
jed tree -e file.json               # expand all nodes
jed tree -p .servers file.json      # tree view of a sub-path
jed query '.users[0]' file.json     # alias for get, with path-filter semantics
jed validate schema.json file.json  # validate against JSON Schema
jed convert yaml file.json          # convert to YAML
jed convert toml file.json          # convert to TOML
```

### Discovery

```bash
jed commands                        # list all available commands
jed explain get                     # detailed help for a specific command
jed completions bash                # generate shell completion script
jed completions zsh
jed completions fish
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

## Why Jed for AI Agents?

| Traditional approach | jed command mode |
|----------------------|-----------------|
| Read full file into context | `get .key` → only the target value |
| Rewrite full file after change | `set .key val` → returns `ok` |
| Agent parses JSON manually | Path addressing handles navigation |
| Agent retries on format errors | `fix` repairs errors automatically |
| Multiple round-trips | `patch` applies batch changes in one call |

### Example: configure Claude Code MCP server

```bash
# 1. Check file structure without reading values
jed schema ~/.claude/settings.json

# 2. Check if a server exists
jed exists .mcpServers.github ~/.claude/settings.json

# 3. Read only the specific value needed
jed get .mcpServers.github.command ~/.claude/settings.json

# 4. Update a single field
jed set .mcpServers.github.env.TOKEN '"ghp_xxxx"' ~/.claude/settings.json

# 5. Batch update (one call)
jed patch '[
  {"op": "replace", "path": ".defaultMode", "value": "acceptEdits"},
  {"op": "add",     "path": ".mcpServers.github.enabled", "value": true}
]' ~/.claude/settings.json
```

---

## Auto-fix Capabilities

`jed fix` repairs the most common JSON format errors found in the wild:

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
cat config.json | jed get .name
echo '{"a":1}' | jed schema
```

---

## Building from Source

```bash
git clone https://github.com/caoergou/jed
cd jed
cargo build --release
./target/release/jed --version
```

---

---

## Shell Completions

Enable tab-completion for `jed` commands and options.

### Bash

```bash
# Write to bash-completion directory (recommended)
jed completions bash > ~/.local/share/bash-completion/completions/jed

# Or add to a custom directory and source it
jed completions bash > ~/.bash_completion.d/jed
echo 'source ~/.bash_completion.d/jed' >> ~/.bashrc
```

### Zsh

```bash
# Write to fpath directory
mkdir -p ~/.zfunc
jed completions zsh > ~/.zfunc/_jed

# Add to .zshrc (before any compinit call):
# fpath=(~/.zfunc $fpath)

# Reload shell
exec zsh
```

### Other Shells

Fish, PowerShell, and Elvish are also supported. See [CLI_SPEC.md](CLI_SPEC.md#completions-shell) for details.

---

## Roadmap

### v1.x — Polish & Distribution
- [x] Shell completions documentation and testing (bash/zsh/fish)
- [x] `diff --json` structured output mode
- [x] TOML conversion (`jed convert toml`)
- [x] Full JSON Schema validation (`type`, `required`, `properties`, `minimum`, `maximum`, `minLength`, `maxLength`, `minItems`, `maxItems`, `items`, `enum`)
- [ ] Package manager distribution: Homebrew, apt/deb, rpm

### v2.x — Power Features
- [ ] Interactive shell mode (`jed shell`) — persistent REPL for batch edits without reopening files
- [ ] JSONC comment preservation on save (CST-based; currently stripped on write)
- [ ] TUI mouse support
- [ ] Large file optimization (virtual scroll for files > 1 MB)

### v3.x — Long Term
- [ ] Multi-file tabs in TUI
- [ ] Watch mode: reload TUI on external file change
- [ ] JSON Pointer (RFC 6901) as alternative path syntax

---

## License

MIT
