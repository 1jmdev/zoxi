use crate::transpiler::error::TranspileError;
use crate::transpiler::source::keywords::is_identifier_start;

pub(crate) fn find_matching(
    source: &str,
    start: usize,
    open: char,
    close: char,
) -> Result<usize, TranspileError> {
    let mut depth = 0usize;
    let bytes = source.as_bytes();
    let mut index = start;

    while index < bytes.len() {
        let ch = bytes[index] as char;
        match ch {
            '"' => index = scan_string(source, index)?,
            '\'' => index = scan_char_or_lifetime(source, index)?,
            '/' if source[index..].starts_with("//") => {
                index = source[index..]
                    .find('\n')
                    .map(|offset| index + offset)
                    .unwrap_or(source.len());
            }
            '/' if source[index..].starts_with("/*") => index = scan_block_comment(source, index)?,
            _ if ch == open => {
                depth += 1;
                index += 1;
            }
            _ if ch == close => {
                depth -= 1;
                index += 1;
                if depth == 0 {
                    return Ok(index);
                }
            }
            _ => index += 1,
        }
    }

    Err(TranspileError::UnclosedDelimiter(open))
}

pub(crate) fn scan_expression_end(source: &str, start: usize) -> Result<usize, TranspileError> {
    let mut depth_paren = 0usize;
    let mut depth_brace = 0usize;
    let mut depth_bracket = 0usize;
    let bytes = source.as_bytes();
    let mut index = start;

    while index < bytes.len() {
        let ch = bytes[index] as char;
        match ch {
            '"' => index = scan_string(source, index)?,
            '\'' => index = scan_char_or_lifetime(source, index)?,
            '/' if source[index..].starts_with("//") => {
                index = source[index..]
                    .find('\n')
                    .map(|offset| index + offset)
                    .unwrap_or(source.len());
            }
            '/' if source[index..].starts_with("/*") => index = scan_block_comment(source, index)?,
            '(' => {
                depth_paren += 1;
                index += 1;
            }
            ')' => {
                if depth_paren == 0 && depth_brace == 0 && depth_bracket == 0 {
                    return Ok(index);
                }
                depth_paren = depth_paren.saturating_sub(1);
                index += 1;
            }
            '[' => {
                depth_bracket += 1;
                index += 1;
            }
            ']' => {
                if depth_paren == 0 && depth_brace == 0 && depth_bracket == 0 {
                    return Ok(index);
                }
                depth_bracket = depth_bracket.saturating_sub(1);
                index += 1;
            }
            '{' => {
                depth_brace += 1;
                index += 1;
            }
            '}' if depth_paren == 0 && depth_brace == 0 && depth_bracket == 0 => return Ok(index),
            '}' => {
                depth_brace = depth_brace.saturating_sub(1);
                index += 1;
            }
            ';' | ',' if depth_paren == 0 && depth_brace == 0 && depth_bracket == 0 => return Ok(index),
            _ => index += 1,
        }
    }

    Ok(source.len())
}

pub(crate) fn scan_string(source: &str, start: usize) -> Result<usize, TranspileError> {
    let bytes = source.as_bytes();
    let mut index = start + 1;

    while index < bytes.len() {
        match bytes[index] as char {
            '\\' => index += 2,
            '"' => return Ok(index + 1),
            _ => index += 1,
        }
    }

    Err(TranspileError::UnclosedDelimiter('"'))
}

pub(crate) fn scan_char_or_lifetime(source: &str, start: usize) -> Result<usize, TranspileError> {
    let next = source[start + 1..].chars().next();
    if matches!(next, Some(ch) if is_identifier_start(ch)) {
        return Ok(start + 1 + next.map(char::len_utf8).unwrap_or(0));
    }

    let bytes = source.as_bytes();
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] as char {
            '\\' => index += 2,
            '\'' => return Ok(index + 1),
            _ => index += 1,
        }
    }

    Err(TranspileError::UnclosedDelimiter('\''))
}

pub(crate) fn scan_block_comment(source: &str, start: usize) -> Result<usize, TranspileError> {
    let mut index = start + 2;
    while index + 1 < source.len() {
        if &source[index..index + 2] == "*/" {
            return Ok(index + 2);
        }
        index += 1;
    }

    Err(TranspileError::UnclosedDelimiter('/'))
}

pub(crate) fn skip_whitespace(source: &str, mut index: usize) -> usize {
    let bytes = source.as_bytes();
    while index < bytes.len() && (bytes[index] as char).is_whitespace() {
        index += 1;
    }
    index
}

pub(crate) fn previous_significant_index(source: &str, index: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut current = index;
    while current > 0 {
        current -= 1;
        let ch = bytes[current] as char;
        if !ch.is_whitespace() {
            return Some(current);
        }
    }
    None
}

pub(crate) fn previous_significant_char(source: &str, index: usize) -> Option<char> {
    previous_significant_index(source, index).map(|position| source.as_bytes()[position] as char)
}

pub(crate) fn trim_end_index(source: &str) -> usize {
    let mut end = source.len();
    let bytes = source.as_bytes();
    while end > 0 && (bytes[end - 1] as char).is_whitespace() {
        end -= 1;
    }
    end
}
