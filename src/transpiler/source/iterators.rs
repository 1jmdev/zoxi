use crate::transpiler::error::TranspileError;
use crate::transpiler::source::keywords::{
    is_identifier_continue, is_identifier_start, scan_identifier,
};
use crate::transpiler::source::scan::{
    find_matching, scan_block_comment, scan_char_or_lifetime, scan_string, skip_whitespace,
};

pub(crate) fn rewrite_iterator_helpers(source: &str) -> Result<String, TranspileError> {
    let mut result = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let mut index = 0usize;

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
                let end = line_comment_end(source, index);
                result.push_str(&source[index..end]);
                index = end;
            }
            '/' if source[index..].starts_with("/*") => {
                let end = scan_block_comment(source, index)?;
                result.push_str(&source[index..end]);
                index = end;
            }
            _ if is_expression_start(source, index) => {
                if let Some((end, rewritten)) = rewrite_chain_from(source, index)? {
                    result.push_str(&rewritten);
                    index = end;
                    continue;
                }

                result.push(ch);
                index += 1;
            }
            _ => {
                result.push(ch);
                index += 1;
            }
        }
    }

    Ok(result)
}

fn rewrite_chain_from(
    source: &str,
    start: usize,
) -> Result<Option<(usize, String)>, TranspileError> {
    let Some((receiver_end, first_call)) = find_first_helper_call(source, start)? else {
        return Ok(None);
    };
    let receiver = rewrite_iterator_helpers(source[start..receiver_end].trim())?;
    let mut rewritten = receiver;
    let iter_kind = iterator_kind(&rewritten);
    if matches!(iter_kind, IteratorKind::NeedsIter) {
        rewritten.push_str(".iter()");
    }
    let mut end = first_call.end;
    let mut next_call = Some(first_call);
    let mut last_kind = HelperKind::Find;
    while let Some(call) = next_call {
        rewritten.push_str(call.kind.rust_name());
        rewritten.push('(');
        rewritten.push_str(&rewrite_helper_argument(
            &call.argument,
            call.kind,
            iter_kind,
        )?);
        rewritten.push(')');
        end = call.end;
        last_kind = call.kind;

        let next_index = skip_whitespace(source, call.end);
        next_call = parse_helper_call(source, next_index)?;
    }
    if matches!(last_kind, HelperKind::Map | HelperKind::Filter) {
        rewritten.push_str(".collect()");
    }
    Ok(Some((end, rewritten)))
}

fn rewrite_helper_argument(
    argument: &str,
    kind: HelperKind,
    iter_kind: IteratorKind,
) -> Result<String, TranspileError> {
    if let Some((param, body)) = split_arrow(argument)? {
        let body = rewrite_iterator_helpers(body.trim())?;
        let param = normalize_closure_param(param);
        let body = rewrite_closure_body(&body, &param, kind.deref_count(iter_kind));
        return Ok(format!("|{}| {}", param, body));
    }

    rewrite_iterator_helpers(argument.trim())
}

fn rewrite_closure_body(body: &str, param: &str, deref_count: usize) -> String {
    if deref_count == 0 || !is_simple_param(param) {
        return body.to_string();
    }

    let mut result = String::with_capacity(body.len() + deref_count * 4);
    let bytes = body.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        let ch = bytes[index] as char;
        match ch {
            '"' => match scan_string(body, index) {
                Ok(end) => {
                    result.push_str(&body[index..end]);
                    index = end;
                }
                Err(_) => {
                    result.push_str(&body[index..]);
                    break;
                }
            },
            '\'' => match scan_char_or_lifetime(body, index) {
                Ok(end) => {
                    result.push_str(&body[index..end]);
                    index = end;
                }
                Err(_) => {
                    result.push_str(&body[index..]);
                    break;
                }
            },
            '/' if body[index..].starts_with("//") => {
                let end = body[index..]
                    .find('\n')
                    .map(|offset| index + offset)
                    .unwrap_or(body.len());
                result.push_str(&body[index..end]);
                index = end;
            }
            '/' if body[index..].starts_with("/*") => match scan_block_comment(body, index) {
                Ok(end) => {
                    result.push_str(&body[index..end]);
                    index = end;
                }
                Err(_) => {
                    result.push_str(&body[index..]);
                    break;
                }
            },
            _ if starts_with_param(body, index, param) => {
                push_deref_expr(&mut result, param, deref_count);
                index += param.len();
            }
            _ => {
                result.push(ch);
                index += 1;
            }
        }
    }

    result
}

fn is_simple_param(param: &str) -> bool {
    let trimmed = param.trim();
    let mut chars = trimmed.chars();
    matches!(chars.next(), Some(ch) if is_identifier_start(ch)) && chars.all(is_identifier_continue)
}

fn starts_with_param(body: &str, index: usize, param: &str) -> bool {
    body[index..].starts_with(param)
        && body[..index]
            .chars()
            .next_back()
            .is_none_or(|ch| !is_identifier_continue(ch))
        && body[index + param.len()..]
            .chars()
            .next()
            .is_none_or(|ch| !is_identifier_continue(ch))
}

fn push_deref_expr(result: &mut String, param: &str, deref_count: usize) {
    result.push('(');
    for _ in 0..deref_count {
        result.push('*');
    }
    result.push_str(param);
    result.push(')');
}

fn find_first_helper_call(
    source: &str,
    start: usize,
) -> Result<Option<(usize, HelperCall)>, TranspileError> {
    let bytes = source.as_bytes();
    let mut depth_paren = 0usize;
    let mut depth_brace = 0usize;
    let mut depth_bracket = 0usize;
    let mut index = start;

    while index < bytes.len() {
        let ch = bytes[index] as char;
        match ch {
            '"' => index = scan_string(source, index)?,
            '\'' => index = scan_char_or_lifetime(source, index)?,
            '/' if source[index..].starts_with("//") => index = line_comment_end(source, index),
            '/' if source[index..].starts_with("/*") => index = scan_block_comment(source, index)?,
            '(' => {
                depth_paren += 1;
                index += 1;
            }
            ')' => {
                if depth_paren == 0 && depth_brace == 0 && depth_bracket == 0 {
                    return Ok(None);
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
                    return Ok(None);
                }
                depth_bracket = depth_bracket.saturating_sub(1);
                index += 1;
            }
            '{' => {
                depth_brace += 1;
                index += 1;
            }
            '}' => {
                if depth_paren == 0 && depth_brace == 0 && depth_bracket == 0 {
                    return Ok(None);
                }
                depth_brace = depth_brace.saturating_sub(1);
                index += 1;
            }
            ';' | ',' if depth_paren == 0 && depth_brace == 0 && depth_bracket == 0 => {
                return Ok(None);
            }
            '.' if depth_paren == 0 && depth_brace == 0 && depth_bracket == 0 => {
                if let Some(call) = parse_helper_call(source, index)? {
                    return Ok(Some((index, call)));
                }
                index += 1;
            }
            _ => index += 1,
        }
    }

    Ok(None)
}

fn parse_helper_call(source: &str, dot_index: usize) -> Result<Option<HelperCall>, TranspileError> {
    if !source[dot_index..].starts_with('.') {
        return Ok(None);
    }
    let name_start = dot_index + 1;
    if name_start >= source.len() || !is_identifier_start(source.as_bytes()[name_start] as char) {
        return Ok(None);
    }
    let name_end = scan_identifier(source, name_start);
    let Some(kind) = HelperKind::from_name(&source[name_start..name_end]) else {
        return Ok(None);
    };
    let args_start = skip_whitespace(source, name_end);
    if !source[args_start..].starts_with('(') {
        return Ok(None);
    }
    let args_end = find_matching(source, args_start, '(', ')')?;
    Ok(Some(HelperCall {
        kind,
        argument: source[args_start + 1..args_end - 1].trim().to_string(),
        end: args_end,
    }))
}

fn split_arrow(argument: &str) -> Result<Option<(&str, &str)>, TranspileError> {
    let bytes = argument.as_bytes();
    let mut depth_paren = 0usize;
    let mut depth_brace = 0usize;
    let mut depth_bracket = 0usize;
    let mut index = 0usize;

    while index < bytes.len() {
        let ch = bytes[index] as char;
        match ch {
            '"' => index = scan_string(argument, index)?,
            '\'' => index = scan_char_or_lifetime(argument, index)?,
            '/' if argument[index..].starts_with("//") => index = line_comment_end(argument, index),
            '/' if argument[index..].starts_with("/*") => {
                index = scan_block_comment(argument, index)?
            }
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
            '=' if depth_paren == 0
                && depth_brace == 0
                && depth_bracket == 0
                && argument[index..].starts_with("=>") =>
            {
                return Ok(Some((&argument[..index], &argument[index + 2..])));
            }
            _ => index += 1,
        }
    }

    Ok(None)
}
fn normalize_closure_param(param: &str) -> String {
    let trimmed = param.trim();
    if trimmed.starts_with('(') && trimmed.ends_with(')') && trimmed.len() >= 2 {
        return trimmed[1..trimmed.len() - 1].trim().to_string();
    }
    trimmed.to_string()
}
fn iterator_kind(receiver: &str) -> IteratorKind {
    let trimmed = receiver.trim_end();
    if trimmed.ends_with(".into_iter()") {
        return IteratorKind::IntoIter;
    }
    if trimmed.ends_with(".iter()") || trimmed.ends_with(".iter_mut()") {
        return IteratorKind::Iter;
    }
    IteratorKind::NeedsIter
}
fn is_expression_start(source: &str, index: usize) -> bool {
    let ch = source.as_bytes()[index] as char;
    (is_identifier_start(ch) || ch == '(')
        && source[..index]
            .chars()
            .next_back()
            .is_none_or(is_expression_boundary)
}
fn is_expression_boundary(ch: char) -> bool {
    ch.is_whitespace()
        || matches!(
            ch,
            '=' | '('
                | '['
                | '{'
                | ','
                | ';'
                | ':'
                | '+'
                | '-'
                | '*'
                | '/'
                | '%'
                | '!'
                | '&'
                | '|'
                | '^'
                | '<'
                | '>'
                | '?'
        )
}
fn line_comment_end(source: &str, index: usize) -> usize {
    source[index..]
        .find('\n')
        .map(|offset| index + offset)
        .unwrap_or(source.len())
}
#[derive(Clone, Copy)]
enum HelperKind {
    Map,
    Filter,
    Find,
    FindIndex,
}

impl HelperKind {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "map" => Some(Self::Map),
            "filter" => Some(Self::Filter),
            "find" => Some(Self::Find),
            "findIndex" => Some(Self::FindIndex),
            _ => None,
        }
    }

    fn rust_name(self) -> &'static str {
        match self {
            Self::Map => ".map",
            Self::Filter => ".filter",
            Self::Find => ".find",
            Self::FindIndex => ".position",
        }
    }

    fn deref_count(self, iter_kind: IteratorKind) -> usize {
        let item_refs = match iter_kind {
            IteratorKind::IntoIter => 0,
            IteratorKind::Iter | IteratorKind::NeedsIter => 1,
        };
        item_refs
            + match self {
                Self::Filter | Self::Find => 1,
                Self::Map | Self::FindIndex => 0,
            }
    }
}

struct HelperCall {
    kind: HelperKind,
    argument: String,
    end: usize,
}

#[derive(Clone, Copy)]
enum IteratorKind {
    NeedsIter,
    Iter,
    IntoIter,
}
