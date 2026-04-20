use chumsky::prelude::Rich;

use crate::transpiler::compiler::lexer::Token;
use crate::transpiler::error::TranspileError;

pub(super) fn parse_error(error: Option<Rich<'_, Token>>, tokens: &[Token]) -> TranspileError {
    match error {
        Some(error) => {
            let span = error.span();
            let start = tokens
                .get(span.start)
                .map(|token| token.span.start)
                .or_else(|| tokens.last().map(|token| token.span.end))
                .unwrap_or(0);
            let end = tokens
                .get(span.end.saturating_sub(1))
                .map(|token| token.span.end)
                .unwrap_or(start);
            TranspileError::diagnostic(error.to_string(), start..end)
        }
        None => TranspileError::diagnostic("failed to parse source", 0..0),
    }
}
