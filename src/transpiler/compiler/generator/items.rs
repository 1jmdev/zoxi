use crate::transpiler::compiler::ast::{
    Block, Expr, Function, Item, LetStmt, Module, StaticItem, Stmt, TypeExpr,
};
use crate::transpiler::error::TranspileError;

use super::expr::{generate_expr, ExprContext};
use super::strings::has_interpolation;
use super::types::{generate_type, is_owned_string_type, is_result_type};

pub(super) fn generate_module(module: &Module) -> Result<String, TranspileError> {
    module
        .items
        .iter()
        .map(generate_item)
        .collect::<Result<Vec<_>, _>>()
        .map(|items| items.join("\n"))
}

fn generate_item(item: &Item) -> Result<String, TranspileError> {
    match item {
        Item::Function(function) => generate_function(function),
        Item::Static(item) => generate_static(item),
    }
}

fn generate_function(function: &Function) -> Result<String, TranspileError> {
    let params = function
        .params
        .iter()
        .map(|param| Ok(format!("{}: {}", param.name, generate_type(&param.ty))))
        .collect::<Result<Vec<_>, TranspileError>>()?;
    let signature = match &function.return_type {
        Some(return_type) => format!(
            "fn {}({}) -> {}",
            function.name,
            params.join(", "),
            generate_type(return_type)
        ),
        None => format!("fn {}({})", function.name, params.join(", ")),
    };
    let body = generate_block(&function.body, function.return_type.as_ref())?;
    Ok(format!("{} {}", signature, body))
}

fn generate_static(item: &StaticItem) -> Result<String, TranspileError> {
    if is_owned_string_type(&item.ty)
        && matches!(&item.value, Expr::String(literal) if !has_interpolation(literal))
    {
        return Ok(format!(
            "static {}: &str = {};",
            item.name,
            match &item.value {
                Expr::String(literal) => &literal.raw,
                _ => unreachable!(),
            }
        ));
    }

    Ok(format!(
        "static {}: {} = {};",
        item.name,
        generate_type(&item.ty),
        generate_expr(&item.value, ExprContext::default())?
    ))
}

fn generate_block(block: &Block, return_type: Option<&TypeExpr>) -> Result<String, TranspileError> {
    let mut parts = block
        .statements
        .iter()
        .map(generate_stmt)
        .collect::<Result<Vec<_>, _>>()?;

    if let Some(tail) = &block.tail {
        let tail = if return_type.is_some_and(is_result_type) && should_wrap_result_expr(tail) {
            format!("Ok({})", generate_expr(tail, ExprContext::default())?)
        } else {
            generate_expr(tail, ExprContext::default())?
        };
        parts.push(tail);
    }

    Ok(format!("{{ {} }}", parts.join(" ")))
}

fn generate_stmt(stmt: &Stmt) -> Result<String, TranspileError> {
    match stmt {
        Stmt::Let(LetStmt {
            mutable,
            name,
            ty,
            value,
        }) => Ok(match ty {
            Some(ty) => format!(
                "let {}{}: {} = {};",
                mutable.then_some("mut ").unwrap_or_default(),
                name,
                generate_type(ty),
                generate_let_value(ty, value)?
            ),
            None => format!(
                "let {}{} = {};",
                mutable.then_some("mut ").unwrap_or_default(),
                name,
                generate_expr(value, ExprContext::default())?
            ),
        }),
        Stmt::IndexAssign(assign) => Ok(format!(
            "{}.insert({}, {});",
            generate_expr(&assign.target, ExprContext::default())?,
            generate_expr(&assign.index, ExprContext::preserve_strings())?,
            generate_expr(&assign.value, ExprContext::default())?
        )),
        Stmt::Expr(expr) => Ok(format!("{};", generate_expr(expr, ExprContext::default())?)),
    }
}

fn generate_let_value(ty: &TypeExpr, expr: &Expr) -> Result<String, TranspileError> {
    if !is_owned_string_type(ty) {
        return generate_expr(expr, ExprContext::default());
    }

    let generated = generate_expr(expr, ExprContext::preserve_strings())?;
    if generated.starts_with("String::from(")
        || generated.starts_with("format!(")
        || generated.ends_with(".to_string()")
        || generated == "String::new()"
    {
        Ok(generated)
    } else {
        Ok(format!("String::from({})", generated))
    }
}

fn should_wrap_result_expr(expr: &Expr) -> bool {
    !matches!(expr, Expr::Return(_))
        && !matches!(expr, Expr::Call { callee, .. } if matches!(&**callee, Expr::Path(path) if path.segments.len() == 1 && matches!(path.segments[0].as_str(), "Ok" | "Err")))
}
