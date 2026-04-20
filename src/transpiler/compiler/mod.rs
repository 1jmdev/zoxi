pub mod ast;
pub mod generator;
pub mod lexer;
pub mod parser;
pub mod pipeline;
pub mod semantics;

pub use pipeline::compile_source;
