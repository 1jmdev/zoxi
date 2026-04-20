use crate::transpiler::compiler::ast::{Analysis, Module, SyntaxNode};
use crate::transpiler::error::TranspileError;

pub fn analyze(module: Module) -> Result<Analysis, TranspileError> {
    validate_nodes(&module.nodes)?;
    Ok(Analysis { module })
}

fn validate_nodes(nodes: &[SyntaxNode]) -> Result<(), TranspileError> {
    for node in nodes {
        if let SyntaxNode::Group(group) = node {
            validate_nodes(&group.nodes)?;
        }
    }
    Ok(())
}
