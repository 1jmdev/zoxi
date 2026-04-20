use crate::transpiler::compiler::ast::Expr;
use crate::transpiler::error::TranspileError;

use super::expr::{generate_expr, ExprContext};

pub(super) fn generate_iterator_chain(expr: &Expr) -> Result<Option<String>, TranspileError> {
    let mut calls = Vec::new();
    let mut current = expr;
    while let Expr::MethodCall {
        receiver,
        method,
        args,
    } = current
    {
        let Some(kind) = HelperKind::from_name(method) else {
            break;
        };
        calls.push((kind, args.as_slice()));
        current = receiver;
    }

    if calls.is_empty() {
        return Ok(None);
    }

    calls.reverse();
    let mut base = generate_expr(current, ExprContext::default())?;
    let iter_kind = iterator_kind(&base);
    if matches!(iter_kind, IteratorKind::NeedsIter) {
        base.push_str(".iter()");
    }

    let mut result = base;
    let mut last_kind = HelperKind::Find;
    for (kind, args) in calls {
        let arg = generate_helper_argument(args, kind, iter_kind)?;
        result.push_str(kind.rust_name());
        result.push('(');
        result.push_str(&arg);
        result.push(')');
        last_kind = kind;
    }
    if matches!(last_kind, HelperKind::Map | HelperKind::Filter) {
        result.push_str(".collect()");
    }
    Ok(Some(result))
}

fn generate_helper_argument(
    args: &[Expr],
    kind: HelperKind,
    iter_kind: IteratorKind,
) -> Result<String, TranspileError> {
    let Some(argument) = args.first() else {
        return Ok(String::new());
    };
    match argument {
        Expr::Closure { param, body } => {
            let body = generate_expr(body, ExprContext::default())?;
            let body = rewrite_closure_body(&body, param, kind.deref_count(iter_kind));
            Ok(format!("|{}| {}", param, body))
        }
        _ => generate_expr(argument, ExprContext::default()),
    }
}

fn rewrite_closure_body(body: &str, param: &str, deref_count: usize) -> String {
    if deref_count == 0 || !is_identifier_like(param) {
        return body.to_string();
    }
    body.replace(param, &format!("({}{})", "*".repeat(deref_count), param))
}

fn iterator_kind(receiver: &str) -> IteratorKind {
    if receiver.ends_with(".into_iter()") {
        IteratorKind::IntoIter
    } else if receiver.ends_with(".iter()") || receiver.ends_with(".iter_mut()") {
        IteratorKind::Iter
    } else {
        IteratorKind::NeedsIter
    }
}

fn is_identifier_like(value: &str) -> bool {
    value.chars().next().is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
        && value.chars().all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

#[derive(Clone, Copy)]
enum HelperKind {
    Map,
    Filter,
    Find,
    FindIndex,
}

impl HelperKind {
    fn from_name(name: &str) -> Option<Self> {
        match name {
            "map" => Some(Self::Map),
            "filter" => Some(Self::Filter),
            "find" => Some(Self::Find),
            "findIndex" => Some(Self::FindIndex),
            _ => None,
        }
    }

    fn rust_name(self) -> &'static str {
        match self {
            Self::Map => ".map",
            Self::Filter => ".filter",
            Self::Find => ".find",
            Self::FindIndex => ".position",
        }
    }

    fn deref_count(self, iter_kind: IteratorKind) -> usize {
        let item_refs = match iter_kind {
            IteratorKind::IntoIter => 0,
            IteratorKind::Iter | IteratorKind::NeedsIter => 1,
        };
        item_refs
            + match self {
                Self::Filter | Self::Find => 1,
                Self::Map | Self::FindIndex => 0,
            }
    }
}

#[derive(Clone, Copy)]
enum IteratorKind {
    NeedsIter,
    Iter,
    IntoIter,
}
