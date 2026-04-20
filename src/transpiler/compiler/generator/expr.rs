use crate::transpiler::compiler::ast::{Expr, MapEntry, UnaryOp};
use crate::transpiler::error::TranspileError;

use super::iterators::generate_iterator_chain;
use super::strings::generate_string_literal;
use super::types::{generate_binary_op, generate_path};

pub(super) fn generate_expr(expr: &Expr, context: ExprContext) -> Result<String, TranspileError> {
    if let Some(iterator_expr) = generate_iterator_chain(expr)? {
        return Ok(iterator_expr);
    }

    match expr {
        Expr::Path(path) => Ok(generate_path(path)),
        Expr::String(literal) => generate_string_literal(literal, context),
        Expr::Number(number) | Expr::Char(number) => Ok(number.clone()),
        Expr::Array(items) => items
            .iter()
            .map(|item| generate_expr(item, ExprContext::default()))
            .collect::<Result<Vec<_>, _>>()
            .map(|items| format!("vec![{}]", items.join(", "))),
        Expr::Map(entries) => generate_map(entries),
        Expr::Call { callee, args } => Ok(format!(
            "{}({})",
            generate_expr(callee, ExprContext::default())?,
            args.iter()
                .map(|arg| generate_expr(arg, ExprContext::preserve_strings()))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        )),
        Expr::MacroCall { path, args } => Ok(format!(
            "{}!({})",
            generate_path(path),
            args.iter()
                .map(|arg| generate_expr(arg, ExprContext::preserve_strings()))
                .collect::<Result<Vec<_>, _>>()?
                .join(", ")
        )),
        Expr::MethodCall {
            receiver,
            method,
            args,
        } => {
            if method == "view" && args.is_empty() {
                return Ok(format!(
                    "{}.as_str()",
                    generate_expr(receiver, ExprContext::default())?
                ));
            }
            Ok(format!(
                "{}.{}({})",
                generate_expr(receiver, ExprContext::default())?,
                method,
                args.iter()
                    .map(|arg| generate_expr(arg, ExprContext::preserve_strings()))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            ))
        }
        Expr::Field { receiver, name } => Ok(format!(
            "{}.{}",
            generate_expr(receiver, ExprContext::default())?,
            name
        )),
        Expr::Index { receiver, index } => Ok(format!(
            "{}[{}]",
            generate_expr(receiver, ExprContext::default())?,
            generate_expr(index, ExprContext::preserve_strings())?
        )),
        Expr::Unary { op, expr } => Ok(format!(
            "{}{}",
            match op {
                UnaryOp::Ref => "&",
                UnaryOp::Not => "!",
                UnaryOp::Neg => "-",
            },
            generate_expr(
                expr,
                if matches!(op, UnaryOp::Ref) {
                    ExprContext::preserve_strings()
                } else {
                    ExprContext::default()
                }
            )?
        )),
        Expr::Binary { lhs, op, rhs } => Ok(format!(
            "{} {} {}",
            generate_expr(lhs, ExprContext::default())?,
            generate_binary_op(op),
            generate_expr(rhs, ExprContext::default())?
        )),
        Expr::Closure { param, body } => Ok(format!(
            "|{}| {}",
            param,
            generate_expr(body, ExprContext::default())?
        )),
        Expr::Paren(expr) => Ok(format!("({})", generate_expr(expr, ExprContext::default())?)),
        Expr::Return(expr) => Ok(format!(
            "return {}",
            generate_expr(expr, ExprContext::default())?
        )),
    }
}

fn generate_map(entries: &[MapEntry]) -> Result<String, TranspileError> {
    if entries.is_empty() {
        return Ok("std::collections::HashMap::new()".to_string());
    }
    entries
        .iter()
        .map(|entry| {
            Ok(format!(
                "({}, {})",
                generate_expr(&entry.key, ExprContext::default())?,
                generate_expr(&entry.value, ExprContext::default())?
            ))
        })
        .collect::<Result<Vec<_>, TranspileError>>()
        .map(|entries| format!("std::collections::HashMap::from([{}])", entries.join(", ")))
}

#[derive(Clone, Copy, Default)]
pub(super) struct ExprContext {
    pub(super) preserve_strings: bool,
}

impl ExprContext {
    pub(super) fn preserve_strings() -> Self {
        Self {
            preserve_strings: true,
        }
    }
}
