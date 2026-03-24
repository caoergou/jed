use std::path::PathBuf;

use clap::{Parser, Subcommand};
use clap_complete::Shell;

#[derive(Debug, Parser)]
#[command(
    name = "jed",
    about = "Jed — JSON editor: dual-interface tool for humans and AI agents",
    version
)]
pub struct Cli {
    /// JSON 文件（位置参数）
    #[arg(default_value = "-", hide_default_value = true)]
    pub file: Option<PathBuf>,

    /// 以 JSON 格式输出结果
    #[arg(short, long)]
    pub json: bool,

    /// 输出语言: en, zh-CN, zh-TW
    #[arg(long, value_parser = ["en", "zh-CN", "zh-TW"])]
    pub lang: Option<String>,

    /// 静默模式
    #[arg(short, long)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

impl Cli {
    /// 获取有效的文件路径
    pub fn get_file(&self) -> Option<PathBuf> {
        self.file.clone()
    }
}

/// 获取命令的文件参数（优先使用全局 --file，子命令后的位置参数，或默认值）
pub fn resolve_file(cli_file: Option<&PathBuf>, cmd_file: Option<&PathBuf>) -> Option<PathBuf> {
    // 1. 全局 --file
    if let Some(f) = cli_file
        && f.to_str() != Some("-")
    {
        return Some(f.clone());
    }
    // 2. 子命令位置参数
    if let Some(f) = cmd_file
        && f.to_str() != Some("-")
    {
        return Some(f.clone());
    }
    // 3. 默认 stdin
    None
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// 获取路径处的值（Agent 友好）
    Get {
        path: String,
        #[arg(default_value = "-")]
        file: PathBuf,
    },

    /// 列出所有 key 或索引
    Keys {
        #[arg(default_value = ".")]
        path: String,
        #[arg(default_value = "-")]
        file: PathBuf,
    },

    /// 返回数组长度或 key 数量
    Len {
        #[arg(default_value = ".")]
        path: String,
        #[arg(default_value = "-")]
        file: PathBuf,
    },

    /// 返回值的类型
    Type {
        #[arg(default_value = ".")]
        path: String,
        #[arg(default_value = "-")]
        file: PathBuf,
    },

    /// 检查路径是否存在
    Exists {
        path: String,
        #[arg(default_value = "-")]
        file: PathBuf,
    },

    /// 推断文件结构
    Schema {
        #[arg(default_value = "-")]
        file: PathBuf,
    },

    /// 校验 JSON 格式
    Check {
        #[arg(default_value = "-")]
        file: PathBuf,
    },

    /// 设置值
    Set {
        path: String,
        value: String,
        #[arg(default_value = "-")]
        file: PathBuf,
    },

    /// 删除
    Del {
        path: String,
        #[arg(default_value = "-")]
        file: PathBuf,
    },

    /// 追加
    Add {
        #[arg(default_value = ".")]
        path: String,
        value: String,
        #[arg(default_value = "-")]
        file: PathBuf,
    },

    /// 批量操作
    Patch {
        operations: String,
        #[arg(default_value = "-")]
        file: PathBuf,
    },

    /// 移动/重命名
    Mv {
        src: String,
        dst: String,
        #[arg(default_value = "-")]
        file: PathBuf,
    },

    /// 格式化
    Fmt {
        #[arg(long, default_value_t = 2)]
        indent: usize,
        #[arg(default_value = "-")]
        file: PathBuf,
    },

    /// 自动修复
    Fix {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        strip_comments: bool,
        #[arg(default_value = "-")]
        file: PathBuf,
    },

    /// 压缩
    Minify {
        #[arg(default_value = "-")]
        file: PathBuf,
    },

    /// 对比差异
    Diff {
        other: PathBuf,
        #[arg(default_value = "-")]
        file: PathBuf,
    },

    /// 列出所有命令
    Commands,

    /// 命令帮助
    Explain { command: String },

    /// 补全脚本
    Completions { shell: Shell },

    /// 树形展示
    Tree {
        #[arg(default_value = "-")]
        file: PathBuf,
        #[arg(long, short)]
        expand_all: bool,
        #[arg(long, short = 'p')]
        path: Option<String>,
    },

    /// 查询过滤
    Query {
        filter: String,
        #[arg(default_value = "-")]
        file: PathBuf,
    },

    /// Schema 验证
    Validate {
        schema: PathBuf,
        #[arg(default_value = "-")]
        file: PathBuf,
    },

    /// 格式转换
    Convert {
        format: String,
        #[arg(default_value = "-")]
        file: PathBuf,
    },
}
