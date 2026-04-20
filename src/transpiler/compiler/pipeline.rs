use crate::transpiler::compiler::generator::generate;
use crate::transpiler::compiler::lexer::lex;
use crate::transpiler::compiler::parser::parse;
use crate::transpiler::compiler::semantics::analyze;
use crate::transpiler::error::TranspileError;

pub fn compile_source(source: &str, _file_name: &str) -> Result<String, TranspileError> {
    let tokens = lex(source)?;
    let module = parse(&tokens)?;
    let analysis = analyze(module)?;
    generate(&analysis)
}
