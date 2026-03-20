use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "je",
    about = "JSON 编辑器：同时为人类和 AI Agent 设计的双接口工具",
    version
)]
pub struct Cli {
    /// 要操作的 JSON 文件
    pub file: PathBuf,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// 获取路径处的值（Agent 友好：只输出目标值）
    Get {
        /// 路径，例如 .key 或 .arr[0].field
        path: String,
    },

    /// 列出对象的所有 key 或数组的所有索引
    Keys {
        /// 路径（默认为根 `.`）
        #[arg(default_value = ".")]
        path: String,
    },

    /// 返回数组长度或对象 key 数量
    Len {
        /// 路径（默认为根 `.`）
        #[arg(default_value = ".")]
        path: String,
    },

    /// 返回路径处值的类型
    Type {
        /// 路径（默认为根 `.`）
        #[arg(default_value = ".")]
        path: String,
    },

    /// 检查路径是否存在（exit 0=存在，exit 2=不存在）
    Exists {
        /// 路径
        path: String,
    },

    /// 推断并输出文件结构（不含实际值）
    Schema,

    /// 校验 JSON 格式，错误输出到 stderr
    Check,

    /// 设置路径处的值（路径不存在时自动创建）
    Set {
        /// 路径
        path: String,
        /// JSON 值（字符串、数字、true/false/null 或 JSON 对象/数组）
        value: String,
    },

    /// 删除路径处的 key 或数组元素
    Del {
        /// 路径
        path: String,
    },

    /// 向数组追加元素，或向对象合并字段
    Add {
        /// 路径（默认为根 `.`）
        #[arg(default_value = ".")]
        path: String,
        /// JSON 值
        value: String,
    },

    /// 一次性批量操作（JSON Patch RFC 6902 格式）
    Patch {
        /// JSON Patch 操作数组
        operations: String,
    },

    /// 移动/重命名 key
    Mv {
        /// 源路径
        src: String,
        /// 目标路径
        dst: String,
    },

    /// 格式化（美化）JSON 文件，原地修改
    Fmt {
        /// 缩进空格数
        #[arg(long, default_value_t = 2)]
        indent: usize,
    },

    /// 自动修复 JSON 格式错误，然后格式化
    Fix {
        /// 预览将修复的内容，不实际写入
        #[arg(long)]
        dry_run: bool,

        /// 若文件含注释则剥离（否则报错）
        #[arg(long)]
        strip_comments: bool,
    },

    /// 压缩 JSON（移除所有空白），原地修改
    Minify,

    /// 对比两个 JSON 文件的结构差异
    Diff {
        /// 另一个 JSON 文件
        other: PathBuf,
    },
}

/// 全局输出选项，从环境变量或标志读取。
#[derive(Debug, Clone, Default)]
pub struct OutputOpts {
    /// 以 JSON 格式输出结果 `{"ok":..., "value":...}`
    pub json_output: bool,
}
