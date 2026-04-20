use chumsky::prelude::*;

use crate::transpiler::compiler::ast::PathExpr;
use crate::transpiler::compiler::lexer::Token;

use super::tokens::{identifier, punct};

pub(super) fn path_parser<'tokens>(
) -> impl Parser<'tokens, &'tokens [Token], PathExpr, extra::Err<Rich<'tokens, Token>>> {
    identifier()
        .then(punct("::").ignore_then(identifier()).repeated().collect::<Vec<_>>())
        .map(|(first, rest): (String, Vec<String>)| {
            let mut segments = Vec::with_capacity(rest.len() + 1);
            segments.push(first);
            segments.extend(rest);
            PathExpr { segments }
        })
}
