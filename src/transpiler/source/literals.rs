use crate::transpiler::error::TranspileError;
use crate::transpiler::source::collections::{
    innermost_container, rewrite_map_literal, should_rewrite_map, should_rewrite_vec,
};
use crate::transpiler::source::keywords::{is_identifier_continue, rewrite_word};
use crate::transpiler::source::scan::{
    find_matching, previous_significant_char, previous_significant_index, scan_block_comment,
    scan_char_or_lifetime, scan_string,
};

pub(crate) fn rewrite_literals(source: &str) -> Result<String, TranspileError> {
    let mut result = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        let ch = bytes[index] as char;
        match ch {
            '"' => {
                let end = scan_string(source, index)?;
                let literal = &source[index..end];
                result.push_str(&rewrite_string_literal(source, index, literal)?);
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
            '[' => {
                let end = find_matching(source, index, '[', ']')?;
                let inner = &source[index + 1..end - 1];
                if should_rewrite_vec(source, index) {
                    result.push_str("vec![");
                    result.push_str(&transpile_nested(inner)?);
                    result.push(']');
                } else {
                    result.push('[');
                    result.push_str(&transpile_nested(inner)?);
                    result.push(']');
                }
                index = end;
            }
            '{' => {
                let end = find_matching(source, index, '{', '}')?;
                let inner = &source[index + 1..end - 1];
                if should_rewrite_map(source, index, inner) {
                    result.push_str(&rewrite_map_literal(inner)?);
                } else {
                    result.push('{');
                    result.push_str(&transpile_nested(inner)?);
                    result.push('}');
                }
                index = end;
            }
            _ => {
                result.push(ch);
                index += 1;
            }
        }
    }

    Ok(result)
}

fn rewrite_string_literal(
    source: &str,
    index: usize,
    literal: &str,
) -> Result<String, TranspileError> {
    if should_keep_raw_string(source, index) {
        return Ok(literal.to_string());
    }

    let content = &literal[1..literal.len() - 1];
    let interpolations = extract_interpolations(content)?;
    if interpolations.is_empty() {
        return Ok(format!("String::from({literal})"));
    }

    let mut format_string = String::new();
    let mut arguments = Vec::with_capacity(interpolations.len());
    let mut cursor = 0usize;

    for interpolation in interpolations {
        format_string.push_str(&escape_format_segment(
            &content[cursor..interpolation.start],
        ));
        format_string.push_str("{}");
        arguments.push(transpile_nested(interpolation.expr.trim())?);
        cursor = interpolation.end;
    }

    format_string.push_str(&escape_format_segment(&content[cursor..]));
    Ok(format!(
        "format!(\"{format_string}\", {})",
        arguments.join(", ")
    ))
}

fn transpile_nested(source: &str) -> Result<String, TranspileError> {
    let source = rewrite_word(source, "string", "String");
    rewrite_literals(&source)
}

fn extract_interpolations(content: &str) -> Result<Vec<Interpolation>, TranspileError> {
    let mut items = Vec::new();
    let bytes = content.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        let ch = bytes[index] as char;
        if ch == '\\' {
            index += 2;
            continue;
        }

        if ch == '{' {
            let mut depth = 1usize;
            let mut end = index + 1;
            while end < bytes.len() {
                let inner = bytes[end] as char;
                if inner == '\\' {
                    end += 2;
                    continue;
                }
                if inner == '{' {
                    depth += 1;
                } else if inner == '}' {
                    depth -= 1;
                    if depth == 0 {
                        items.push(Interpolation {
                            start: index,
                            end: end + 1,
                            expr: content[index + 1..end].to_string(),
                        });
                        index = end + 1;
                        break;
                    }
                }
                end += 1;
            }

            if depth != 0 {
                return Err(TranspileError::UnclosedDelimiter('{'));
            }

            continue;
        }

        index += 1;
    }

    Ok(items)
}

fn escape_format_segment(segment: &str) -> String {
    segment.replace('{', "{{").replace('}', "}}")
}

fn should_keep_raw_string(source: &str, index: usize) -> bool {
    let prev = previous_significant_char(source, index);
    let prev_prev = previous_significant_char(
        source,
        previous_significant_index(source, index).unwrap_or(index),
    );

    if prev == Some('=') || prev == Some(':') {
        return false;
    }

    if prev == Some(',')
        && let Some(container) = innermost_container(source, index)
    {
        return match container.open {
            '(' => {
                matches!(container.before_open, Some('!') | Some(')') | Some(']'))
                    || container.before_open.is_some_and(is_identifier_continue)
            }
            '[' => {
                matches!(container.before_open, Some(')') | Some(']') | Some('"'))
                    || container.before_open.is_some_and(is_identifier_continue)
            }
            _ => false,
        };
    }

    if prev == Some('[')
        && matches!(
            prev_prev,
            Some(')') | Some(']') | Some('"') | Some('s') | Some('r')
        )
    {
        return true;
    }

    if prev == Some('(') {
        if prev_prev == Some('!') {
            return true;
        }

        if prev_prev.is_some_and(is_identifier_continue)
            || matches!(prev_prev, Some(')') | Some(']'))
        {
            return true;
        }
    }

    if prev == Some('!') || prev == Some('&') {
        return true;
    }

    false
}

struct Interpolation {
    start: usize,
    end: usize,
    expr: String,
}
