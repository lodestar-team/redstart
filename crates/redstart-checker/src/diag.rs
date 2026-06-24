//! Checker diagnostics.
//!
//! Errors are the product (the design report's "errors teach" principle). Each
//! diagnostic carries its own named source so multi-file projects render the
//! right file, points precisely at the span, and suggests the fix.

use miette::{Diagnostic, GraphicalReportHandler, LabeledSpan, NamedSource, SourceCode};
use redstart_parser::Span;
use std::fmt;
use thiserror::Error;

/// A single semantic-analysis diagnostic.
#[derive(Debug, Error)]
#[error("{message}")]
pub struct Diag {
    /// The headline message.
    pub message: String,
    label: String,
    help: Option<String>,
    code: String,
    src: NamedSource<String>,
    offset: usize,
    len: usize,
}

impl Diag {
    /// Create a diagnostic pointing at `span` within file `file`.
    pub fn new(
        file: &str,
        span: &Span,
        code: &str,
        message: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        Self {
            message: message.into(),
            label: label.into(),
            help: None,
            code: format!("redstart::check::{code}"),
            src: NamedSource::new(file, span.source.to_string()),
            offset: span.start,
            len: span.len(),
        }
    }

    /// Attach a help line.
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Render this diagnostic to a string using the graphical handler.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        let _ = GraphicalReportHandler::new().render_report(&mut out, self);
        out
    }
}

impl Diagnostic for Diag {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        Some(Box::new(self.code.clone()))
    }

    fn source_code(&self) -> Option<&dyn SourceCode> {
        Some(&self.src)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        Some(Box::new(std::iter::once(LabeledSpan::new_with_span(
            Some(self.label.clone()),
            (self.offset, self.len),
        ))))
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        self.help
            .as_ref()
            .map(|h| Box::new(h.clone()) as Box<dyn fmt::Display>)
    }
}
