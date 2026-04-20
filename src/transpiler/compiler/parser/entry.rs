use chumsky::Parser;

use crate::transpiler::compiler::ast::{Expr, Module};
use crate::transpiler::compiler::lexer::Token;
use crate::transpiler::error::TranspileError;

use super::error::parse_error;
use super::expr::expr_parser;
use super::items::module_parser;

pub fn parse(tokens: &[Token]) -> Result<Module, TranspileError> {
    module_parser()
        .parse(tokens)
        .into_result()
        .map_err(|errors| parse_error(errors.into_iter().next(), tokens))
}

pub fn parse_expression(tokens: &[Token]) -> Result<Expr, TranspileError> {
    expr_parser()
        .then_ignore(chumsky::prelude::end())
        .parse(tokens)
        .into_result()
        .map_err(|errors| parse_error(errors.into_iter().next(), tokens))
}
