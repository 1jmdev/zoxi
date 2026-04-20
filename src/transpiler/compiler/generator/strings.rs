use crate::transpiler::compiler::ast::StringLiteral;
use crate::transpiler::compiler::lexer::Token;
use crate::transpiler::compiler::parser::parse_expression;
use crate::transpiler::error::TranspileError;

use super::expr::{generate_expr, ExprContext};

pub(super) fn generate_string_literal(
    literal: &StringLiteral,
    context: ExprContext,
) -> Result<String, TranspileError> {
    if context.preserve_strings {
        return Ok(literal.raw.clone());
    }

    let content = &literal.raw[1..literal.raw.len() - 1];
    let interpolations = extract_interpolations(content, literal.span.start + 1)?;
    if interpolations.is_empty() {
        return Ok(format!("String::from({})", literal.raw));
    }

    let mut format_string = String::new();
    let mut arguments = Vec::with_capacity(interpolations.len());
    let mut cursor = 0usize;
    for interpolation in interpolations {
        format_string.push_str(&escape_format_segment(&content[cursor..interpolation.start]));
        format_string.push_str("{}");
        let expression = &content[interpolation.start + 1..interpolation.end - 1];
        arguments.push(compile_embedded_expression(
            expression,
            literal.span.start + 1 + interpolation.start + 1,
        )?);
        cursor = interpolation.end;
    }
    format_string.push_str(&escape_format_segment(&content[cursor..]));
    Ok(format!("format!(\"{}\", {})", format_string, arguments.join(", ")))
}

pub(super) fn has_interpolation(literal: &StringLiteral) -> bool {
    extract_interpolations(&literal.raw[1..literal.raw.len() - 1], literal.span.start + 1)
        .map(|items| !items.is_empty())
        .unwrap_or(false)
}

fn compile_embedded_expression(source: &str, offset: usize) -> Result<String, TranspileError> {
    let tokens = crate::transpiler::compiler::lexer::lex(source)?;
    let adjusted = tokens
        .into_iter()
        .map(|mut token| {
            token.span = (token.span.start + offset)..(token.span.end + offset);
            token
        })
        .collect::<Vec<Token>>();
    let expression = parse_expression(&adjusted)?;
    generate_expr(&expression, ExprContext::default())
}

fn extract_interpolations(source: &str, base_offset: usize) -> Result<Vec<Interpolation>, TranspileError> {
    let bytes = source.as_bytes();
    let mut items = Vec::new();
    let mut index = 0usize;

    while index < bytes.len() {
        match bytes[index] as char {
            '\\' => index += 2,
            '{' => {
                let start = index;
                let mut depth = 1usize;
                index += 1;
                while index < bytes.len() {
                    match bytes[index] as char {
                        '\\' => index += 2,
                        '{' => {
                            depth += 1;
                            index += 1;
                        }
                        '}' => {
                            depth -= 1;
                            index += 1;
                            if depth == 0 {
                                items.push(Interpolation { start, end: index });
                                break;
                            }
                        }
                        _ => index += 1,
                    }
                }
                if depth != 0 {
                    return Err(TranspileError::diagnostic(
                        "unclosed string interpolation",
                        (base_offset + start)..(base_offset + source.len()),
                    ));
                }
            }
            _ => index += 1,
        }
    }

    Ok(items)
}

fn escape_format_segment(segment: &str) -> String {
    segment.replace('{', "{{").replace('}', "}}")
}

#[derive(Clone)]
struct Interpolation {
    start: usize,
    end: usize,
}
