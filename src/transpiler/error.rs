use std::ops::Range;

use ariadne::{Color, Label, Report, ReportKind, Source};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TranspileError {
    #[error("{message}")]
    Diagnostic { message: String, span: Range<usize> },
    #[error("missing src directory at {0}")]
    MissingSourceDirectory(String),
    #[error("no .zo source files were found in {0}")]
    NoSources(String),
}

impl TranspileError {
    pub fn diagnostic(message: impl Into<String>, span: Range<usize>) -> Self {
        Self::Diagnostic {
            message: message.into(),
            span,
        }
    }

    pub fn render(&self, file_name: &str, source: &str) -> String {
        match self {
            Self::Diagnostic { message, span } => {
                let mut output = Vec::new();
                let report = Report::build(ReportKind::Error, (file_name, span.clone()))
                    .with_message(message.clone())
                    .with_label(
                        Label::new((file_name, span.clone()))
                            .with_message(message.clone())
                            .with_color(Color::Red),
                    )
                    .finish();
                if report
                    .write((file_name, Source::from(source)), &mut output)
                    .is_ok()
                {
                    String::from_utf8_lossy(&output).into_owned()
                } else {
                    message.clone()
                }
            }
            _ => self.to_string(),
        }
    }
}
