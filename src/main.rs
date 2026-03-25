mod cli;
mod command;
mod engine;
mod i18n;
mod output;
mod tui;

use clap::{CommandFactory, Parser};
use std::io::{Read, Write};
use std::path::PathBuf;

use crate::{
    cli::{Cli, Command, resolve_file},
    output::Ctx,
};

fn main() {
    let cli = Cli::parse();

    // 设置语言环境
    if let Some(lang) = &cli.lang {
        // SAFETY: 单线程 CLI 应用中设置环境变量是安全的
        unsafe { std::env::set_var("JZEN_LANG", lang) };
    }

    let json = cli.json;

    match &cli.command {
        // 列出所有命令
        Some(Command::Commands) => {
            let ctx = Ctx::new("commands", json);
            print_commands(&ctx);
        }

        // 命令帮助
        Some(Command::Explain { command }) => {
            let ctx = Ctx::new("explain", json);
            print_command_help(command, &ctx);
        }

        // 补全脚本
        Some(Command::Completions { shell }) => {
            use clap_complete::generate;
            let mut cmd = Cli::command();
            generate(*shell, &mut cmd, "jed", &mut std::io::stdout());
        }

        // Tree 命令
        Some(Command::Tree {
            file,
            expand_all,
            path,
        }) => {
            let f = resolve_file(cli.get_file().as_ref(), Some(file));
            let f = resolve_input_file(f.as_ref());
            command::run_tree(&f, *expand_all, path.as_deref(), json);
        }

        // Query 命令
        Some(Command::Query { filter, file }) => {
            let f = resolve_file(cli.get_file().as_ref(), Some(file));
            let f = resolve_input_file(f.as_ref());
            command::run_query(&f, filter, json);
        }

        // Validate 命令
        Some(Command::Validate { schema, file }) => {
            let f = resolve_file(cli.get_file().as_ref(), Some(file));
            let f = resolve_input_file(f.as_ref());
            command::run_validate(&f, schema, json);
        }

        // Convert 命令
        Some(Command::Convert { format, file }) => {
            let f = resolve_file(cli.get_file().as_ref(), Some(file));
            let f = resolve_input_file(f.as_ref());
            command::run_convert(&f, format, json);
        }

        Some(cmd) => {
            let file = get_file_from_command(cmd, cli.get_file().as_ref());
            let file = resolve_input_file(Some(&file));
            command::run(&file, cmd.clone(), json);
        }

        None => {
            // 无命令时进入 TUI 模式
            let file = cli.get_file().inspect(|f| {
                if f.to_str() == Some("-") {
                    let locale = i18n::get_locale();
                    eprintln!("{}", i18n::t_to("main.need_file", &locale));
                    std::process::exit(1);
                }
            });

            if let Some(file) = file {
                if let Err(e) = tui::run_tui(file) {
                    let locale = i18n::get_locale();
                    eprintln!("{}: {e}", i18n::t_to("main.tui_error", &locale));
                    std::process::exit(1);
                }
            } else {
                let locale = i18n::get_locale();
                eprintln!("{}", i18n::t_to("main.need_file", &locale));
                std::process::exit(1);
            }
        }
    }
}

/// 从命令参数中提取文件路径
fn get_file_from_command(cmd: &Command, cli_file: Option<&PathBuf>) -> PathBuf {
    if let Some(f) = cli_file
        && f.to_str() != Some("-")
    {
        return f.clone();
    }

    match cmd {
        Command::Get { file, .. }
        | Command::Keys { file, .. }
        | Command::Len { file, .. }
        | Command::Type { file, .. }
        | Command::Exists { file, .. }
        | Command::Schema { file }
        | Command::Check { file }
        | Command::Set { file, .. }
        | Command::Del { file, .. }
        | Command::Add { file, .. }
        | Command::Patch { file, .. }
        | Command::Mv { file, .. }
        | Command::Fmt { file, .. }
        | Command::Fix { file, .. }
        | Command::Minify { file }
        | Command::Diff { file, .. }
        | Command::Tree { file, .. }
        | Command::Query { file, .. }
        | Command::Convert { file, .. }
        | Command::Validate { file, .. } => file.clone(),
        _ => PathBuf::from("-"),
    }
}

/// 解析输入文件，stdin 时写入唯一临时文件。
fn resolve_input_file(file: Option<&PathBuf>) -> PathBuf {
    let Some(f) = file else {
        eprintln!("Error: No input file specified");
        std::process::exit(1);
    };

    let f_str = f.to_str().unwrap_or("-");
    if f_str != "-" {
        return f.clone();
    }

    // 从 stdin 读取，写入唯一临时文件（避免多实例竞态）
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        eprintln!("Error: Failed to read from stdin");
        std::process::exit(1);
    }

    let mut tmp = match tempfile::NamedTempFile::new() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("Error: Failed to create temp file: {e}");
            std::process::exit(1);
        }
    };

    if tmp.write_all(input.as_bytes()).is_err() {
        eprintln!("Error: Failed to write to temp file");
        std::process::exit(1);
    }

    // keep() prevents the temp file from being deleted when `tmp` drops
    match tmp.keep() {
        Ok((_, path)) => path,
        Err(e) => {
            eprintln!("Error: Failed to persist temp file: {e}");
            std::process::exit(1);
        }
    }
}

// ── 命令发现 / 帮助 ───────────────────────────────────────────────────────────

/// 所有命令的元数据
struct CmdMeta {
    name: &'static str,
    usage: &'static str,
    description: &'static str,
    example: &'static str,
}

#[allow(clippy::too_many_lines)]
fn all_commands() -> Vec<CmdMeta> {
    vec![
        CmdMeta {
            name: "get",
            usage: "jed get <path> <file>",
            description: "Get value at path",
            example: "jed get .database.host config.json",
        },
        CmdMeta {
            name: "keys",
            usage: "jed keys [path] <file>",
            description: "List all keys or array indices at path",
            example: "jed keys .users config.json",
        },
        CmdMeta {
            name: "len",
            usage: "jed len [path] <file>",
            description: "Get array length or object key count",
            example: "jed len .items data.json",
        },
        CmdMeta {
            name: "type",
            usage: "jed type [path] <file>",
            description: "Get the type of a value",
            example: "jed type .version config.json",
        },
        CmdMeta {
            name: "exists",
            usage: "jed exists <path> <file>",
            description: "Check if a path exists (exit 0=yes, 2=no)",
            example: "jed exists .debug config.json",
        },
        CmdMeta {
            name: "schema",
            usage: "jed schema <file>",
            description: "Infer and display the structure of the file",
            example: "jed schema config.json",
        },
        CmdMeta {
            name: "check",
            usage: "jed check <file>",
            description: "Validate JSON syntax",
            example: "jed check config.json",
        },
        CmdMeta {
            name: "set",
            usage: "jed set <path> <value> <file>",
            description: "Set a value at path (creates if missing)",
            example: "jed set .debug true config.json",
        },
        CmdMeta {
            name: "del",
            usage: "jed del <path> <file>",
            description: "Delete a key or array element",
            example: "jed del .deprecated config.json",
        },
        CmdMeta {
            name: "add",
            usage: "jed add [path] <value> <file>",
            description: "Append to array or merge into object",
            example: "jed add .tags '\"beta\"' config.json",
        },
        CmdMeta {
            name: "patch",
            usage: "jed patch <operations> <file>",
            description: "Batch operations via JSON Patch (RFC 6902)",
            example: "jed patch '[{\"op\":\"replace\",\"path\":\".x\",\"value\":1}]' f.json",
        },
        CmdMeta {
            name: "mv",
            usage: "jed mv <src> <dst> <file>",
            description: "Move/rename a key",
            example: "jed mv .oldName .newName config.json",
        },
        CmdMeta {
            name: "fmt",
            usage: "jed fmt [--indent N] <file>",
            description: "Pretty-format JSON in-place",
            example: "jed fmt --indent 4 config.json",
        },
        CmdMeta {
            name: "fix",
            usage: "jed fix [--dry-run] [--strip-comments] <file>",
            description: "Auto-repair common JSON errors",
            example: "jed fix --dry-run broken.json",
        },
        CmdMeta {
            name: "minify",
            usage: "jed minify <file>",
            description: "Minify JSON (remove all whitespace)",
            example: "jed minify data.json",
        },
        CmdMeta {
            name: "diff",
            usage: "jed diff <other> <file>",
            description: "Compare two JSON files (exit 0=same, 1=diff)",
            example: "jed diff new.json old.json",
        },
        CmdMeta {
            name: "tree",
            usage: "jed tree [-e] [-p <path>] <file>",
            description: "Display JSON as a tree",
            example: "jed tree -e config.json",
        },
        CmdMeta {
            name: "query",
            usage: "jed query <filter> <file>",
            description: "Filter/query JSON using path expressions",
            example: "jed query .users[0] data.json",
        },
        CmdMeta {
            name: "validate",
            usage: "jed validate <schema> <file>",
            description: "Validate against a JSON Schema file",
            example: "jed validate schema.json data.json",
        },
        CmdMeta {
            name: "convert",
            usage: "jed convert <format> <file>",
            description: "Convert JSON to another format (yaml)",
            example: "jed convert yaml config.json",
        },
        CmdMeta {
            name: "commands",
            usage: "jed commands",
            description: "List all available commands",
            example: "jed commands",
        },
        CmdMeta {
            name: "explain",
            usage: "jed explain <command>",
            description: "Show detailed help for a command",
            example: "jed explain set",
        },
        CmdMeta {
            name: "completions",
            usage: "jed completions <shell>",
            description: "Generate shell completion script",
            example: "jed completions bash > ~/.bash_completion.d/jed",
        },
    ]
}

fn print_commands(ctx: &Ctx) {
    let cmds: Vec<serde_json::Value> = all_commands()
        .iter()
        .map(|c| {
            serde_json::json!({
                "name": c.name,
                "usage": c.usage,
                "description": c.description,
            })
        })
        .collect();

    let actions = vec!["jed explain <command>".to_string()];
    ctx.print_raw_with_actions(serde_json::json!({"commands": cmds}), &actions);
}

fn print_command_help(cmd_name: &str, ctx: &Ctx) {
    let cmds = all_commands();
    if let Some(c) = cmds.iter().find(|c| c.name == cmd_name) {
        ctx.print_raw(serde_json::json!({
            "name":        c.name,
            "usage":       c.usage,
            "description": c.description,
            "example":     c.example,
        }));
    } else {
        let fix = "Run 'jed commands' to see all available commands";
        let actions = vec!["jed commands".to_string()];
        ctx.print_error(
            &format!("Unknown command: '{cmd_name}'"),
            Some(fix),
            &actions,
        );
        std::process::exit(command::exit_code::ERROR);
    }
}
