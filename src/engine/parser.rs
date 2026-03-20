use indexmap::IndexMap;

use crate::engine::value::JsonValue;

/// 描述宽松解析过程中自动修复的一处问题。
#[derive(Debug, Clone)]
pub struct Repair {
    pub line: usize,
    pub col: usize,
    pub description: String,
}

/// 宽松解析的输出结果。
pub struct ParseOutput {
    pub value: JsonValue,
    pub repairs: Vec<Repair>,
}

/// 解析错误。
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ParseError {
    #[error("第 {line} 行第 {col} 列：意外的字符 '{ch}'")]
    UnexpectedChar { ch: char, line: usize, col: usize },

    #[error("意外的输入结束")]
    UnexpectedEof,

    #[error("第 {line} 行第 {col} 列：无效的数字")]
    InvalidNumber { line: usize, col: usize },

    #[error("第 {line} 行第 {col} 列：未终止的字符串")]
    UnterminatedString { line: usize, col: usize },
}

/// 严格模式解析：使用 serde_json，仅接受合法 JSON。
pub fn parse_strict(input: &str) -> Result<JsonValue, ParseError> {
    let v: serde_json::Value =
        serde_json::from_str(input).map_err(|e| ParseError::UnexpectedChar {
            ch: '?',
            line: e.line(),
            col: e.column(),
        })?;
    Ok(JsonValue::from(v))
}

/// 宽松模式解析：容忍 JSONC 注释、尾部逗号、单引号、未加引号的 key、Python 字面量等。
pub fn parse_lenient(input: &str) -> Result<ParseOutput, ParseError> {
    // 剥离 UTF-8 BOM
    let input = input.strip_prefix('\u{FEFF}').unwrap_or(input);
    let mut parser = LenientParser::new(input.as_bytes());
    let value = parser.parse_root()?;
    Ok(ParseOutput {
        value,
        repairs: parser.repairs,
    })
}

// ── 内部实现 ─────────────────────────────────────────────────────────────────

struct LenientParser<'a> {
    input: &'a [u8],
    pos: usize,
    line: usize,
    col: usize,
    repairs: Vec<Repair>,
}

impl<'a> LenientParser<'a> {
    fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            pos: 0,
            line: 1,
            col: 1,
            repairs: Vec::new(),
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn peek2(&self) -> Option<u8> {
        self.input.get(self.pos + 1).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let b = self.input.get(self.pos).copied()?;
        self.pos += 1;
        if b == b'\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(b)
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            // 跳过空白
            while matches!(
                self.peek(),
                Some(b' ') | Some(b'\t') | Some(b'\r') | Some(b'\n')
            ) {
                self.advance();
            }

            // 行注释 //
            if self.peek() == Some(b'/') && self.peek2() == Some(b'/') {
                while !matches!(self.peek(), Some(b'\n') | None) {
                    self.advance();
                }
                continue;
            }

            // 块注释 /* ... */
            if self.peek() == Some(b'/') && self.peek2() == Some(b'*') {
                self.advance(); // /
                self.advance(); // *
                loop {
                    match self.peek() {
                        None => break,
                        Some(b'*') if self.peek2() == Some(b'/') => {
                            self.advance(); // *
                            self.advance(); // /
                            break;
                        }
                        _ => {
                            self.advance();
                        }
                    }
                }
                continue;
            }

            break;
        }
    }

    fn parse_root(&mut self) -> Result<JsonValue, ParseError> {
        self.skip_ws_and_comments();
        let value = self.parse_value()?;
        // 根层级允许有尾部注释/空白，不视为错误
        Ok(value)
    }

    fn parse_value(&mut self) -> Result<JsonValue, ParseError> {
        self.skip_ws_and_comments();
        match self.peek() {
            None => Err(ParseError::UnexpectedEof),
            Some(b'{') => self.parse_object(),
            Some(b'[') => self.parse_array(),
            Some(b'"') => Ok(JsonValue::String(self.parse_string(b'"')?)),
            Some(b'\'') => {
                let line = self.line;
                let col = self.col;
                let s = self.parse_string(b'\'')?;
                self.repairs.push(Repair {
                    line,
                    col,
                    description: "单引号字符串替换为双引号".into(),
                });
                Ok(JsonValue::String(s))
            }
            Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
            Some(_) => self.parse_keyword_or_unquoted_value(),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, ParseError> {
        self.advance(); // {
        let mut map = IndexMap::new();

        loop {
            self.skip_ws_and_comments();

            // 对象结束（同时处理尾部逗号后的 `}` 情况）
            if self.peek() == Some(b'}') {
                self.advance();
                return Ok(JsonValue::Object(map));
            }
            if self.peek().is_none() {
                return Err(ParseError::UnexpectedEof);
            }

            let key = self.parse_object_key()?;

            self.skip_ws_and_comments();

            // 期望 ':'
            match self.peek() {
                Some(b':') => {
                    self.advance();
                }
                None => return Err(ParseError::UnexpectedEof),
                Some(ch) => {
                    return Err(ParseError::UnexpectedChar {
                        ch: char::from(ch),
                        line: self.line,
                        col: self.col,
                    });
                }
            }

            self.skip_ws_and_comments();
            let val = self.parse_value()?;
            map.insert(key, val);

            self.skip_ws_and_comments();

            match self.peek() {
                Some(b',') => {
                    let comma_line = self.line;
                    let comma_col = self.col;
                    self.advance();
                    self.skip_ws_and_comments();
                    // 尾部逗号：逗号后紧跟 `}` 则记录修复，由循环顶部处理
                    if self.peek() == Some(b'}') {
                        self.repairs.push(Repair {
                            line: comma_line,
                            col: comma_col,
                            description: "移除对象尾部逗号".into(),
                        });
                    }
                }
                Some(b'}') => { /* 由循环顶部处理 */ }
                None => return Err(ParseError::UnexpectedEof),
                Some(_) => {
                    // 缺少逗号，记录修复并继续（不 advance，让循环重新解析下一 key）
                    self.repairs.push(Repair {
                        line: self.line,
                        col: self.col,
                        description: "插入缺失的逗号".into(),
                    });
                }
            }
        }
    }

    fn parse_object_key(&mut self) -> Result<String, ParseError> {
        match self.peek() {
            Some(b'"') => self.parse_string(b'"'),
            Some(b'\'') => {
                let line = self.line;
                let col = self.col;
                let s = self.parse_string(b'\'')?;
                self.repairs.push(Repair {
                    line,
                    col,
                    description: format!("key 单引号替换为双引号: {s}"),
                });
                Ok(s)
            }
            Some(_) => {
                let line = self.line;
                let col = self.col;
                let key = self.parse_unquoted_key()?;
                self.repairs.push(Repair {
                    line,
                    col,
                    description: format!("未加引号的 key 加引号: {key}"),
                });
                Ok(key)
            }
            None => Err(ParseError::UnexpectedEof),
        }
    }

    fn parse_array(&mut self) -> Result<JsonValue, ParseError> {
        self.advance(); // [
        let mut arr = Vec::new();

        loop {
            self.skip_ws_and_comments();

            if self.peek() == Some(b']') {
                self.advance();
                return Ok(JsonValue::Array(arr));
            }
            if self.peek().is_none() {
                return Err(ParseError::UnexpectedEof);
            }

            let val = self.parse_value()?;
            arr.push(val);

            self.skip_ws_and_comments();

            match self.peek() {
                Some(b',') => {
                    let comma_line = self.line;
                    let comma_col = self.col;
                    self.advance();
                    self.skip_ws_and_comments();
                    // 尾部逗号：逗号后紧跟 `]` 则记录修复
                    if self.peek() == Some(b']') {
                        self.repairs.push(Repair {
                            line: comma_line,
                            col: comma_col,
                            description: "移除数组尾部逗号".into(),
                        });
                    }
                }
                Some(b']') => { /* 由循环顶部处理 */ }
                None => return Err(ParseError::UnexpectedEof),
                Some(_) => {
                    self.repairs.push(Repair {
                        line: self.line,
                        col: self.col,
                        description: "插入缺失的逗号".into(),
                    });
                }
            }
        }
    }

    fn parse_string(&mut self, quote: u8) -> Result<String, ParseError> {
        let line = self.line;
        let col = self.col;
        self.advance(); // 开引号

        let mut s = String::new();

        loop {
            let byte = match self.peek() {
                None => return Err(ParseError::UnterminatedString { line, col }),
                Some(b'\n') if quote == b'"' => {
                    return Err(ParseError::UnterminatedString { line, col });
                }
                Some(b) => b,
            };

            if byte == quote {
                self.advance(); // 闭引号
                break;
            }

            if byte == b'\\' {
                self.advance(); // 反斜杠
                match self.advance() {
                    None => return Err(ParseError::UnterminatedString { line, col }),
                    Some(b'"') => s.push('"'),
                    Some(b'\'') => s.push('\''),
                    Some(b'\\') => s.push('\\'),
                    Some(b'/') => s.push('/'),
                    Some(b'n') => s.push('\n'),
                    Some(b'r') => s.push('\r'),
                    Some(b't') => s.push('\t'),
                    Some(b'b') => s.push('\x08'),
                    Some(b'f') => s.push('\x0C'),
                    Some(b'u') => {
                        let mut hex = [0u8; 4];
                        for h in &mut hex {
                            *h = self.advance().unwrap_or(b'0');
                        }
                        let hex_str = std::str::from_utf8(&hex).unwrap_or("0000");
                        let code = u32::from_str_radix(hex_str, 16).unwrap_or(0xFFFD);
                        s.push(char::from_u32(code).unwrap_or('\u{FFFD}'));
                    }
                    Some(b) => {
                        // 未知转义序列，原样保留
                        s.push('\\');
                        s.push(char::from(b));
                    }
                }
                continue;
            }

            self.advance();
            if byte < 0x80 {
                s.push(char::from(byte));
            } else {
                // 多字节 UTF-8：收集首字节及其后续连续字节
                let mut utf8 = vec![byte];
                while let Some(next) = self.peek() {
                    if (next & 0xC0) != 0x80 {
                        break; // 非连续字节，下一个字符开始
                    }
                    self.advance();
                    utf8.push(next);
                }
                if let Ok(ch_str) = std::str::from_utf8(&utf8) {
                    s.push_str(ch_str);
                }
                // 无效 UTF-8 静默跳过（输入来自 Rust &str，理论上不会出现）
            }
        }

        Ok(s)
    }

    fn parse_number(&mut self) -> Result<JsonValue, ParseError> {
        let start = self.pos;
        let line = self.line;
        let col = self.col;

        if self.peek() == Some(b'-') {
            self.advance();
        }
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.advance();
        }
        if self.peek() == Some(b'.') {
            self.advance();
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.advance();
            }
        }
        if matches!(self.peek(), Some(b'e') | Some(b'E')) {
            self.advance();
            if matches!(self.peek(), Some(b'+') | Some(b'-')) {
                self.advance();
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.advance();
            }
        }

        let num_str =
            std::str::from_utf8(&self.input[start..self.pos]).map_err(|_| {
                ParseError::InvalidNumber { line, col }
            })?;

        let n: f64 = num_str
            .parse()
            .map_err(|_| ParseError::InvalidNumber { line, col })?;

        Ok(JsonValue::Number(n))
    }

    fn parse_keyword_or_unquoted_value(&mut self) -> Result<JsonValue, ParseError> {
        let start = self.pos;
        let line = self.line;
        let col = self.col;

        while matches!(
            self.peek(),
            Some(b'a'..=b'z') | Some(b'A'..=b'Z') | Some(b'_') | Some(b'0'..=b'9')
        ) {
            self.advance();
        }

        let word = std::str::from_utf8(&self.input[start..self.pos])
            .map_err(|_| ParseError::UnexpectedChar { ch: '?', line, col })?;

        match word {
            "true" => Ok(JsonValue::Bool(true)),
            "false" => Ok(JsonValue::Bool(false)),
            "null" => Ok(JsonValue::Null),
            "True" => {
                self.repairs.push(Repair {
                    line,
                    col,
                    description: "Python True → true".into(),
                });
                Ok(JsonValue::Bool(true))
            }
            "False" => {
                self.repairs.push(Repair {
                    line,
                    col,
                    description: "Python False → false".into(),
                });
                Ok(JsonValue::Bool(false))
            }
            "None" | "undefined" => {
                self.repairs.push(Repair {
                    line,
                    col,
                    description: format!("{word} → null"),
                });
                Ok(JsonValue::Null)
            }
            "" => Err(ParseError::UnexpectedChar {
                ch: char::from(self.peek().unwrap_or(b'?')),
                line,
                col,
            }),
            other => {
                self.repairs.push(Repair {
                    line,
                    col,
                    description: format!("未加引号的值 {other:?} 转换为字符串"),
                });
                Ok(JsonValue::String(other.to_string()))
            }
        }
    }

    fn parse_unquoted_key(&mut self) -> Result<String, ParseError> {
        let start = self.pos;
        let line = self.line;
        let col = self.col;

        while matches!(
            self.peek(),
            Some(b'a'..=b'z')
                | Some(b'A'..=b'Z')
                | Some(b'_')
                | Some(b'$')
                | Some(b'0'..=b'9')
                | Some(b'-')
        ) {
            self.advance();
        }

        let key = std::str::from_utf8(&self.input[start..self.pos])
            .map_err(|_| ParseError::UnexpectedChar { ch: '?', line, col })?
            .to_string();

        if key.is_empty() {
            return Err(ParseError::UnexpectedChar {
                ch: char::from(self.peek().unwrap_or(b'?')),
                line,
                col,
            });
        }

        Ok(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    mod parse_strict {
        use super::*;

        #[test]
        fn parses_valid_json_object() {
            let v = parse_strict(r#"{"a": 1}"#).unwrap();
            assert_eq!(v.type_name(), "object");
        }

        #[test]
        fn returns_error_on_trailing_comma() {
            assert!(parse_strict(r#"{"a": 1,}"#).is_err());
        }
    }

    mod parse_lenient {
        use super::*;

        #[test]
        fn accepts_trailing_comma_in_object() {
            let out = parse_lenient(r#"{"a": 1,}"#).unwrap();
            assert!(out.value.as_object().is_some());
            assert_eq!(out.repairs.len(), 1);
        }

        #[test]
        fn accepts_trailing_comma_in_array() {
            let out = parse_lenient(r#"[1, 2,]"#).unwrap();
            assert_eq!(out.value.as_array().unwrap().len(), 2);
        }

        #[test]
        fn accepts_line_comment() {
            let out = parse_lenient("// comment\n{\"a\": 1}").unwrap();
            assert!(out.value.as_object().is_some());
        }

        #[test]
        fn accepts_block_comment() {
            let out = parse_lenient("/* comment */ {\"a\": 1}").unwrap();
            assert!(out.value.as_object().is_some());
        }

        #[test]
        fn accepts_single_quoted_string() {
            let out = parse_lenient("{'key': 'val'}").unwrap();
            let obj = out.value.as_object().unwrap();
            assert_eq!(obj.get("key").unwrap().as_str(), Some("val"));
            assert!(!out.repairs.is_empty());
        }

        #[test]
        fn accepts_unquoted_keys() {
            let out = parse_lenient("{key: \"val\"}").unwrap();
            let obj = out.value.as_object().unwrap();
            assert!(obj.contains_key("key"));
            assert!(!out.repairs.is_empty());
        }

        #[test]
        fn converts_python_true_to_bool() {
            let out = parse_lenient("[True, False, None]").unwrap();
            let arr = out.value.as_array().unwrap();
            assert_eq!(arr[0], JsonValue::Bool(true));
            assert_eq!(arr[1], JsonValue::Bool(false));
            assert_eq!(arr[2], JsonValue::Null);
        }

        #[test]
        fn strips_bom() {
            let input = "\u{FEFF}{\"a\": 1}";
            let out = parse_lenient(input).unwrap();
            assert!(out.value.as_object().is_some());
        }

        #[test]
        fn returns_error_on_unterminated_string() {
            assert!(parse_lenient(r#"{"a": "unterminated}"#).is_err());
        }
    }
}
