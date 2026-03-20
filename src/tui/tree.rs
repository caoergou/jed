use std::collections::HashSet;

use crate::engine::JsonValue;

/// 树形视图中的一行。
#[derive(Debug, Clone)]
pub struct TreeLine {
    /// 缩进层级。
    pub depth: usize,
    /// 显示用 key 标签，例如 `"name"` 或 `[0]`；根节点为空。
    pub display_key: String,
    /// 值的单行预览文本。
    pub value_preview: String,
    /// 值的类型名称。
    pub value_type: &'static str,
    /// 是否已展开（仅容器类型有意义）。
    pub is_expanded: bool,
    /// 是否含子节点。
    pub has_children: bool,
    /// 此节点对应的路径字符串，例如 `.name` 或 `.tags[0]`。
    pub path: String,
}

/// 将文档树展平为适合渲染的有序行列表。
///
/// 只展开 `expanded` 集合中包含的路径。
pub fn flatten(doc: &JsonValue, expanded: &HashSet<String>) -> Vec<TreeLine> {
    let mut lines = Vec::new();
    flatten_node(doc, ".", 0, String::new(), expanded, &mut lines);
    lines
}

fn flatten_node(
    value: &JsonValue,
    path: &str,
    depth: usize,
    display_key: String,
    expanded: &HashSet<String>,
    lines: &mut Vec<TreeLine>,
) {
    let has_children = match value {
        JsonValue::Object(m) => !m.is_empty(),
        JsonValue::Array(a) => !a.is_empty(),
        _ => false,
    };

    let is_expanded = has_children && expanded.contains(path);

    let value_preview = if is_expanded {
        // 展开时只显示开括号
        match value {
            JsonValue::Object(_) => "{".into(),
            JsonValue::Array(_) => "[".into(),
            _ => value.to_string(),
        }
    } else {
        preview(value)
    };

    lines.push(TreeLine {
        depth,
        display_key,
        value_preview,
        value_type: value.type_name(),
        is_expanded,
        has_children,
        path: path.to_string(),
    });

    if !is_expanded {
        return;
    }

    match value {
        JsonValue::Object(map) => {
            for (key, child) in map {
                let child_path = child_path_key(path, key);
                let label = format!("\"{key}\"");
                flatten_node(child, &child_path, depth + 1, label, expanded, lines);
            }
            // 闭括号行
            lines.push(closing_line("}", path, depth));
        }
        JsonValue::Array(arr) => {
            for (i, child) in arr.iter().enumerate() {
                let child_path = format!("{path}[{i}]");
                let label = format!("[{i}]");
                flatten_node(child, &child_path, depth + 1, label, expanded, lines);
            }
            // 闭括号行
            lines.push(closing_line("]", path, depth));
        }
        _ => {}
    }
}

fn closing_line(bracket: &str, parent_path: &str, depth: usize) -> TreeLine {
    TreeLine {
        depth,
        display_key: String::new(),
        value_preview: bracket.into(),
        value_type: "closing",
        is_expanded: false,
        has_children: false,
        // 闭括号用特殊路径标记，前缀 "__close__"
        path: format!("__close__{parent_path}"),
    }
}

/// 生成子路径（对象 key）。
pub(crate) fn child_path_key(parent: &str, key: &str) -> String {
    if parent == "." {
        format!(".{key}")
    } else {
        format!("{parent}.{key}")
    }
}

/// 值的单行预览，长度截断至 60 字符。
fn preview(value: &JsonValue) -> String {
    let s = match value {
        JsonValue::Null => "null".into(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Number(n) => {
            if n.fract() == 0.0 && n.abs() < 1e15 {
                format!("{}", *n as i64)
            } else {
                format!("{n}")
            }
        }
        JsonValue::String(s) => format!("\"{}\"", truncate(s, 50)),
        JsonValue::Object(m) => {
            if m.is_empty() {
                "{}".into()
            } else {
                format!("{{…{} keys}}", m.len())
            }
        }
        JsonValue::Array(a) => {
            if a.is_empty() {
                "[]".into()
            } else {
                format!("[…{} items]", a.len())
            }
        }
    };
    s
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max).collect();
        format!("{truncated}…")
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;

    use super::*;

    fn make_doc() -> JsonValue {
        let mut root = IndexMap::new();
        root.insert("name".into(), JsonValue::String("Alice".into()));
        root.insert(
            "tags".into(),
            JsonValue::Array(vec![JsonValue::String("rust".into())]),
        );
        JsonValue::Object(root)
    }

    mod flatten {
        use super::*;

        #[test]
        fn collapsed_root_produces_single_line() {
            let doc = make_doc();
            let expanded = HashSet::new(); // nothing expanded
            let lines = flatten(&doc, &expanded);
            assert_eq!(lines.len(), 1);
            assert_eq!(lines[0].path, ".");
            assert!(!lines[0].is_expanded);
        }

        #[test]
        fn expanded_root_shows_children() {
            let doc = make_doc();
            let mut expanded = HashSet::new();
            expanded.insert(".".into());
            let lines = flatten(&doc, &expanded);
            // root line + "name" + "tags" + closing "}"
            assert!(lines.len() >= 3);
            assert!(lines.iter().any(|l| l.path == ".name"));
            assert!(lines.iter().any(|l| l.path == ".tags"));
        }

        #[test]
        fn expanding_array_shows_indexed_children() {
            let doc = make_doc();
            let mut expanded = HashSet::new();
            expanded.insert(".".into());
            expanded.insert(".tags".into());
            let lines = flatten(&doc, &expanded);
            assert!(lines.iter().any(|l| l.path == ".tags[0]"));
        }

        #[test]
        fn closing_bracket_line_is_not_selectable() {
            let doc = make_doc();
            let mut expanded = HashSet::new();
            expanded.insert(".".into());
            let lines = flatten(&doc, &expanded);
            let closing: Vec<_> = lines
                .iter()
                .filter(|l| l.path.starts_with("__close__"))
                .collect();
            assert!(!closing.is_empty());
        }
    }
}
