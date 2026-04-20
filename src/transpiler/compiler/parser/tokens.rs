use chumsky::prelude::*;

use crate::transpiler::compiler::lexer::{Token, TokenKind};

pub(super) fn identifier<'tokens>(
) -> impl Parser<'tokens, &'tokens [Token], String, extra::Err<Rich<'tokens, Token>>> {
    select_ref! { Token { kind: TokenKind::Ident, text, .. } => text.clone() }.boxed()
}

pub(super) fn lifetime<'tokens>(
) -> impl Parser<'tokens, &'tokens [Token], String, extra::Err<Rich<'tokens, Token>>> {
    select_ref! { Token { kind: TokenKind::Lifetime, text, .. } => text.clone() }.boxed()
}

pub(super) fn keyword<'tokens>(
    expected: &'static str,
) -> impl Parser<'tokens, &'tokens [Token], String, extra::Err<Rich<'tokens, Token>>> {
    select_ref! { Token { kind: TokenKind::Ident, text, .. } if text == expected => text.clone() }.boxed()
}

pub(super) fn punct<'tokens>(
    expected: &'static str,
) -> impl Parser<'tokens, &'tokens [Token], String, extra::Err<Rich<'tokens, Token>>> {
    select_ref! { Token { text, .. } if text == expected => text.clone() }.boxed()
}

pub(super) fn symbol<'tokens>(
    kind: TokenKind,
) -> impl Parser<'tokens, &'tokens [Token], Token, extra::Err<Rich<'tokens, Token>>> {
    any().filter(move |token: &Token| token.kind == kind).boxed()
}
