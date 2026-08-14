//! `war3map.wts` — TRIGSTR string table.

use std::collections::HashMap;

use crate::error::Result;

/// Parse a `TRIGSTR_<id>` reference, returning the id.
///
/// Accepts leading zeros, a negative sign, surrounding whitespace, and
/// trailing junk after the digits (all seen in real maps, e.g. `TRIGSTR_007ab`).
pub fn trigstr_id(text: &str) -> Option<i32> {
    let rest = text.trim().strip_prefix("TRIGSTR_")?;
    let digits_len = rest
        .char_indices()
        .take_while(|&(i, c)| c.is_ascii_digit() || (i == 0 && c == '-'))
        .count();
    rest[..digits_len].parse().ok()
}

/// String table parsed from `war3map.wts`.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Default)]
pub struct War3MapWts {
    pub string_map: HashMap<i32, String>,
}

impl War3MapWts {
    /// Parse a WTS text buffer.
    ///
    /// Handles World Editor quirks:
    /// - optional UTF-8 BOM
    /// - `//` comment lines between `STRING id` and `{`
    /// - both `\r\n` and `\n` line endings
    /// - multi-line bodies
    pub fn parse(buffer: &str) -> Result<Self> {
        let text = buffer.strip_prefix('\u{feff}').unwrap_or(buffer);
        let bytes = text.as_bytes();
        let mut string_map = HashMap::new();
        let mut i = 0usize;

        while i < bytes.len() {
            let Some(rel) = find_string_keyword(&text[i..]) else {
                break;
            };
            i += rel + "STRING".len();
            skip_ws_inline(bytes, &mut i);

            let id_start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if id_start == i {
                continue;
            }
            let Ok(id) = text[id_start..i].parse::<i32>() else {
                continue;
            };

            if !skip_until_brace(bytes, &mut i) {
                break;
            }
            i += 1; // consume '{'

            let body = read_brace_body(text, &mut i);
            string_map.insert(id, trim_wts_body(&body));
        }

        Ok(Self { string_map })
    }

    /// Look up a string by id.
    pub fn get(&self, id: i32) -> Option<&str> {
        self.string_map.get(&id).map(String::as_str)
    }

    /// Resolve a `TRIGSTR_<id>` reference; returns `None` when `text` is not
    /// a TRIGSTR reference or the id is missing from the table.
    pub fn resolve(&self, text: &str) -> Option<&str> {
        self.get(trigstr_id(text)?)
    }
}

fn find_string_keyword(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i..].starts_with(b"STRING") {
            let after = i + "STRING".len();
            let ok_before = i == 0 || bytes[i - 1] == b'\n' || bytes[i - 1] == b'\r';
            let ok_after = after >= bytes.len()
                || bytes[after].is_ascii_whitespace()
                || bytes[after].is_ascii_digit();
            if ok_before && ok_after {
                return Some(i);
            }
        }
    }
    None
}

fn skip_ws_inline(bytes: &[u8], i: &mut usize) {
    while *i < bytes.len() && (bytes[*i] == b' ' || bytes[*i] == b'\t') {
        *i += 1;
    }
}

fn skip_until_brace(bytes: &[u8], i: &mut usize) -> bool {
    while *i < bytes.len() {
        if bytes[*i] == b'{' {
            return true;
        }
        *i += 1;
    }
    false
}

fn read_brace_body(text: &str, i: &mut usize) -> String {
    let bytes = text.as_bytes();
    if *i < bytes.len() && bytes[*i] == b'\r' {
        *i += 1;
    }
    if *i < bytes.len() && bytes[*i] == b'\n' {
        *i += 1;
    }

    let start = *i;
    while *i < bytes.len() {
        if bytes[*i] == b'}' {
            let body = text[start..*i].to_string();
            *i += 1;
            return body;
        }
        *i += 1;
    }
    text[start..].to_string()
}

fn trim_wts_body(body: &str) -> String {
    // Trim the single trailing newline that precedes '}'.
    body.strip_suffix('\n')
        .map(|s| s.strip_suffix('\r').unwrap_or(s))
        .unwrap_or(body)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_comment_and_lf() {
        let raw = "STRING 1\n// comment\n{\nHello\n}\nSTRING 2\r\n{\r\nWorld\r\n}\r\n";
        let wts = War3MapWts::parse(raw).unwrap();
        assert_eq!(wts.get(1), Some("Hello"));
        assert_eq!(wts.get(2), Some("World"));
    }

    #[test]
    fn parses_bom() {
        let raw = "\u{feff}STRING 7\n{\nX\n}\n";
        let wts = War3MapWts::parse(raw).unwrap();
        assert_eq!(wts.get(7), Some("X"));
    }

    #[test]
    fn multiline_body_is_preserved() {
        let raw = "STRING 3\n{\nline one\nline two\n}\n";
        let wts = War3MapWts::parse(raw).unwrap();
        assert_eq!(wts.get(3), Some("line one\nline two"));
    }

    #[test]
    fn trigstr_id_variants() {
        assert_eq!(trigstr_id("TRIGSTR_007"), Some(7));
        assert_eq!(trigstr_id("TRIGSTR_7"), Some(7));
        assert_eq!(trigstr_id("TRIGSTR_-3"), Some(-3));
        assert_eq!(trigstr_id("TRIGSTR_007ab"), Some(7));
        assert_eq!(trigstr_id(" TRIGSTR_12 "), Some(12));
        assert_eq!(trigstr_id("TRIGSTR_"), None);
        assert_eq!(trigstr_id("hello"), None);
    }

    #[test]
    fn resolve_falls_back_to_none() {
        let raw = "STRING 1\n{\nHi\n}\n";
        let wts = War3MapWts::parse(raw).unwrap();
        assert_eq!(wts.resolve("TRIGSTR_001"), Some("Hi"));
        assert_eq!(wts.resolve("TRIGSTR_9"), None);
        assert_eq!(wts.resolve("plain text"), None);
    }
}
