use crate::engine::value::JsonValue;

/// 路径中的单个段。
#[derive(Debug, Clone, PartialEq)]
pub enum PathSegment {
    /// 对象字段名，例如 `.key`
    Key(String),
    /// 数组索引，负数表示从末尾计数，例如 `[0]` 或 `[-1]`
    Index(i64),
}

/// 解析后的路径，由若干段组成。
pub type Path = Vec<PathSegment>;

/// 路径操作错误。
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum PathError {
    #[error("路径语法无效：{0}")]
    InvalidSyntax(String),

    #[error("key '{key}' 在对象中不存在")]
    KeyNotFound { key: String },

    #[error("索引 {index} 越界（长度 {len}）")]
    IndexOutOfBounds { index: i64, len: usize },

    #[error("路径期望对象，但实际是 {type_name}")]
    ExpectedObject { type_name: &'static str },

    #[error("路径期望数组，但实际是 {type_name}")]
    ExpectedArray { type_name: &'static str },
}

/// 将路径字符串解析为路径段列表。
///
/// 支持的语法：
/// - `.` — 根节点（返回空路径）
/// - `.key` — 对象字段
/// - `[0]` 或 `[-1]` — 数组索引
/// - 以上组合，例如 `.servers[0].host`
pub fn parse_path(path: &str) -> Result<Path, PathError> {
    if path == "." || path.is_empty() {
        return Ok(Vec::new());
    }

    let mut segments = Vec::new();
    let bytes = path.as_bytes();
    let mut pos = 0;

    while pos < bytes.len() {
        match bytes[pos] {
            b'.' => {
                pos += 1; // 跳过 '.'
                if pos >= bytes.len() || bytes[pos] == b'[' {
                    // 单独的 '.' 或 '.[' — 不产生段，继续
                    continue;
                }
                let start = pos;
                while pos < bytes.len() && bytes[pos] != b'.' && bytes[pos] != b'[' {
                    pos += 1;
                }
                let key = &path[start..pos];
                if key.is_empty() {
                    return Err(PathError::InvalidSyntax(format!(
                        "第 {pos} 位置出现空 key"
                    )));
                }
                segments.push(PathSegment::Key(key.to_string()));
            }
            b'[' => {
                pos += 1; // 跳过 '['
                let start = pos;
                while pos < bytes.len() && bytes[pos] != b']' {
                    pos += 1;
                }
                if pos >= bytes.len() {
                    return Err(PathError::InvalidSyntax("缺少右括号 ']'".into()));
                }
                let idx_str = &path[start..pos];
                let idx: i64 = idx_str.parse().map_err(|_| {
                    PathError::InvalidSyntax(format!("无效的数组索引: '{idx_str}'"))
                })?;
                segments.push(PathSegment::Index(idx));
                pos += 1; // 跳过 ']'
            }
            ch => {
                return Err(PathError::InvalidSyntax(format!(
                    "路径中意外的字符 '{}'",
                    char::from(ch)
                )));
            }
        }
    }

    Ok(segments)
}

/// 将负索引或越界索引解析为实际的 `usize` 位置。
pub(crate) fn resolve_index(idx: i64, len: usize) -> Result<usize, PathError> {
    if idx >= 0 {
        let u = idx as usize;
        if u < len {
            Ok(u)
        } else {
            Err(PathError::IndexOutOfBounds { index: idx, len })
        }
    } else {
        let from_end = (-idx) as usize;
        if from_end <= len {
            Ok(len - from_end)
        } else {
            Err(PathError::IndexOutOfBounds { index: idx, len })
        }
    }
}

/// 在文档中查找路径对应的值（不可变引用）。
pub fn get<'a>(doc: &'a JsonValue, path: &str) -> Result<&'a JsonValue, PathError> {
    let segments = parse_path(path)?;
    navigate(doc, &segments)
}

/// 在文档中查找路径对应的值（可变引用）。
pub fn get_mut<'a>(
    doc: &'a mut JsonValue,
    path: &str,
) -> Result<&'a mut JsonValue, PathError> {
    let segments = parse_path(path)?;
    navigate_mut(doc, &segments)
}

/// 检查路径是否存在。
pub fn exists(doc: &JsonValue, path: &str) -> bool {
    get(doc, path).is_ok()
}

fn navigate<'a>(node: &'a JsonValue, segments: &[PathSegment]) -> Result<&'a JsonValue, PathError> {
    let Some((head, tail)) = segments.split_first() else {
        return Ok(node);
    };

    match head {
        PathSegment::Key(key) => {
            let map = node.as_object().ok_or(PathError::ExpectedObject {
                type_name: node.type_name(),
            })?;
            let child = map
                .get(key.as_str())
                .ok_or_else(|| PathError::KeyNotFound { key: key.clone() })?;
            navigate(child, tail)
        }
        PathSegment::Index(idx) => {
            let arr = node.as_array().ok_or(PathError::ExpectedArray {
                type_name: node.type_name(),
            })?;
            let i = resolve_index(*idx, arr.len())?;
            navigate(&arr[i], tail)
        }
    }
}

fn navigate_mut<'a>(
    node: &'a mut JsonValue,
    segments: &[PathSegment],
) -> Result<&'a mut JsonValue, PathError> {
    let Some((head, tail)) = segments.split_first() else {
        return Ok(node);
    };

    match head {
        PathSegment::Key(key) => {
            let type_name = node.type_name();
            let map = node
                .as_object_mut()
                .ok_or(PathError::ExpectedObject { type_name })?;
            let child = map
                .get_mut(key.as_str())
                .ok_or_else(|| PathError::KeyNotFound { key: key.clone() })?;
            navigate_mut(child, tail)
        }
        PathSegment::Index(idx) => {
            let type_name = node.type_name();
            let arr = node
                .as_array_mut()
                .ok_or(PathError::ExpectedArray { type_name })?;
            let len = arr.len();
            let i = resolve_index(*idx, len)?;
            navigate_mut(&mut arr[i], tail)
        }
    }
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;

    use super::*;

    fn make_doc() -> JsonValue {
        let mut root = IndexMap::new();
        root.insert("name".into(), JsonValue::String("Alice".into()));
        root.insert("age".into(), JsonValue::Number(30.0));
        root.insert(
            "tags".into(),
            JsonValue::Array(vec![
                JsonValue::String("rust".into()),
                JsonValue::String("cli".into()),
            ]),
        );
        let mut nested = IndexMap::new();
        nested.insert("port".into(), JsonValue::Number(8080.0));
        root.insert("server".into(), JsonValue::Object(nested));
        JsonValue::Object(root)
    }

    mod parse_path {
        use super::*;

        #[test]
        fn dot_returns_empty_path() {
            assert_eq!(parse_path(".").unwrap(), vec![]);
        }

        #[test]
        fn single_key_parsed_correctly() {
            assert_eq!(
                parse_path(".name").unwrap(),
                vec![PathSegment::Key("name".into())]
            );
        }

        #[test]
        fn nested_key_parsed_correctly() {
            assert_eq!(
                parse_path(".server.port").unwrap(),
                vec![
                    PathSegment::Key("server".into()),
                    PathSegment::Key("port".into()),
                ]
            );
        }

        #[test]
        fn array_index_parsed_correctly() {
            assert_eq!(
                parse_path(".tags[0]").unwrap(),
                vec![
                    PathSegment::Key("tags".into()),
                    PathSegment::Index(0),
                ]
            );
        }

        #[test]
        fn negative_index_parsed_correctly() {
            assert_eq!(
                parse_path(".tags[-1]").unwrap(),
                vec![
                    PathSegment::Key("tags".into()),
                    PathSegment::Index(-1),
                ]
            );
        }

        #[test]
        fn returns_error_on_unclosed_bracket() {
            assert!(parse_path(".arr[0").is_err());
        }

        #[test]
        fn returns_error_on_non_integer_index() {
            assert!(parse_path(".arr[abc]").is_err());
        }
    }

    mod get {
        use super::*;

        #[test]
        fn gets_top_level_string() {
            let doc = make_doc();
            let v = get(&doc, ".name").unwrap();
            assert_eq!(v.as_str(), Some("Alice"));
        }

        #[test]
        fn gets_array_element_by_index() {
            let doc = make_doc();
            let v = get(&doc, ".tags[0]").unwrap();
            assert_eq!(v.as_str(), Some("rust"));
        }

        #[test]
        fn gets_array_last_element_with_negative_index() {
            let doc = make_doc();
            let v = get(&doc, ".tags[-1]").unwrap();
            assert_eq!(v.as_str(), Some("cli"));
        }

        #[test]
        fn gets_nested_object_field() {
            let doc = make_doc();
            let v = get(&doc, ".server.port").unwrap();
            assert_eq!(v.as_f64(), Some(8080.0));
        }

        #[test]
        fn returns_root_on_empty_path() {
            let doc = make_doc();
            let v = get(&doc, ".").unwrap();
            assert_eq!(v.type_name(), "object");
        }

        #[test]
        fn returns_key_not_found_error_on_missing_key() {
            let doc = make_doc();
            let err = get(&doc, ".missing").unwrap_err();
            assert!(matches!(err, PathError::KeyNotFound { .. }));
        }

        #[test]
        fn returns_index_out_of_bounds_error_on_bad_index() {
            let doc = make_doc();
            let err = get(&doc, ".tags[99]").unwrap_err();
            assert!(matches!(err, PathError::IndexOutOfBounds { .. }));
        }
    }

    mod exists {
        use super::*;

        #[test]
        fn returns_true_for_existing_path() {
            let doc = make_doc();
            assert!(exists(&doc, ".name"));
        }

        #[test]
        fn returns_false_for_missing_path() {
            let doc = make_doc();
            assert!(!exists(&doc, ".missing"));
        }
    }
}
