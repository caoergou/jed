pub mod read;
pub mod repair;
pub mod write;

use std::{fs, path::Path, process};

use crate::{
    cli::Command,
    engine::{parse_lenient, parse_strict, JsonValue},
};

/// 命令模式的统一退出码。
pub mod exit_code {
    pub const OK: i32 = 0;
    pub const ERROR: i32 = 1;
    pub const NOT_FOUND: i32 = 2;
    pub const TYPE_MISMATCH: i32 = 3;
}

/// 执行命令并以适当的退出码退出。
pub fn run(file: &Path, cmd: Command) {
    let result = dispatch(file, cmd);
    match result {
        Ok(code) => process::exit(code),
        Err(e) => {
            eprintln!("错误：{e}");
            process::exit(exit_code::ERROR);
        }
    }
}

fn dispatch(file: &Path, cmd: Command) -> Result<i32, Box<dyn std::error::Error>> {
    match cmd {
        Command::Get { path } => read::cmd_get(file, &path),
        Command::Keys { path } => read::cmd_keys(file, &path),
        Command::Len { path } => read::cmd_len(file, &path),
        Command::Type { path } => read::cmd_type(file, &path),
        Command::Exists { path } => read::cmd_exists(file, &path),
        Command::Schema => read::cmd_schema(file),
        Command::Check => read::cmd_check(file),
        Command::Set { path, value } => write::cmd_set(file, &path, &value),
        Command::Del { path } => write::cmd_del(file, &path),
        Command::Add { path, value } => write::cmd_add(file, &path, &value),
        Command::Patch { operations } => write::cmd_patch(file, &operations),
        Command::Mv { src, dst } => write::cmd_mv(file, &src, &dst),
        Command::Fmt { indent } => repair::cmd_fmt(file, indent),
        Command::Fix {
            dry_run,
            strip_comments,
        } => repair::cmd_fix(file, dry_run, strip_comments),
        Command::Minify => repair::cmd_minify(file),
        Command::Diff { other } => read::cmd_diff(file, &other),
    }
}

/// 读取文件内容，返回错误信息若文件不存在。
pub(crate) fn read_file(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    fs::read_to_string(path).map_err(|e| format!("无法读取 '{}': {e}", path.display()).into())
}

/// 读取并宽松解析文件，返回文档和修复列表。
pub(crate) fn load_lenient(
    path: &Path,
) -> Result<(JsonValue, Vec<crate::engine::Repair>), Box<dyn std::error::Error>> {
    let content = read_file(path)?;
    let output = parse_lenient(&content)
        .map_err(|e| format!("解析失败 '{}': {e}", path.display()))?;
    Ok((output.value, output.repairs))
}

/// 读取并严格解析文件。
pub(crate) fn load_strict(path: &Path) -> Result<JsonValue, Box<dyn std::error::Error>> {
    let content = read_file(path)?;
    parse_strict(&content).map_err(|e| format!("解析失败 '{}': {e}", path.display()).into())
}

/// 原子写入文件：写临时文件 → fsync → 重命名。
pub(crate) fn write_file_atomic(
    path: &Path,
    content: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, content)
        .map_err(|e| format!("写入临时文件失败: {e}"))?;
    fs::rename(&tmp_path, path)
        .map_err(|e| format!("重命名文件失败: {e}"))?;
    Ok(())
}
