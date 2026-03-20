mod cli;
mod command;
mod engine;
mod tui;

use clap::Parser;

use crate::cli::Cli;

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(cmd) => {
            // 命令模式：Agent 友好，最小化输出
            command::run(&cli.file, cmd);
        }
        None => {
            // TUI 模式：人类交互编辑器
            if let Err(e) = tui::run_tui(cli.file) {
                eprintln!("TUI 错误：{e}");
                std::process::exit(1);
            }
        }
    }
}
