use indexmap::IndexMap;

use crate::engine::{
    path::{PathError, PathSegment, parse_path, resolve_index},
    value::JsonValue,
};

/// 编辑操作错误。
#[derive(Debug, thiserror::Error)]
pub enum EditError {
    #[error(transparent)]
    Path(#[from] PathError),

    #[error("不能向 {type_name} 追加元素：期望数组或对象")]
    NotAddable { type_name: &'static str },

    #[error("源路径与目标路径相同")]
    SamePath,
}

/// 设置指定路径的值。路径不存在时自动创建中间层对象节点。
///
/// 若路径为 `"."` 或 `""` 则替换整个文档。
pub fn set(doc: &mut JsonValue, path: &str, value: JsonValue) -> Result<(), EditError> {
    let segments = parse_path(path)?;
    if segments.is_empty() {
        *doc = value;
        return Ok(());
    }
    set_recursive(doc, &segments, value)
}

/// 删除指定路径的 key 或数组元素，返回被删除的值。
pub fn delete(doc: &mut JsonValue, path: &str) -> Result<JsonValue, EditError> {
    let segments = parse_path(path)?;
    if segments.is_empty() {
        // 不允许删除根节点
        return Err(EditError::Path(PathError::InvalidSyntax(
            "不能删除根节点".into(),
        )));
    }

    let (parent_segments, last) = segments.split_at(segments.len() - 1);
    let parent = navigate_to_mut(doc, parent_segments)?;

    match &last[0] {
        PathSegment::Key(key) => {
            let type_name = parent.type_name();
            let map = parent
                .as_object_mut()
                .ok_or(PathError::ExpectedObject { type_name })?;
            map.shift_remove(key.as_str())
                .ok_or_else(|| PathError::KeyNotFound { key: key.clone() }.into())
        }
        PathSegment::Index(idx) => {
            let type_name = parent.type_name();
            let arr = parent
                .as_array_mut()
                .ok_or(PathError::ExpectedArray { type_name })?;
            let i = resolve_index(*idx, arr.len())?;
            Ok(arr.remove(i))
        }
    }
}

/// 重命名指定路径的 key（仅适用于对象中的 key，不适用于数组索引）。
pub fn rename_key(doc: &mut JsonValue, path: &str, new_key: &str) -> Result<(), EditError> {
    let segments = parse_path(path)?;
    if segments.is_empty() {
        return Err(EditError::Path(PathError::InvalidSyntax(
            "不能重命名根节点".into(),
        )));
    }

    let (parent_segments, last) = segments.split_at(segments.len() - 1);
    let parent = navigate_to_mut(doc, parent_segments)?;

    // 获取要重命名的 key 名称
    let old_key = match &last[0] {
        PathSegment::Key(k) => k.as_str(),
        PathSegment::Index(_) => {
            return Err(EditError::Path(PathError::InvalidSyntax(
                "数组索引不能重命名".into(),
            )));
        }
    };

    // 不能重命名空 key
    if new_key.is_empty() {
        return Err(EditError::Path(PathError::InvalidSyntax(
            "key 不能为空".into(),
        )));
    }

    let type_name = parent.type_name();
    let map = parent
        .as_object_mut()
        .ok_or(PathError::ExpectedObject { type_name })?;

    // 获取旧 key 的值
    let value = map
        .get(old_key)
        .ok_or_else(|| PathError::KeyNotFound {
            key: old_key.into(),
        })?
        .clone();

    // 删除旧 key 并插入新 key（保持顺序）
    map.shift_remove(old_key);
    map.insert(new_key.into(), value);

    Ok(())
}

/// 向数组末尾追加元素，或向对象合并新字段（已存在的 key 会被覆盖）。
pub fn add(doc: &mut JsonValue, path: &str, value: JsonValue) -> Result<(), EditError> {
    let node = if path == "." || path.is_empty() {
        doc
    } else {
        let segments = parse_path(path)?;
        navigate_to_mut(doc, &segments)?
    };

    match node {
        JsonValue::Array(arr) => {
            arr.push(value);
            Ok(())
        }
        JsonValue::Object(map) => {
            // value 必须是对象，将其字段合并进来
            if let JsonValue::Object(new_fields) = value {
                for (k, v) in new_fields {
                    map.insert(k, v);
                }
                Ok(())
            } else {
                Err(EditError::NotAddable {
                    type_name: node.type_name(),
                })
            }
        }
        other => Err(EditError::NotAddable {
            type_name: other.type_name(),
        }),
    }
}

/// 向数组的指定索引位置插入元素。
/// 对于对象，等价于设置一个新字段（需要提供 key）。
pub fn insert(
    doc: &mut JsonValue,
    path: &str,
    index: usize,
    value: JsonValue,
) -> Result<(), EditError> {
    let node = if path == "." || path.is_empty() {
        doc
    } else {
        let segments = parse_path(path)?;
        navigate_to_mut(doc, &segments)?
    };

    let JsonValue::Array(arr) = node else {
        return Err(EditError::NotAddable {
            type_name: node.type_name(),
        });
    };

    // 允许在末尾插入（index == len）以及正常插入
    if index > arr.len() {
        return Err(EditError::Path(PathError::IndexOutOfBounds {
            index: i64::try_from(index).unwrap_or(i64::MAX),
            len: arr.len(),
        }));
    }
    arr.insert(index, value);
    Ok(())
}

/// 将源路径的值移动到目标路径（先删除再设置）。
pub fn move_value(doc: &mut JsonValue, src: &str, dst: &str) -> Result<(), EditError> {
    if src == dst {
        return Err(EditError::SamePath);
    }
    let value = delete(doc, src)?;
    set(doc, dst, value)
}

// ── 内部辅助 ─────────────────────────────────────────────────────────────────

/// 递归设置值，沿途自动创建缺失的中间对象节点。
fn set_recursive(
    node: &mut JsonValue,
    segments: &[PathSegment],
    value: JsonValue,
) -> Result<(), EditError> {
    let Some((head, tail)) = segments.split_first() else {
        *node = value;
        return Ok(());
    };

    match head {
        PathSegment::Key(key) => {
            // 若当前节点不是对象，替换为空对象（创建中间层）
            if !matches!(node, JsonValue::Object(_)) {
                *node = JsonValue::Object(IndexMap::new());
            }
            let JsonValue::Object(map) = node else {
                unreachable!()
            };
            let child = map.entry(key.clone()).or_insert(JsonValue::Null);
            set_recursive(child, tail, value)
        }
        PathSegment::Index(idx) => {
            let type_name = node.type_name();
            let arr = node
                .as_array_mut()
                .ok_or(PathError::ExpectedArray { type_name })?;
            let i = resolve_index(*idx, arr.len())?;
            set_recursive(&mut arr[i], tail, value)
        }
    }
}

/// 导航到指定路径对应的节点（可变引用），不创建中间层。
fn navigate_to_mut<'a>(
    node: &'a mut JsonValue,
    segments: &[PathSegment],
) -> Result<&'a mut JsonValue, EditError> {
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
            navigate_to_mut(child, tail)
        }
        PathSegment::Index(idx) => {
            let type_name = node.type_name();
            let arr = node
                .as_array_mut()
                .ok_or(PathError::ExpectedArray { type_name })?;
            let len = arr.len();
            let i = resolve_index(*idx, len)?;
            navigate_to_mut(&mut arr[i], tail)
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
        root.insert(
            "tags".into(),
            JsonValue::Array(vec![
                JsonValue::String("rust".into()),
                JsonValue::String("cli".into()),
            ]),
        );
        JsonValue::Object(root)
    }

    mod set {
        use super::*;

        #[test]
        fn sets_existing_top_level_key() {
            let mut doc = make_doc();
            set(&mut doc, ".name", JsonValue::String("Bob".into())).unwrap();
            assert_eq!(
                doc.as_object().unwrap().get("name").unwrap().as_str(),
                Some("Bob")
            );
        }

        #[test]
        fn creates_new_key_when_missing() {
            let mut doc = make_doc();
            set(&mut doc, ".age", JsonValue::Number(25.0)).unwrap();
            assert!(doc.as_object().unwrap().contains_key("age"));
        }

        #[test]
        fn creates_intermediate_objects_automatically() {
            let mut doc = make_doc();
            set(
                &mut doc,
                ".server.host",
                JsonValue::String("localhost".into()),
            )
            .unwrap();
            let host = doc
                .as_object()
                .unwrap()
                .get("server")
                .unwrap()
                .as_object()
                .unwrap()
                .get("host")
                .unwrap();
            assert_eq!(host.as_str(), Some("localhost"));
        }

        #[test]
        fn sets_array_element_by_index() {
            let mut doc = make_doc();
            set(&mut doc, ".tags[0]", JsonValue::String("go".into())).unwrap();
            let tags = doc
                .as_object()
                .unwrap()
                .get("tags")
                .unwrap()
                .as_array()
                .unwrap();
            assert_eq!(tags[0].as_str(), Some("go"));
        }

        #[test]
        fn replaces_root_when_path_is_dot() {
            let mut doc = make_doc();
            set(&mut doc, ".", JsonValue::Null).unwrap();
            assert_eq!(doc, JsonValue::Null);
        }
    }

    mod delete {
        use super::*;

        #[test]
        fn deletes_existing_key_and_returns_value() {
            let mut doc = make_doc();
            let removed = delete(&mut doc, ".name").unwrap();
            assert_eq!(removed.as_str(), Some("Alice"));
            assert!(!doc.as_object().unwrap().contains_key("name"));
        }

        #[test]
        fn deletes_array_element_by_index() {
            let mut doc = make_doc();
            delete(&mut doc, ".tags[0]").unwrap();
            let tags = doc
                .as_object()
                .unwrap()
                .get("tags")
                .unwrap()
                .as_array()
                .unwrap();
            assert_eq!(tags.len(), 1);
            assert_eq!(tags[0].as_str(), Some("cli"));
        }

        #[test]
        fn returns_error_on_missing_key() {
            let mut doc = make_doc();
            let err = delete(&mut doc, ".missing").unwrap_err();
            assert!(matches!(
                err,
                EditError::Path(PathError::KeyNotFound { .. })
            ));
        }

        #[test]
        fn returns_error_when_deleting_root() {
            let mut doc = make_doc();
            assert!(delete(&mut doc, ".").is_err());
        }
    }

    mod add {
        use super::*;

        #[test]
        fn appends_to_array() {
            let mut doc = make_doc();
            add(&mut doc, ".tags", JsonValue::String("go".into())).unwrap();
            let tags = doc
                .as_object()
                .unwrap()
                .get("tags")
                .unwrap()
                .as_array()
                .unwrap();
            assert_eq!(tags.len(), 3);
            assert_eq!(tags[2].as_str(), Some("go"));
        }

        #[test]
        fn returns_error_when_adding_non_object_to_object() {
            let mut doc = make_doc();
            // 尝试向对象 add 一个非对象值
            let err = add(&mut doc, ".", JsonValue::String("oops".into())).unwrap_err();
            assert!(matches!(err, EditError::NotAddable { .. }));
        }
    }

    mod move_value {
        use super::*;

        #[test]
        fn moves_key_to_new_location() {
            let mut doc = make_doc();
            move_value(&mut doc, ".name", ".fullName").unwrap();
            let obj = doc.as_object().unwrap();
            assert!(!obj.contains_key("name"));
            assert_eq!(obj.get("fullName").unwrap().as_str(), Some("Alice"));
        }

        #[test]
        fn returns_error_when_src_equals_dst() {
            let mut doc = make_doc();
            let err = move_value(&mut doc, ".name", ".name").unwrap_err();
            assert!(matches!(err, EditError::SamePath));
        }
    }
}
