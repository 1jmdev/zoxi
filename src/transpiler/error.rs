use thiserror::Error;

#[derive(Debug, Error)]
pub enum TranspileError {
    #[error("unclosed delimiter starting with `{0}`")]
    UnclosedDelimiter(char),
    #[error("missing src directory at {0}")]
    MissingSourceDirectory(String),
    #[error("no .zo source files were found in {0}")]
    NoSources(String),
}
