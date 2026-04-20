use crate::transpiler::compiler::ast::{BinaryOp, PathExpr, TypeExpr, TypePath, TypeSegment};

pub(super) fn generate_type(ty: &TypeExpr) -> String {
    match ty {
        TypeExpr::Path(path) => generate_type_path(path),
        TypeExpr::Reference {
            lifetime,
            mutable,
            inner,
        } => {
            let mut result = String::from("&");
            if let Some(lifetime) = lifetime {
                result.push_str(lifetime);
                result.push(' ');
            }
            if *mutable {
                result.push_str("mut ");
            }
            result.push_str(&generate_type(inner));
            result
        }
    }
}

pub(super) fn generate_type_path(path: &TypePath) -> String {
    path.segments
        .iter()
        .map(generate_type_segment)
        .collect::<Vec<_>>()
        .join("::")
}

fn generate_type_segment(segment: &TypeSegment) -> String {
    let name = if segment.name == "string" {
        "String".to_string()
    } else {
        segment.name.clone()
    };
    if segment.generics.is_empty() {
        name
    } else {
        format!(
            "{}<{}>",
            name,
            segment
                .generics
                .iter()
                .map(generate_type)
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

pub(super) fn generate_path(path: &PathExpr) -> String {
    path.segments.join("::")
}

pub(super) fn generate_binary_op(op: &BinaryOp) -> &'static str {
    match op {
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Rem => "%",
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Gt => ">",
        BinaryOp::Lt => "<",
        BinaryOp::Ge => ">=",
        BinaryOp::Le => "<=",
        BinaryOp::Eq => "==",
        BinaryOp::NotEq => "!=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
    }
}

pub(super) fn is_owned_string_type(ty: &TypeExpr) -> bool {
    matches!(ty, TypeExpr::Path(path) if path.segments.len() == 1 && matches!(path.segments[0].name.as_str(), "string" | "String"))
}

pub(super) fn is_result_type(ty: &TypeExpr) -> bool {
    matches!(ty, TypeExpr::Path(path) if path.segments.len() == 1 && path.segments[0].name == "Result")
}
