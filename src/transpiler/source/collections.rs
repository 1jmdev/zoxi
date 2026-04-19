use regex::Regex;

use crate::transpiler::error::TranspileError;
use crate::transpiler::source::keywords::{is_identifier_start, scan_identifier};
use crate::transpiler::source::literals::rewrite_literals;
use crate::transpiler::source::scan::{
    find_matching, previous_significant_char, scan_block_comment, scan_char_or_lifetime,
    scan_expression_end, scan_string, skip_whitespace,
};

pub(crate) fn rewrite_hash_map_index_assignments(source: &str) -> Result<String, TranspileError> {
    let mut result = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        let ch = bytes[index] as char;
        match ch {
            '"' => {
                let end = scan_string(source, index)?;
                result.push_str(&source[index..end]);
                index = end;
            }
            '\'' => {
                let end = scan_char_or_lifetime(source, index)?;
                result.push_str(&source[index..end]);
                index = end;
            }
            '/' if source[index..].starts_with("//") => {
                let end = source[index..]
                    .find('\n')
                    .map(|offset| index + offset)
                    .unwrap_or(source.len());
                result.push_str(&source[index..end]);
                index = end;
            }
            '/' if source[index..].starts_with("/*") => {
                let end = scan_block_comment(source, index)?;
                result.push_str(&source[index..end]);
                index = end;
            }
            _ if is_identifier_start(ch) => {
                let ident_end = scan_identifier(source, index);
                let ident = &source[index..ident_end];
                let after_ident = skip_whitespace(source, ident_end);
                if source[after_ident..].starts_with('[') {
                    let key_end = find_matching(source, after_ident, '[', ']')?;
                    let after_key = skip_whitespace(source, key_end);
                    if source[after_key..].starts_with('=') {
                        let value_start = skip_whitespace(source, after_key + 1);
                        let value_end = scan_expression_end(source, value_start)?;
                        result.push_str(ident);
                        result.push_str(".insert(");
                        result.push_str(source[after_ident + 1..key_end - 1].trim());
                        result.push_str(", ");
                        result.push_str(source[value_start..value_end].trim());
                        result.push(')');
                        index = value_end;
                        continue;
                    }
                }

                result.push_str(ident);
                index = ident_end;
            }
            _ => {
                result.push(ch);
                index += 1;
            }
        }
    }

    Ok(result)
}

pub(crate) fn rewrite_string_bindings(source: &str) -> Result<String, TranspileError> {
    let binding = Regex::new(r"(?m)(let\s+(?:mut\s+)?[A-Za-z_][A-Za-z0-9_]*\s*:\s*String\s*=\s*)([^;]+);")
        .map_err(|_| TranspileError::UnclosedDelimiter('r'))?;

    Ok(binding
        .replace_all(source, |captures: &regex::Captures<'_>| {
            let rhs = captures.get(2).map(|m| m.as_str().trim()).unwrap_or_default();
            if rhs.starts_with("String::from(")
                || rhs.starts_with("format!(")
                || rhs.ends_with(".to_string()")
                || rhs == "String::new()"
            {
                captures.get(0).map(|m| m.as_str()).unwrap_or_default().to_string()
            } else {
                format!("{}String::from({rhs});", captures.get(1).map(|m| m.as_str()).unwrap_or_default())
            }
        })
        .into_owned())
}

pub(crate) fn rewrite_static_strings(source: &str) -> Result<String, TranspileError> {
    let static_string = Regex::new(
        r#"(?m)(static\s+[A-Za-z_][A-Za-z0-9_]*\s*:\s*)String(\s*=\s*)String::from\(("(?:\\.|[^"])*")\);"#,
    )
    .map_err(|_| TranspileError::UnclosedDelimiter('r'))?;

    Ok(static_string.replace_all(source, "$1&str$2$3;").into_owned())
}

pub(crate) fn rewrite_map_literal(inner: &str) -> Result<String, TranspileError> {
    if inner.trim().is_empty() {
        return Ok("std::collections::HashMap::new()".to_string());
    }

    let entries = split_top_level(inner, ',')?
        .into_iter()
        .filter(|entry| !entry.trim().is_empty())
        .map(|entry| {
            let parts = split_top_level(&entry, ':')?;
            let key = parts.first().map(|part| part.trim()).unwrap_or_default();
            let value = parts.get(1).map(|part| part.trim()).unwrap_or_default();
            Ok(format!("({}, {})", rewrite_literals(key)?, rewrite_literals(value)?))
        })
        .collect::<Result<Vec<_>, TranspileError>>()?;

    Ok(format!("std::collections::HashMap::from([{}])", entries.join(", ")))
}

pub(crate) fn should_rewrite_vec(source: &str, index: usize) -> bool {
    !top_level_contains(&source[index + 1..find_matching(source, index, '[', ']').unwrap_or(index + 1) - 1], ';')
        && matches!(previous_significant_char(source, index), None | Some('=') | Some('(') | Some(',') | Some('{') | Some('[') | Some(';'))
}

pub(crate) fn should_rewrite_map(source: &str, index: usize, inner: &str) -> bool {
    matches!(previous_significant_char(source, index), None | Some('=') | Some('(') | Some(',') | Some('[') | Some('{') | Some(';'))
        && (inner.trim().is_empty() || top_level_contains(inner, ':'))
}

pub(crate) fn innermost_container(source: &str, end: usize) -> Option<ContainerContext> {
    let mut stack = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0usize;

    while index < end && index < bytes.len() {
        let ch = bytes[index] as char;
        match ch {
            '"' => match scan_string(source, index) {
                Ok(next) => index = next,
                Err(_) => return None,
            },
            '\'' => match scan_char_or_lifetime(source, index) {
                Ok(next) => index = next,
                Err(_) => return None,
            },
            '/' if source[index..].starts_with("//") => {
                index = source[index..]
                    .find('\n')
                    .map(|offset| index + offset)
                    .unwrap_or(end);
            }
            '/' if source[index..].starts_with("/*") => match scan_block_comment(source, index) {
                Ok(next) => index = next,
                Err(_) => return None,
            },
            '(' | '[' | '{' => {
                stack.push(ContainerContext {
                    open: ch,
                    before_open: previous_significant_char(source, index),
                });
                index += 1;
            }
            ')' | ']' | '}' => {
                let _ = stack.pop();
                index += 1;
            }
            _ => index += 1,
        }
    }

    stack.pop()
}

fn top_level_contains(source: &str, needle: char) -> bool {
    let mut depth_paren = 0usize;
    let mut depth_brace = 0usize;
    let mut depth_bracket = 0usize;
    let bytes = source.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        let ch = bytes[index] as char;
        match ch {
            '"' => match scan_string(source, index) {
                Ok(end) => index = end,
                Err(_) => return false,
            },
            '\'' => match scan_char_or_lifetime(source, index) {
                Ok(end) => index = end,
                Err(_) => return false,
            },
            '(' => {
                depth_paren += 1;
                index += 1;
            }
            ')' => {
                depth_paren = depth_paren.saturating_sub(1);
                index += 1;
            }
            '[' => {
                depth_bracket += 1;
                index += 1;
            }
            ']' => {
                depth_bracket = depth_bracket.saturating_sub(1);
                index += 1;
            }
            '{' => {
                depth_brace += 1;
                index += 1;
            }
            '}' => {
                depth_brace = depth_brace.saturating_sub(1);
                index += 1;
            }
            _ if ch == needle && depth_paren == 0 && depth_brace == 0 && depth_bracket == 0 => return true,
            _ => index += 1,
        }
    }

    false
}

fn split_top_level(source: &str, delimiter: char) -> Result<Vec<String>, TranspileError> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth_paren = 0usize;
    let mut depth_brace = 0usize;
    let mut depth_bracket = 0usize;
    let bytes = source.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        let ch = bytes[index] as char;
        match ch {
            '"' => index = scan_string(source, index)?,
            '\'' => index = scan_char_or_lifetime(source, index)?,
            '(' => {
                depth_paren += 1;
                index += 1;
            }
            ')' => {
                depth_paren = depth_paren.saturating_sub(1);
                index += 1;
            }
            '[' => {
                depth_bracket += 1;
                index += 1;
            }
            ']' => {
                depth_bracket = depth_bracket.saturating_sub(1);
                index += 1;
            }
            '{' => {
                depth_brace += 1;
                index += 1;
            }
            '}' => {
                depth_brace = depth_brace.saturating_sub(1);
                index += 1;
            }
            _ if ch == delimiter && depth_paren == 0 && depth_brace == 0 && depth_bracket == 0 => {
                parts.push(source[start..index].to_string());
                index += 1;
                start = index;
            }
            _ => index += 1,
        }
    }

    parts.push(source[start..].to_string());
    Ok(parts)
}

pub(crate) struct ContainerContext {
    pub(crate) open: char,
    pub(crate) before_open: Option<char>,
}
