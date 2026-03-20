use std::collections::HashSet;

use crate::engine::value::JsonValue;

/// 推断并返回 JSON 值的紧凑结构描述（不含实际值，只有类型和形状）。
///
/// 适合 Agent 在操作前先了解文件结构，减少 token 消耗。
///
/// # 示例
///
/// ```text
/// 输入：{"name": "Alice", "age": 30, "tags": ["rust", "cli"]}
/// 输出：{name: string, age: number, tags: [string]}
/// ```
pub fn infer_schema(value: &JsonValue) -> String {
    infer(value, 0)
}

fn infer(value: &JsonValue, depth: usize) -> String {
    match value {
        JsonValue::Null => "null".into(),
        JsonValue::Bool(_) => "boolean".into(),
        JsonValue::Number(_) => "number".into(),
        JsonValue::String(_) => "string".into(),
        JsonValue::Array(arr) => infer_array_schema(arr, depth),
        JsonValue::Object(map) => infer_object_schema(map, depth),
    }
}

fn infer_array_schema(arr: &[JsonValue], depth: usize) -> String {
    if arr.is_empty() {
        return "[]".into();
    }

    // 收集数组中出现的所有类型，对象类型合并结构
    let mut type_set: HashSet<String> = HashSet::new();
    let mut merged_object: Option<indexmap::IndexMap<String, String>> = None;

    for item in arr {
        match item {
            JsonValue::Object(map) => {
                let obj_schema = infer_object_fields(map, depth + 1);
                match merged_object {
                    None => merged_object = Some(obj_schema),
                    Some(ref mut existing) => {
                        // 合并对象结构：只保留两者都有的字段
                        for (k, v) in &obj_schema {
                            existing.entry(k.clone()).or_insert_with(|| v.clone());
                        }
                    }
                }
            }
            other => {
                type_set.insert(infer(other, depth + 1));
            }
        }
    }

    let mut parts: Vec<String> = type_set.into_iter().collect();
    parts.sort(); // 排序保证输出稳定

    if let Some(obj_fields) = merged_object {
        parts.push(format_object_fields(&obj_fields));
    }

    format!("[{}]", parts.join(" | "))
}

fn infer_object_schema(
    map: &indexmap::IndexMap<String, JsonValue>,
    depth: usize,
) -> String {
    let fields = infer_object_fields(map, depth);
    format_object_fields(&fields)
}

fn infer_object_fields(
    map: &indexmap::IndexMap<String, JsonValue>,
    depth: usize,
) -> indexmap::IndexMap<String, String> {
    map.iter()
        .map(|(k, v)| (k.clone(), infer(v, depth + 1)))
        .collect()
}

fn format_object_fields(fields: &indexmap::IndexMap<String, String>) -> String {
    if fields.is_empty() {
        return "{}".into();
    }
    let inner: Vec<String> = fields
        .iter()
        .map(|(k, v)| format!("{k}: {v}"))
        .collect();
    format!("{{{}}}", inner.join(", "))
}

#[cfg(test)]
mod tests {
    use indexmap::IndexMap;

    use super::*;

    mod infer_schema {
        use super::*;

        #[test]
        fn null_returns_null() {
            assert_eq!(infer_schema(&JsonValue::Null), "null");
        }

        #[test]
        fn bool_returns_boolean() {
            assert_eq!(infer_schema(&JsonValue::Bool(true)), "boolean");
        }

        #[test]
        fn number_returns_number() {
            assert_eq!(infer_schema(&JsonValue::Number(42.0)), "number");
        }

        #[test]
        fn string_returns_string() {
            assert_eq!(infer_schema(&JsonValue::String("hi".into())), "string");
        }

        #[test]
        fn empty_array_returns_empty_brackets() {
            assert_eq!(infer_schema(&JsonValue::Array(vec![])), "[]");
        }

        #[test]
        fn homogeneous_string_array_schema() {
            let v = JsonValue::Array(vec![
                JsonValue::String("a".into()),
                JsonValue::String("b".into()),
            ]);
            assert_eq!(infer_schema(&v), "[string]");
        }

        #[test]
        fn heterogeneous_array_uses_union_type() {
            let v = JsonValue::Array(vec![
                JsonValue::String("a".into()),
                JsonValue::Number(1.0),
            ]);
            let s = infer_schema(&v);
            assert!(s.contains("string"));
            assert!(s.contains("number"));
            assert!(s.contains('|'));
        }

        #[test]
        fn object_schema_shows_key_types() {
            let mut m = IndexMap::new();
            m.insert("name".into(), JsonValue::String("Alice".into()));
            m.insert("age".into(), JsonValue::Number(30.0));
            let s = infer_schema(&JsonValue::Object(m));
            assert!(s.contains("name: string"));
            assert!(s.contains("age: number"));
        }

        #[test]
        fn nested_object_schema_is_recursive() {
            let mut inner = IndexMap::new();
            inner.insert("port".into(), JsonValue::Number(8080.0));
            let mut outer = IndexMap::new();
            outer.insert("server".into(), JsonValue::Object(inner));
            let s = infer_schema(&JsonValue::Object(outer));
            assert!(s.contains("server: {port: number}"));
        }
    }
}
