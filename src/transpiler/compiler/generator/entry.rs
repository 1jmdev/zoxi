use crate::transpiler::compiler::ast::Analysis;
use crate::transpiler::error::TranspileError;

use super::items::generate_module;

pub fn generate(analysis: &Analysis) -> Result<String, TranspileError> {
    generate_module(&analysis.module)
}
