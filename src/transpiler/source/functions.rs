use crate::transpiler::error::TranspileError;
use crate::transpiler::source::keywords::starts_with_keyword;
use crate::transpiler::source::scan::{
    find_matching, scan_block_comment, scan_char_or_lifetime, scan_string, skip_whitespace,
    trim_end_index,
};

pub(crate) fn rewrite_signature_returns(source: &str) -> Result<String, TranspileError> {
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
            ')' | '|' => {
                result.push(ch);
                index += 1;
                let whitespace_end = skip_whitespace(source, index);
                if source[whitespace_end..].starts_with(':') {
                    result.push_str(&source[index..whitespace_end]);
                    result.push_str(" ->");
                    index = whitespace_end + 1;
                }
            }
            _ => {
                result.push(ch);
                index += 1;
            }
        }
    }

    Ok(result)
}

pub(crate) fn rewrite_result_functions(source: &str) -> Result<String, TranspileError> {
    let mut result = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        if starts_with_keyword(source, index, "fn") {
            let body_start = source[index..]
                .find('{')
                .map(|offset| index + offset)
                .unwrap_or(source.len());
            if body_start < source.len() {
                let signature = &source[index..body_start];
                let body_end = find_matching(source, body_start, '{', '}')?;
                let body = &source[body_start + 1..body_end - 1];

                result.push_str(signature);
                result.push('{');
                if signature.contains("-> Result<") {
                    result.push_str(&wrap_result_tail_expression(body)?);
                } else {
                    result.push_str(body);
                }
                result.push('}');
                index = body_end;
                continue;
            }
        }

        result.push(bytes[index] as char);
        index += 1;
    }

    Ok(result)
}

fn wrap_result_tail_expression(body: &str) -> Result<String, TranspileError> {
    let trimmed_end = trim_end_index(body);
    let body = &body[..trimmed_end];
    let last_statement_start = find_last_statement_start(body)?;
    let tail = body[last_statement_start..].trim();

    if tail.is_empty()
        || tail.ends_with(';')
        || tail.starts_with("return")
        || tail.starts_with("Ok(")
        || tail.starts_with("Err(")
    {
        return Ok(body.to_string());
    }

    let mut rewritten = String::from(&body[..last_statement_start]);
    rewritten.push_str("Ok(");
    rewritten.push_str(tail);
    rewritten.push(')');
    Ok(rewritten)
}

fn find_last_statement_start(body: &str) -> Result<usize, TranspileError> {
    let mut depth_paren = 0usize;
    let mut depth_brace = 0usize;
    let mut depth_bracket = 0usize;
    let mut last = 0usize;
    let bytes = body.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        let ch = bytes[index] as char;
        match ch {
            '"' => index = scan_string(body, index)?,
            '\'' => index = scan_char_or_lifetime(body, index)?,
            '/' if body[index..].starts_with("//") => {
                index = body[index..]
                    .find('\n')
                    .map(|offset| index + offset)
                    .unwrap_or(body.len());
            }
            '/' if body[index..].starts_with("/*") => index = scan_block_comment(body, index)?,
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
            ';' if depth_paren == 0 && depth_brace == 0 && depth_bracket == 0 => {
                last = index + 1;
                index += 1;
            }
            _ => index += 1,
        }
    }

    Ok(last)
}
