use crate::transpiler::error::TranspileError;
use crate::transpiler::source::collections::{
    rewrite_hash_map_index_assignments, rewrite_static_strings, rewrite_string_bindings,
};
use crate::transpiler::source::functions::{rewrite_result_functions, rewrite_signature_returns};
use crate::transpiler::source::iterators::rewrite_iterator_helpers;
use crate::transpiler::source::keywords::rewrite_word;
use crate::transpiler::source::literals::rewrite_literals;

pub fn transpile_source(source: &str, is_main_module: bool) -> Result<String, TranspileError> {
    let mut transpiled = source.replace("\r\n", "\n");
    transpiled = rewrite_signature_returns(&transpiled)?;
    transpiled = rewrite_word(&transpiled, "string", "String");
    transpiled = transpiled.replace(".view()", ".as_str()");
    transpiled = rewrite_literals(&transpiled)?;
    transpiled = rewrite_iterator_helpers(&transpiled)?;
    transpiled = rewrite_hash_map_index_assignments(&transpiled)?;
    transpiled = rewrite_string_bindings(&transpiled)?;
    transpiled = rewrite_static_strings(&transpiled)?;
    transpiled = rewrite_result_functions(&transpiled)?;

    let _ = is_main_module;
    Ok(transpiled)
}
