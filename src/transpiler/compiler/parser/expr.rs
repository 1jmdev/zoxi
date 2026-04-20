use chumsky::prelude::*;

use crate::transpiler::compiler::ast::{BinaryOp, Expr, MapEntry, StringLiteral, UnaryOp};
use crate::transpiler::compiler::lexer::{Token, TokenKind};

use super::path::path_parser;
use super::tokens::{identifier, keyword, punct, symbol};

pub(super) fn expr_parser<'tokens>(
) -> impl Parser<'tokens, &'tokens [Token], Expr, extra::Err<Rich<'tokens, Token>>> {
    recursive(|expr| {
        let path = path_parser().boxed();
        let args = expr
            .clone()
            .separated_by(symbol(TokenKind::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(symbol(TokenKind::OpenParen), symbol(TokenKind::CloseParen))
            .boxed();

        let array = expr
            .clone()
            .separated_by(symbol(TokenKind::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(symbol(TokenKind::OpenBracket), symbol(TokenKind::CloseBracket))
            .map(Expr::Array);

        let map = expr
            .clone()
            .then_ignore(symbol(TokenKind::Colon))
            .then(expr.clone())
            .map(|(key, value): (Expr, Expr)| MapEntry { key, value })
            .separated_by(symbol(TokenKind::Comma))
            .allow_trailing()
            .collect::<Vec<_>>()
            .delimited_by(symbol(TokenKind::OpenBrace), symbol(TokenKind::CloseBrace))
            .map(Expr::Map);

        let paren = expr
            .clone()
            .delimited_by(symbol(TokenKind::OpenParen), symbol(TokenKind::CloseParen))
            .map(|expr| Expr::Paren(Box::new(expr)));

        let closure_param = identifier().or(
            identifier().delimited_by(symbol(TokenKind::OpenParen), symbol(TokenKind::CloseParen)),
        );
        let closure = closure_param
            .then_ignore(symbol(TokenKind::FatArrow))
            .then(expr.clone())
            .map(|(param, body): (String, Expr)| Expr::Closure {
                param,
                body: Box::new(body),
            });

        let macro_call = path
            .clone()
            .then_ignore(symbol(TokenKind::Bang))
            .then(args.clone())
            .map(|(path, args)| Expr::MacroCall { path, args });

        let string = select_ref! {
            Token { kind: TokenKind::String, text, span } => Expr::String(StringLiteral {
                raw: text.clone(),
                span: span.clone(),
            })
        };
        let number =
            select_ref! { Token { kind: TokenKind::Number, text, .. } => Expr::Number(text.clone()) };
        let character =
            select_ref! { Token { kind: TokenKind::Char, text, .. } => Expr::Char(text.clone()) };
        let path_expr = path.map(Expr::Path);

        let atom = choice((
            closure, array, map, paren, macro_call, string, number, character, path_expr,
        ))
        .boxed();

        let suffix = choice((
            args.clone().map(Suffix::Call),
            expr.clone()
                .delimited_by(symbol(TokenKind::OpenBracket), symbol(TokenKind::CloseBracket))
                .map(Suffix::Index),
            symbol(TokenKind::Dot)
                .ignore_then(identifier())
                .then(args.clone().or_not())
                .map(|(name, args)| Suffix::Member { name, args }),
        ));

        let postfix = atom
            .then(suffix.repeated().collect::<Vec<_>>())
            .map(|(expr, suffixes): (Expr, Vec<Suffix>)| {
                suffixes.into_iter().fold(expr, |expr, suffix| match suffix {
                    Suffix::Call(args) => Expr::Call {
                        callee: Box::new(expr),
                        args,
                    },
                    Suffix::Index(index) => Expr::Index {
                        receiver: Box::new(expr),
                        index: Box::new(index),
                    },
                    Suffix::Member { name, args } => match args {
                        Some(args) => Expr::MethodCall {
                            receiver: Box::new(expr),
                            method: name,
                            args,
                        },
                        None => Expr::Field {
                            receiver: Box::new(expr),
                            name,
                        },
                    },
                })
            })
            .boxed();

        let unary_prefix = choice((
            symbol(TokenKind::Ampersand).to(UnaryOp::Ref),
            symbol(TokenKind::Bang).to(UnaryOp::Not),
            punct("-").to(UnaryOp::Neg),
        ));

        let unary = keyword("return")
            .ignore_then(expr.clone())
            .map(|expr| Expr::Return(Box::new(expr)))
            .or(unary_prefix.repeated().collect::<Vec<_>>().then(postfix.clone()).map(
                |(ops, expr): (Vec<UnaryOp>, Expr)| {
                    ops.into_iter().rev().fold(expr, |expr, op| Expr::Unary {
                        op,
                        expr: Box::new(expr),
                    })
                },
            ))
            .or(postfix)
            .boxed();

        let product = unary
            .clone()
            .foldl(
                choice((
                    punct("*").to(BinaryOp::Mul),
                    punct("/").to(BinaryOp::Div),
                    punct("%").to(BinaryOp::Rem),
                ))
                .then(unary.clone())
                .repeated(),
                fold_binary,
            )
            .boxed();

        let sum = product
            .clone()
            .foldl(
                choice((punct("+").to(BinaryOp::Add), punct("-").to(BinaryOp::Sub)))
                    .then(product.clone())
                    .repeated(),
                fold_binary,
            )
            .boxed();

        let compare = sum
            .clone()
            .foldl(
                choice((
                    punct(">=").to(BinaryOp::Ge),
                    punct("<=").to(BinaryOp::Le),
                    punct("==").to(BinaryOp::Eq),
                    punct("!=").to(BinaryOp::NotEq),
                    punct(">").to(BinaryOp::Gt),
                    punct("<").to(BinaryOp::Lt),
                ))
                .then(sum.clone())
                .repeated(),
                fold_binary,
            )
            .boxed();

        let and = compare
            .clone()
            .foldl(
                punct("&&").to(BinaryOp::And).then(compare.clone()).repeated(),
                fold_binary,
            )
            .boxed();

        and.clone()
            .foldl(punct("||").to(BinaryOp::Or).then(and).repeated(), fold_binary)
            .boxed()
    })
}

fn fold_binary(lhs: Expr, (op, rhs): (BinaryOp, Expr)) -> Expr {
    Expr::Binary {
        lhs: Box::new(lhs),
        op,
        rhs: Box::new(rhs),
    }
}

enum Suffix {
    Call(Vec<Expr>),
    Index(Expr),
    Member { name: String, args: Option<Vec<Expr>> },
}
