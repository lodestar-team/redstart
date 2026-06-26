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
    /// The file this diagnostic belongs to.
    pub file: String,
    /// Byte offset of the labelled span.
    pub offset: usize,
    /// Byte length of the labelled span.
    pub len: usize,
    /// 1-indexed line of the labelled span.
    pub line: usize,
    /// 1-indexed column of the labelled span.
    pub col: usize,
    severity: miette::Severity,
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
        let (line, col) = span.line_col();
        Self {
            message: message.into(),
            label: label.into(),
            help: None,
            code: format!("redstart::check::{code}"),
            src: NamedSource::new(file, span.source.to_string()),
            file: file.to_string(),
            offset: span.start,
            len: span.len(),
            line,
            col,
            severity: miette::Severity::Error,
        }
    }

    /// Attach a help line.
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Mark this diagnostic as a warning (a lint) rather than a hard error.
    /// Warnings are reported but do not fail the build.
    #[must_use]
    pub fn warning(mut self) -> Self {
        self.severity = miette::Severity::Warning;
        self
    }

    /// Whether this diagnostic is an error (vs a warning).
    #[must_use]
    pub fn is_error(&self) -> bool {
        matches!(self.severity, miette::Severity::Error)
    }

    /// The severity as a lowercase string (`"error"` / `"warning"`).
    #[must_use]
    pub fn severity_str(&self) -> &'static str {
        match self.severity {
            miette::Severity::Error => "error",
            miette::Severity::Warning => "warning",
            miette::Severity::Advice => "advice",
        }
    }

    /// The diagnostic code (e.g. `redstart::check::E051`).
    #[must_use]
    pub fn code_str(&self) -> &str {
        &self.code
    }

    /// The bare diagnostic code (e.g. `E051`), without the `redstart::check::`
    /// prefix — the form used in `--json` output and `redstart explain`.
    #[must_use]
    pub fn code_short(&self) -> &str {
        self.code.rsplit("::").next().unwrap_or(&self.code)
    }

    /// The span's label text.
    #[must_use]
    pub fn label_str(&self) -> &str {
        &self.label
    }

    /// The help line, if any.
    #[must_use]
    pub fn help_str(&self) -> Option<&str> {
        self.help.as_deref()
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

    fn severity(&self) -> Option<miette::Severity> {
        Some(self.severity)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn diag_exposes_short_code_and_line_col() {
        let source: Arc<str> = Arc::from("entity A {\n  id: Id<Bytes>\n}");
        let span = Span::new(13, 15, Arc::clone(&source)); // on line 2
        let d = Diag::new("a.red", &span, "E062", "boom", "here").with_help("do x");
        assert_eq!(d.code_str(), "redstart::check::E062");
        assert_eq!(d.code_short(), "E062");
        assert_eq!(d.label_str(), "here");
        assert_eq!(d.help_str(), Some("do x"));
        assert_eq!(d.line, 2);
        assert_eq!(d.col, 3);
    }
}
