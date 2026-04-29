use std::collections::BTreeMap;

use anyhow::{anyhow, bail};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ErrorEntry {
    pub emotion: String,
    pub cause: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub button: Option<String>,
}

pub fn extract_catalog_from_typescript(
    source: &str,
) -> anyhow::Result<BTreeMap<String, ErrorEntry>> {
    let marker = "export const CATALOG";
    let start = source
        .find(marker)
        .ok_or_else(|| anyhow!("CATALOG export not found"))?;
    let object_start = source[start..]
        .find('{')
        .map(|i| start + i)
        .ok_or_else(|| anyhow!("CATALOG object start not found"))?;
    let object_start_char = source[..object_start + 1].chars().count();
    let mut parser = Parser::new(source, object_start_char);
    let mut out = BTreeMap::new();

    loop {
        parser.skip_ws_comments_commas();
        if parser.peek() == Some('}') {
            break;
        }
        let key = parser.parse_string()?;
        parser.skip_ws_comments();
        parser.expect(':')?;
        parser.skip_ws_comments();
        parser.expect('{')?;

        let mut emotion = None;
        let mut cause = None;
        let mut action = None;
        let mut button = None;
        loop {
            parser.skip_ws_comments_commas();
            if parser.peek() == Some('}') {
                parser.bump();
                break;
            }
            let field = parser.parse_identifier_or_string()?;
            parser.skip_ws_comments();
            parser.expect(':')?;
            parser.skip_ws_comments();
            let value = parser.parse_string()?;
            match field.as_str() {
                "emotion" => emotion = Some(value),
                "cause" => cause = Some(value),
                "action" => action = Some(value),
                "button" => button = Some(value),
                _ => {}
            }
            parser.skip_ws_comments();
            if parser.peek() == Some(',') {
                parser.bump();
            }
        }
        out.insert(
            key,
            ErrorEntry {
                emotion: emotion.ok_or_else(|| anyhow!("catalog entry missing emotion"))?,
                cause: cause.ok_or_else(|| anyhow!("catalog entry missing cause"))?,
                action: action.ok_or_else(|| anyhow!("catalog entry missing action"))?,
                button,
            },
        );
        parser.skip_ws_comments();
        if parser.peek() == Some(',') {
            parser.bump();
        }
    }

    if out.is_empty() {
        bail!("catalog extraction produced zero entries");
    }
    Ok(out)
}

pub fn generate_catalog_json(source: &str) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(
        &extract_catalog_from_typescript(source)?,
    )?)
}

struct Parser<'a> {
    chars: Vec<char>,
    pos: usize,
    _source: &'a str,
}

impl<'a> Parser<'a> {
    fn new(source: &'a str, pos: usize) -> Self {
        Self {
            chars: source.chars().collect(),
            pos,
            _source: source,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }
    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }
    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    fn skip_ws_comments(&mut self) {
        loop {
            while matches!(self.peek(), Some(c) if c.is_whitespace()) {
                self.pos += 1;
            }
            if self.peek() == Some('/') && self.peek_next() == Some('/') {
                while !matches!(self.peek(), None | Some('\n')) {
                    self.pos += 1;
                }
                continue;
            }
            if self.peek() == Some('/') && self.peek_next() == Some('*') {
                self.pos += 2;
                while !(self.peek() == Some('*') && self.peek_next() == Some('/')) {
                    if self.bump().is_none() {
                        return;
                    }
                }
                self.pos += 2;
                continue;
            }
            break;
        }
    }

    fn skip_ws_comments_commas(&mut self) {
        loop {
            self.skip_ws_comments();
            if self.peek() == Some(',') {
                self.pos += 1;
                continue;
            }
            break;
        }
    }

    fn expect(&mut self, expected: char) -> anyhow::Result<()> {
        match self.bump() {
            Some(c) if c == expected => Ok(()),
            got => bail!("expected {expected:?}, got {got:?} at char {}", self.pos),
        }
    }

    fn parse_identifier_or_string(&mut self) -> anyhow::Result<String> {
        self.skip_ws_comments();
        if matches!(self.peek(), Some('"' | '\'')) {
            return self.parse_string();
        }
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c == '_' || c == '$' || c.is_ascii_alphanumeric()) {
            self.pos += 1;
        }
        if self.pos == start {
            bail!("expected identifier at char {}", self.pos);
        }
        Ok(self.chars[start..self.pos].iter().collect())
    }

    fn parse_string(&mut self) -> anyhow::Result<String> {
        let quote = self.bump().ok_or_else(|| anyhow!("expected string"))?;
        if quote != '"' && quote != '\'' {
            bail!("expected string quote, got {quote:?}");
        }
        let mut out = String::new();
        loop {
            let c = self.bump().ok_or_else(|| anyhow!("unterminated string"))?;
            if c == quote {
                break;
            }
            if c == '\\' {
                let e = self.bump().ok_or_else(|| anyhow!("unterminated escape"))?;
                match e {
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    '\\' => out.push('\\'),
                    '"' => out.push('"'),
                    '\'' => out.push('\''),
                    other => out.push(other),
                }
            } else {
                out.push(c);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_catalog_entries() {
        let src = r#"/* 한국어 주석이 CATALOG 앞에 있어도 byte index 와 char index 가 섞이면 안 돼요. */ export const CATALOG = { "0": { emotion: "축하해요", cause: "원인", action: '해결 "ok"', button: "닫기" } };"#;
        let got = extract_catalog_from_typescript(src).unwrap();
        assert_eq!(got["0"].emotion, "축하해요");
        assert_eq!(got["0"].action, "해결 \"ok\"");
        assert_eq!(got["0"].button.as_deref(), Some("닫기"));
    }

    #[test]
    fn extracts_entries_with_comments_commas_escapes_and_quoted_fields() {
        let src = r#"
          // leading comment
          export const CATALOG = {
            ,
            // inner comment
            "64": {
              /* block */
              "emotion": "잠깐만요",
              cause: "첫 줄\n둘째 줄",
              action: "a\rb\tc\\d\"e\'f\q",
              extra: "ignored",
            },
            "65": {
              emotion: "다시 로그인",
              cause: "auth",
              action: "axhub auth login",
              button: "로그인"
            },
          } as const;
        "#;
        let got = extract_catalog_from_typescript(src).unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got["64"].cause, "첫 줄\n둘째 줄");
        assert_eq!(got["64"].action, "a\rb\tc\\d\"e'fq");
        assert_eq!(got["65"].button.as_deref(), Some("로그인"));
    }

    #[test]
    fn generate_catalog_json_is_pretty_json_for_build_script_consumers() {
        let src =
            r#"export const CATALOG = { "0": { emotion: "ok", cause: "why", action: "do" } };"#;
        let json = generate_catalog_json(src).unwrap();
        assert!(json.contains("\"emotion\": \"ok\""));
        let parsed: BTreeMap<String, ErrorEntry> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["0"].cause, "why");
        assert_eq!(parsed["0"].button, None);
    }

    #[test]
    fn reports_actionable_parse_errors_for_missing_catalog_and_bad_shapes() {
        assert!(extract_catalog_from_typescript("const OTHER = {};")
            .unwrap_err()
            .to_string()
            .contains("CATALOG export not found"));
        assert!(
            extract_catalog_from_typescript("export const CATALOG = [];")
                .unwrap_err()
                .to_string()
                .contains("CATALOG object start not found")
        );
        assert!(extract_catalog_from_typescript(
            r#"export const CATALOG = { "0": { emotion: "ok", action: "do" } };"#
        )
        .unwrap_err()
        .to_string()
        .contains("missing cause"));
        assert!(
            extract_catalog_from_typescript(r#"export const CATALOG = { };"#)
                .unwrap_err()
                .to_string()
                .contains("zero entries")
        );
    }

    #[test]
    fn rejects_unterminated_strings_and_invalid_field_names() {
        assert!(extract_catalog_from_typescript(
            r#"export const CATALOG = { "0": { emotion: "ok } };"#
        )
        .unwrap_err()
        .to_string()
        .contains("unterminated string"));
        assert!(extract_catalog_from_typescript(
            r#"export const CATALOG = { "0": { -bad: "bad", emotion: "ok", cause: "why", action: "do" } };"#
        )
        .unwrap_err()
        .to_string()
        .contains("expected identifier"));
    }
}
