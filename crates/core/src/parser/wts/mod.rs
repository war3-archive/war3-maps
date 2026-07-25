use std::collections::HashMap;

use super::error::ParserError;

/// String table (`war3map.wts`)
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct War3MapWts {
    pub string_map: HashMap<i32, String>,
}

impl War3MapWts {
    /// Parse a WTS buffer.
    ///
    /// Handles World Editor quirks:
    /// - optional UTF-8 BOM
    /// - `//` comment lines between `STRING id` and `{`
    /// - both `\r\n` and `\n` line endings
    /// - multi-line bodies
    pub fn load(buffer: &str) -> Result<Self, ParserError> {
        let text = buffer.strip_prefix('\u{feff}').unwrap_or(buffer);
        let bytes = text.as_bytes();
        let mut string_map = HashMap::new();
        let mut i = 0usize;

        while i < bytes.len() {
            // Find next STRING keyword at line start (or file start)
            let rest = &text[i..];
            let Some(rel) = find_string_keyword(rest) else {
                break;
            };
            i += rel;

            // Skip "STRING"
            i += "STRING".len();
            skip_ws_inline(text, &mut i);

            // Parse id
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

            // Skip rest of line (and any // comment lines) until '{'
            if !skip_until_brace(text, &mut i) {
                break;
            }
            // consume '{'
            i += 1;

            // Body until matching '}' at beginning of a line (WE style) or first lone '}'
            let body = read_brace_body(text, &mut i);
            string_map.insert(id, trim_wts_body(&body));
        }

        Ok(Self { string_map })
    }
}

fn find_string_keyword(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        // Match STRING at start or after newline
        if matches_keyword_at(bytes, i, b"STRING") {
            // Ensure it's a whole word
            let after = i + 6;
            let ok_before = i == 0 || bytes[i - 1] == b'\n' || bytes[i - 1] == b'\r';
            let ok_after = after >= bytes.len()
                || bytes[after].is_ascii_whitespace()
                || bytes[after].is_ascii_digit();
            if ok_before && ok_after {
                return Some(i);
            }
        }
        i += 1;
    }
    None
}

fn matches_keyword_at(bytes: &[u8], i: usize, kw: &[u8]) -> bool {
    i + kw.len() <= bytes.len() && &bytes[i..i + kw.len()] == kw
}

fn skip_ws_inline(text: &str, i: &mut usize) {
    let bytes = text.as_bytes();
    while *i < bytes.len() && (bytes[*i] == b' ' || bytes[*i] == b'\t') {
        *i += 1;
    }
}

fn skip_until_brace(text: &str, i: &mut usize) -> bool {
    let bytes = text.as_bytes();
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
    // Optional leading newline after '{'
    if *i < bytes.len() && bytes[*i] == b'\r' {
        *i += 1;
    }
    if *i < bytes.len() && bytes[*i] == b'\n' {
        *i += 1;
    }

    let start = *i;
    while *i < bytes.len() {
        if bytes[*i] == b'}' {
            // Prefer WE style: '}' alone on a line
            let at_line_start = *i == start
                || bytes[*i - 1] == b'\n'
                || (bytes[*i - 1] == b'\r' && (*i < 2 || bytes[*i - 2] == b'\n'));
            if at_line_start {
                let body = text[start..*i].to_string();
                *i += 1;
                return body;
            }
            // Fallback: first '}' closes (bodies rarely contain '}')
            let body = text[start..*i].to_string();
            *i += 1;
            return body;
        }
        *i += 1;
    }
    text[start..].to_string()
}

fn trim_wts_body(body: &str) -> String {
    // Trim one trailing newline commonly present before '}'
    let mut s = body.to_string();
    if s.ends_with("\r\n") {
        s.truncate(s.len() - 2);
    } else if s.ends_with('\n') {
        s.pop();
        if s.ends_with('\r') {
            s.pop();
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_comment_and_lf() {
        let raw = "STRING 1\n// comment\n{\nHello\n}\nSTRING 2\r\n{\r\nWorld\r\n}\r\n";
        let wts = War3MapWts::load(raw).unwrap();
        assert_eq!(wts.string_map.get(&1).map(String::as_str), Some("Hello"));
        assert_eq!(wts.string_map.get(&2).map(String::as_str), Some("World"));
    }

    #[test]
    fn parses_bom() {
        let raw = "\u{feff}STRING 7\n{\nX\n}\n";
        let wts = War3MapWts::load(raw).unwrap();
        assert_eq!(wts.string_map.get(&7).map(String::as_str), Some("X"));
    }
}
