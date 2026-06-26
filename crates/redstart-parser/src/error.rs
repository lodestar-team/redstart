//! Parser diagnostics.
//!
//! Errors are the product (per the design report's "errors teach" principle), so
//! every [`ParseError`] renders through `miette` with a precise label and a help
//! line suggesting the fix.

use crate::span::Span;
use miette::{Diagnostic, LabeledSpan, NamedSource, SourceCode};
use std::fmt;
use thiserror::Error;

/// A single parse error with a span and an optional fix hint.
#[derive(Error, Debug, Clone)]
#[error("{message}")]
pub struct ParseError {
    /// The headline message.
    pub message: String,
    /// What the parser was looking at when it gave up.
    pub label: String,
    /// An optional help line.
    pub help: Option<String>,
    /// The offending span.
    pub span: Span,
}

impl ParseError {
    /// Construct a parse error.
    pub fn new(message: impl Into<String>, label: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            label: label.into(),
            help: None,
            span,
        }
    }

    /// Attach a help line.
    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

/// A bundle of parse errors carrying the named source for rendering.
#[derive(Error, Debug)]
#[error("parsing failed: {} error(s)", errors.len())]
pub struct ParseErrors {
    /// The named source code.
    pub source_code: NamedSource<String>,
    /// The individual errors.
    pub errors: Vec<ParseError>,
}

impl Diagnostic for ParseErrors {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        Some(Box::new("redstart::parse"))
    }

    fn source_code(&self) -> Option<&dyn SourceCode> {
        Some(&self.source_code)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        let labels = self
            .errors
            .iter()
            .map(|e| LabeledSpan::new_with_span(Some(e.label.clone()), e.span.label()));
        Some(Box::new(labels))
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        // Surface the first available help line.
        self.errors
            .iter()
            .find_map(|e| e.help.clone())
            .map(|h| Box::new(h) as Box<dyn fmt::Display>)
    }
}

impl ParseErrors {
    /// Bundle parse errors with their source for rendering.
    #[must_use]
    pub fn new(filename: &str, source: String, errors: Vec<ParseError>) -> Self {
        Self {
            source_code: NamedSource::new(filename, source),
            errors,
        }
    }
}
