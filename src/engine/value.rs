use std::fmt;

use indexmap::IndexMap;

/// JSON 文档的核心值类型。
///
/// 对象使用 `IndexMap` 保留 key 的插入顺序，这对最小化保存时的 diff 至关重要。
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    /// 数字统一用 f64 存储；格式化时若为整数值则输出为整数形式。
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(IndexMap<String, JsonValue>),
}

impl JsonValue {
    /// 返回值的类型名称字符串。
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null => "null",
            Self::Bool(_) => "boolean",
            Self::Number(_) => "number",
            Self::String(_) => "string",
            Self::Array(_) => "array",
            Self::Object(_) => "object",
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(b) = self { Some(*b) } else { None }
    }

    pub fn as_f64(&self) -> Option<f64> {
        if let Self::Number(n) = self { Some(*n) } else { None }
    }

    pub fn as_str(&self) -> Option<&str> {
        if let Self::String(s) = self { Some(s) } else { None }
    }

    pub fn as_array(&self) -> Option<&[JsonValue]> {
        if let Self::Array(a) = self { Some(a) } else { None }
    }

    pub fn as_object(&self) -> Option<&IndexMap<String, JsonValue>> {
        if let Self::Object(o) = self { Some(o) } else { None }
    }

    pub fn as_array_mut(&mut self) -> Option<&mut Vec<JsonValue>> {
        if let Self::Array(a) = self { Some(a) } else { None }
    }

    pub fn as_object_mut(&mut self) -> Option<&mut IndexMap<String, JsonValue>> {
        if let Self::Object(o) = self { Some(o) } else { None }
    }

    /// 返回数组的长度或对象的 key 数量；非容器类型返回 `None`。
    pub fn len(&self) -> Option<usize> {
        match self {
            Self::Array(a) => Some(a.len()),
            Self::Object(o) => Some(o.len()),
            _ => None,
        }
    }

    /// 容器为空时返回 `true`，非容器类型返回 `false`。
    pub fn is_empty(&self) -> bool {
        self.len().map_or(false, |n| n == 0)
    }
}

impl fmt::Display for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => write!(f, "null"),
            Self::Bool(b) => write!(f, "{b}"),
            Self::Number(n) => fmt_number(*n, f),
            Self::String(s) => {
                write!(f, "\"")?;
                write!(f, "{}", escape_str(s))?;
                write!(f, "\"")
            }
            // 容器类型委托给紧凑格式化
            Self::Array(_) | Self::Object(_) => {
                write!(f, "{}", crate::engine::format::format_compact(self))
            }
        }
    }
}

/// 将 f64 格式化为合法的 JSON 数字字符串。
fn fmt_number(n: f64, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if n.is_nan() || n.is_infinite() {
        // JSON 不支持 NaN/Infinity，降级为 null
        write!(f, "null")
    } else if n.fract() == 0.0 && n.abs() < 1e15 {
        write!(f, "{}", n as i64)
    } else {
        write!(f, "{n}")
    }
}

/// 对 JSON 字符串内的特殊字符进行转义。
pub(crate) fn escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

impl From<serde_json::Value> for JsonValue {
    fn from(v: serde_json::Value) -> Self {
        match v {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(b) => Self::Bool(b),
            serde_json::Value::Number(n) => Self::Number(n.as_f64().unwrap_or(0.0)),
            serde_json::Value::String(s) => Self::String(s),
            serde_json::Value::Array(arr) => {
                Self::Array(arr.into_iter().map(Self::from).collect())
            }
            serde_json::Value::Object(map) => {
                let mut imap = IndexMap::with_capacity(map.len());
                for (k, v) in map {
                    imap.insert(k, Self::from(v));
                }
                Self::Object(imap)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod type_name {
        use super::*;

        #[test]
        fn null_returns_null() {
            assert_eq!(JsonValue::Null.type_name(), "null");
        }

        #[test]
        fn bool_returns_boolean() {
            assert_eq!(JsonValue::Bool(true).type_name(), "boolean");
        }

        #[test]
        fn number_returns_number() {
            assert_eq!(JsonValue::Number(1.0).type_name(), "number");
        }

        #[test]
        fn string_returns_string() {
            assert_eq!(JsonValue::String("hi".into()).type_name(), "string");
        }

        #[test]
        fn array_returns_array() {
            assert_eq!(JsonValue::Array(vec![]).type_name(), "array");
        }

        #[test]
        fn object_returns_object() {
            assert_eq!(JsonValue::Object(IndexMap::new()).type_name(), "object");
        }
    }

    mod display {
        use super::*;

        #[test]
        fn integer_number_has_no_decimal_point() {
            assert_eq!(JsonValue::Number(42.0).to_string(), "42");
        }

        #[test]
        fn float_number_preserves_decimal() {
            assert_eq!(JsonValue::Number(3.14).to_string(), "3.14");
        }

        #[test]
        fn nan_becomes_null() {
            assert_eq!(JsonValue::Number(f64::NAN).to_string(), "null");
        }

        #[test]
        fn string_is_quoted_and_escaped() {
            assert_eq!(
                JsonValue::String("say \"hi\"".into()).to_string(),
                r#""say \"hi\"""#
            );
        }
    }

    mod len {
        use super::*;

        #[test]
        fn array_returns_element_count() {
            let v = JsonValue::Array(vec![JsonValue::Null, JsonValue::Bool(true)]);
            assert_eq!(v.len(), Some(2));
        }

        #[test]
        fn object_returns_key_count() {
            let mut m = IndexMap::new();
            m.insert("a".into(), JsonValue::Null);
            assert_eq!(JsonValue::Object(m).len(), Some(1));
        }

        #[test]
        fn scalar_returns_none() {
            assert_eq!(JsonValue::Number(1.0).len(), None);
        }
    }
}
