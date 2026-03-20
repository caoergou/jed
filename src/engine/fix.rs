use crate::engine::{
    format::{format_pretty, FormatOptions},
    parser::{parse_lenient, ParseError, Repair},
    value::JsonValue,
};

/// 自动修复操作的结果。
pub struct FixResult {
    /// 修复后的文档值（若有无法修复的错误则为 `None`）。
    pub value: Option<JsonValue>,
    /// 已完成的修复列表。
    pub repairs: Vec<Repair>,
    /// 无法自动修复的错误列表。
    pub errors: Vec<ParseError>,
}

impl FixResult {
    /// 是否存在无法修复的错误。
    pub fn has_unfixable(&self) -> bool {
        !self.errors.is_empty()
    }

    /// 是否进行了任何修复。
    pub fn was_repaired(&self) -> bool {
        !self.repairs.is_empty()
    }
}

/// 尝试修复输入字符串中的 JSON 格式错误。
///
/// 使用宽松解析器解析，收集修复记录，然后将结果格式化回合法 JSON。
/// 无法修复的错误会记录在 `FixResult::errors` 中。
pub fn fix(input: &str) -> FixResult {
    match parse_lenient(input) {
        Ok(output) => {
            let formatted = format_pretty(&output.value, &FormatOptions::default());
            FixResult {
                value: Some(JsonValue::String(formatted)), // 暂存为字符串供调用方使用
                repairs: output.repairs,
                errors: Vec::new(),
            }
        }
        Err(e) => FixResult {
            value: None,
            repairs: Vec::new(),
            errors: vec![e],
        },
    }
}

/// 与 `fix` 相同，但将修复后的值作为 `JsonValue` 返回，而非格式化字符串。
pub fn fix_to_value(input: &str) -> FixResult {
    match parse_lenient(input) {
        Ok(output) => FixResult {
            value: Some(output.value),
            repairs: output.repairs,
            errors: Vec::new(),
        },
        Err(e) => FixResult {
            value: None,
            repairs: Vec::new(),
            errors: vec![e],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod fix_to_value {
        use super::*;

        #[test]
        fn fixes_trailing_comma_in_object() {
            let result = fix_to_value(r#"{"a": 1,}"#);
            assert!(result.value.is_some());
            assert!(result.was_repaired());
            assert!(!result.has_unfixable());
        }

        #[test]
        fn fixes_trailing_comma_in_array() {
            let result = fix_to_value("[1, 2, 3,]");
            assert!(result.value.is_some());
            assert!(result.was_repaired());
        }

        #[test]
        fn fixes_single_quoted_strings() {
            let result = fix_to_value("{'key': 'val'}");
            assert!(result.value.is_some());
            assert!(result.was_repaired());
        }

        #[test]
        fn fixes_python_true_false_none() {
            let result = fix_to_value("[True, False, None]");
            assert!(result.value.is_some());
            assert_eq!(result.repairs.len(), 3);
        }

        #[test]
        fn returns_error_on_unterminated_string() {
            let result = fix_to_value(r#"{"a": "no end"#);
            assert!(result.value.is_none());
            assert!(result.has_unfixable());
        }

        #[test]
        fn valid_json_has_no_repairs() {
            let result = fix_to_value(r#"{"a": 1, "b": [1, 2, 3]}"#);
            assert!(result.value.is_some());
            assert!(!result.was_repaired());
        }

        #[test]
        fn strips_line_comments_from_jsonc() {
            let result = fix_to_value("// comment\n{\"a\": 1}");
            assert!(result.value.is_some());
        }

        #[test]
        fn strips_block_comments_from_jsonc() {
            let result = fix_to_value("/* comment */ {\"a\": 1}");
            assert!(result.value.is_some());
        }
    }
}
