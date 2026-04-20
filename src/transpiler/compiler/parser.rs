use crate::transpiler::compiler::ast::{Delimiter, Group, Module, Span, SyntaxNode};
use crate::transpiler::compiler::lexer::{Token, TokenKind};
use crate::transpiler::error::TranspileError;

pub fn parse(tokens: &[Token]) -> Result<Module, TranspileError> {
    let mut index = 0usize;
    Ok(Module {
        nodes: parse_sequence(tokens, &mut index, None)?,
    })
}

fn parse_sequence(
    tokens: &[Token],
    index: &mut usize,
    until: Option<Delimiter>,
) -> Result<Vec<SyntaxNode>, TranspileError> {
    let mut nodes = Vec::new();

    while *index < tokens.len() {
        let token = tokens[*index].clone();
        if let Some(delimiter) = until
            && matches_closing(&token.kind, delimiter)
        {
            *index += 1;
            return Ok(nodes);
        }

        match opening_delimiter(&token.kind) {
            Some(delimiter) => {
                *index += 1;
                let children = parse_sequence(tokens, index, Some(delimiter))?;
                nodes.push(SyntaxNode::Group(Group {
                    delimiter,
                    nodes: children,
                }));
            }
            None if is_closing(&token.kind) => {
                return Err(TranspileError::diagnostic(
                    format!("unexpected closing delimiter `{}`", token.text),
                    token.span,
                ));
            }
            None => {
                nodes.push(SyntaxNode::Token(token));
                *index += 1;
            }
        }
    }

    if let Some(delimiter) = until {
        return Err(TranspileError::diagnostic(
            format!("unclosed delimiter `{}`", delimiter.open()),
            missing_span(tokens),
        ));
    }

    Ok(nodes)
}

fn opening_delimiter(kind: &TokenKind) -> Option<Delimiter> {
    match kind {
        TokenKind::OpenParen => Some(Delimiter::Paren),
        TokenKind::OpenBracket => Some(Delimiter::Bracket),
        TokenKind::OpenBrace => Some(Delimiter::Brace),
        _ => None,
    }
}

fn matches_closing(kind: &TokenKind, delimiter: Delimiter) -> bool {
    matches!(
        (kind, delimiter),
        (TokenKind::CloseParen, Delimiter::Paren)
            | (TokenKind::CloseBracket, Delimiter::Bracket)
            | (TokenKind::CloseBrace, Delimiter::Brace)
    )
}

fn is_closing(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::CloseParen | TokenKind::CloseBracket | TokenKind::CloseBrace
    )
}

fn missing_span(tokens: &[Token]) -> Span {
    tokens
        .last()
        .map(|token| token.span.end..token.span.end)
        .unwrap_or(0..0)
}
