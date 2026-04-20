use chumsky::prelude::*;

use crate::transpiler::compiler::ast::{
    Block, Expr, Function, IndexAssignStmt, Item, LetStmt, Module, Param, StaticItem, Stmt,
    TypeExpr,
};
use crate::transpiler::compiler::lexer::{Token, TokenKind};

use super::expr::expr_parser;
use super::tokens::{identifier, keyword, symbol};
use super::types::type_parser;

pub(super) fn module_parser<'tokens>(
) -> impl Parser<'tokens, &'tokens [Token], Module, extra::Err<Rich<'tokens, Token>>> {
    item_parser()
        .repeated()
        .collect::<Vec<_>>()
        .then_ignore(end())
        .map(|items| Module { items })
}

fn item_parser<'tokens>() -> impl Parser<'tokens, &'tokens [Token], Item, extra::Err<Rich<'tokens, Token>>> {
    let function = keyword("fn")
        .ignore_then(identifier())
        .then(
            param_parser()
                .separated_by(symbol(TokenKind::Comma))
                .collect::<Vec<_>>()
                .delimited_by(symbol(TokenKind::OpenParen), symbol(TokenKind::CloseParen)),
        )
        .then(return_type_parser().or_not())
        .then(block_parser())
        .map(
            |(((name, params), return_type), body): (((String, Vec<Param>), Option<TypeExpr>), Block)| {
                Item::Function(Function {
                    name,
                    params,
                    return_type,
                    body,
                })
            },
        );

    let static_item = keyword("static")
        .ignore_then(identifier())
        .then_ignore(symbol(TokenKind::Colon))
        .then(type_parser())
        .then_ignore(symbol(TokenKind::Eq))
        .then(expr_parser())
        .then_ignore(symbol(TokenKind::Semi))
        .map(|((name, ty), value): ((String, TypeExpr), Expr)| {
            Item::Static(StaticItem { name, ty, value })
        });

    function.or(static_item).boxed()
}

fn param_parser<'tokens>() -> impl Parser<'tokens, &'tokens [Token], Param, extra::Err<Rich<'tokens, Token>>> {
    identifier()
        .then_ignore(symbol(TokenKind::Colon))
        .then(type_parser())
        .map(|(name, ty): (String, TypeExpr)| Param { name, ty })
}

fn return_type_parser<'tokens>() -> impl Parser<'tokens, &'tokens [Token], TypeExpr, extra::Err<Rich<'tokens, Token>>> {
    symbol(TokenKind::Colon)
        .ignored()
        .or(symbol(TokenKind::ThinArrow).ignored())
        .ignore_then(type_parser())
}

fn block_parser<'tokens>() -> impl Parser<'tokens, &'tokens [Token], Block, extra::Err<Rich<'tokens, Token>>> {
    let expr = expr_parser().boxed();

    let let_stmt = keyword("let")
        .ignore_then(keyword("mut").or_not())
        .then(identifier())
        .then(symbol(TokenKind::Colon).ignored().ignore_then(type_parser()).or_not())
        .then_ignore(symbol(TokenKind::Eq))
        .then(expr.clone())
        .then_ignore(symbol(TokenKind::Semi))
        .map(
            |(((mutable, name), ty), value): (((Option<String>, String), Option<TypeExpr>), Expr)| {
                Stmt::Let(LetStmt {
                    mutable: mutable.is_some(),
                    name,
                    ty,
                    value,
                })
            },
        );

    let assign_stmt = expr
        .clone()
        .then_ignore(symbol(TokenKind::Eq))
        .then(expr.clone())
        .then_ignore(symbol(TokenKind::Semi))
        .try_map(|(lhs, value), span| match lhs {
            Expr::Index { receiver, index } => Ok(Stmt::IndexAssign(IndexAssignStmt {
                target: *receiver,
                index: *index,
                value,
            })),
            _ => Err(Rich::custom(span, "expected index assignment target")),
        });

    let expr_stmt = expr.clone().then_ignore(symbol(TokenKind::Semi)).map(Stmt::Expr);

    let_stmt
        .or(assign_stmt)
        .or(expr_stmt)
        .repeated()
        .collect::<Vec<_>>()
        .then(expr.or_not())
        .delimited_by(symbol(TokenKind::OpenBrace), symbol(TokenKind::CloseBrace))
        .map(|(statements, tail): (Vec<Stmt>, Option<Expr>)| Block { statements, tail })
}
