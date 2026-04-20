use crate::transpiler::compiler::ast::{
    Analysis, BinaryOp, Block, Expr, Function, Item, LetStmt, Module, Param, PathExpr, Stmt,
    TypeExpr, TypePath, TypeSegment, UnaryOp,
};
use crate::transpiler::error::TranspileError;

pub fn analyze(module: Module) -> Result<Analysis, TranspileError> {
    for item in &module.items {
        validate_item(item)?;
    }
    Ok(Analysis { module })
}

fn validate_item(item: &Item) -> Result<(), TranspileError> {
    match item {
        Item::Function(function) => validate_function(function),
        Item::Static(item) => {
            validate_type(&item.ty)?;
            validate_expr(&item.value)
        }
    }
}

fn validate_function(function: &Function) -> Result<(), TranspileError> {
    for Param { ty, .. } in &function.params {
        validate_type(ty)?;
    }
    if let Some(return_type) = &function.return_type {
        validate_type(return_type)?;
    }
    validate_block(&function.body)
}

fn validate_block(block: &Block) -> Result<(), TranspileError> {
    for statement in &block.statements {
        match statement {
            Stmt::Let(LetStmt { ty, value, .. }) => {
                if let Some(ty) = ty {
                    validate_type(ty)?;
                }
                validate_expr(value)?;
            }
            Stmt::IndexAssign(assign) => {
                validate_expr(&assign.target)?;
                validate_expr(&assign.index)?;
                validate_expr(&assign.value)?;
            }
            Stmt::Expr(expr) => validate_expr(expr)?,
        }
    }
    if let Some(tail) = &block.tail {
        validate_expr(tail)?;
    }
    Ok(())
}

fn validate_expr(expr: &Expr) -> Result<(), TranspileError> {
    match expr {
        Expr::Path(PathExpr { .. })
        | Expr::String(_)
        | Expr::Number(_)
        | Expr::Char(_) => Ok(()),
        Expr::Array(items) => items.iter().try_for_each(validate_expr),
        Expr::Map(entries) => entries.iter().try_for_each(|entry| {
            validate_expr(&entry.key)?;
            validate_expr(&entry.value)
        }),
        Expr::Call { callee, args } => {
            validate_expr(callee)?;
            args.iter().try_for_each(validate_expr)
        }
        Expr::MacroCall { args, .. } => args.iter().try_for_each(validate_expr),
        Expr::MethodCall { receiver, args, .. } => {
            validate_expr(receiver)?;
            args.iter().try_for_each(validate_expr)
        }
        Expr::Field { receiver, .. } => validate_expr(receiver),
        Expr::Index { receiver, index } => {
            validate_expr(receiver)?;
            validate_expr(index)
        }
        Expr::Unary { op, expr } => {
            let _ = match op {
                UnaryOp::Ref | UnaryOp::Not | UnaryOp::Neg => op,
            };
            validate_expr(expr)
        }
        Expr::Binary { lhs, op, rhs } => {
            let _ = match op {
                BinaryOp::Mul
                | BinaryOp::Div
                | BinaryOp::Rem
                | BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Gt
                | BinaryOp::Lt
                | BinaryOp::Ge
                | BinaryOp::Le
                | BinaryOp::Eq
                | BinaryOp::NotEq
                | BinaryOp::And
                | BinaryOp::Or => op,
            };
            validate_expr(lhs)?;
            validate_expr(rhs)
        }
        Expr::Closure { body, .. } => validate_expr(body),
        Expr::Paren(expr) | Expr::Return(expr) => validate_expr(expr),
    }
}

fn validate_type(ty: &TypeExpr) -> Result<(), TranspileError> {
    match ty {
        TypeExpr::Path(TypePath { segments }) => segments.iter().try_for_each(validate_type_segment),
        TypeExpr::Reference { inner, .. } => validate_type(inner),
    }
}

fn validate_type_segment(segment: &TypeSegment) -> Result<(), TranspileError> {
    segment.generics.iter().try_for_each(validate_type)
}
