use chumsky::prelude::*;

use crate::transpiler::compiler::ast::{TypeExpr, TypePath, TypeSegment};
use crate::transpiler::compiler::lexer::{Token, TokenKind};

use super::tokens::{identifier, keyword, lifetime, punct, symbol};

pub(super) fn type_parser<'tokens>(
) -> impl Parser<'tokens, &'tokens [Token], TypeExpr, extra::Err<Rich<'tokens, Token>>> {
    recursive(|ty| {
        let generics = ty
            .clone()
            .separated_by(symbol(TokenKind::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(punct("<"), punct(">"))
            .boxed();

        let segment = identifier()
            .then(generics.or_not())
            .map(|(name, generics): (String, Option<Vec<TypeExpr>>)| TypeSegment {
                name,
                generics: generics.unwrap_or_default(),
            })
            .boxed();

        let path = segment
            .clone()
            .then(punct("::").ignore_then(segment).repeated().collect::<Vec<_>>())
            .map(|(first, rest): (TypeSegment, Vec<TypeSegment>)| {
                let mut segments = Vec::with_capacity(rest.len() + 1);
                segments.push(first);
                segments.extend(rest);
                TypeExpr::Path(TypePath { segments })
            })
            .boxed();

        symbol(TokenKind::Ampersand)
            .ignore_then(lifetime().or_not())
            .then(keyword("mut").or_not())
            .then(ty.clone().or(path.clone()))
            .map(|((lifetime, mutable), inner): ((Option<String>, Option<String>), TypeExpr)| {
                TypeExpr::Reference {
                    lifetime,
                    mutable: mutable.is_some(),
                    inner: Box::new(inner),
                }
            })
            .or(path)
            .boxed()
    })
}
