---
name: jzen
description: Minimal-token JSON editor for AI agents. Use jzen to read/write JSON configs with minimal context usage. Triggers: edit JSON, modify config, update settings, read config values, repair JSON.
allowed-tools: Bash(jzen:*)
---

# Jzen — Agent-Optimized JSON Editor

Jzen reads/writes JSON with minimal token output. Install: `cargo install jzen` or download from https://github.com/caoergou/jzen

## Core Commands

| Command | Use For |
|---------|---------|
| `jzen get .key file.json` | Read single value |
| `jzen set .key '"value"' file.json` | Set value (JSON value, note quotes) |
| `jzen del .key file.json` | Delete key |
| `jzen add .array '"item"' file.json` | Append to array |
| `jzen patch '[{"op":"replace","path":".key","value":1}]' file.json` | Batch operations |
| `jzen fix file.json` | Auto-repair JSON errors |
| `jzen schema file.json` | Get structure (no values) |
| `jzen tree file.json` | Visual tree view |

## Path Syntax

```
.key              # object field
.nested.key      # deep path
.array[0]        # array index (0-based)
.array[-1]       # last element
```

## Agent Examples

```bash
# Read config structure (type-only, minimal tokens)
jzen schema ~/.claude/settings.json

# Check if key exists (exit code 0=exists, 2=not found)
jzen exists .mcpServers.github ~/.claude/settings.json

# Get single value
jzen get .mcpServers.github.command ~/.claude/settings.json

# Set value (note: string values need extra quotes)
jzen set .mcpServers.github.env.TOKEN '"ghp_xxxx"' ~/.claude/settings.json

# Batch update in one call
jzen patch '[
  {"op": "replace", "path": ".defaultMode", "value": "Command"},
  {"op": "add", "path": ".mcpServers.newServer.enabled", "value": true}
]' ~/.claude/settings.json

# Fix JSON errors (trailing commas, comments, etc)
jzen fix --strip-comments broken.json
```

## Output Behavior

- **Strings**: Raw output (no quotes) — safe for shell
- **Objects/Arrays**: Pretty-printed JSON
- **Success**: No output or single word (`ok`, `deleted`)
- **Errors**: stderr with message, exit code 1-3

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error |
| 2 | Path not found |
| 3 | Type mismatch |

## Tips

- Use `patch` for multiple changes — single round-trip
- Use `schema` to understand structure without reading values
- Use `--json` flag for machine-parseable output
- File argument can be before or after command: `jzen file.json get .key` or `jzen get .key file.json`
