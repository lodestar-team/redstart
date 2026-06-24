//! Source span types for error reporting.
//!
//! Spans carry an `Arc<str>` handle to their originating source so that any AST
//! node can render a `miette` diagnostic without the caller threading the source
//! text back through every layer.

use std::fmt;
use std::sync::Arc;

/// A byte range in a source file, plus a handle to that source.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Span {
    /// Byte offset of the start of the span.
    pub start: usize,
    /// Byte offset of the end of the span (exclusive).
    pub end: usize,
    /// The source text this span refers to.
    pub source: Arc<str>,
}

impl Span {
    /// Create a new span.
    #[must_use]
    pub fn new(start: usize, end: usize, source: Arc<str>) -> Self {
        Self { start, end, source }
    }

    /// Create a dummy span for testing or synthetic nodes.
    #[must_use]
    pub fn dummy() -> Self {
        Self {
            start: 0,
            end: 0,
            source: Arc::from(""),
        }
    }

    /// Length of this span in bytes.
    #[must_use]
    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Whether this span is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The source text covered by this span.
    #[must_use]
    pub fn text(&self) -> &str {
        self.source.get(self.start..self.end).unwrap_or("")
    }

    /// Merge two spans into one that covers both.
    #[must_use]
    pub fn merge(&self, other: &Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
            source: Arc::clone(&self.source),
        }
    }

    /// 1-indexed line and column for the start of this span.
    #[must_use]
    pub fn line_col(&self) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
        for (i, ch) in self.source.char_indices() {
            if i >= self.start {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    /// The span as a `(offset, length)` tuple, for `miette` labels.
    #[must_use]
    pub fn label(&self) -> (usize, usize) {
        (self.start, self.len())
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (line, col) = self.line_col();
        write!(f, "{line}:{col}..{}", self.len())
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (line, col) = self.line_col();
        write!(f, "{line}:{col}")
    }
}

/// An identifier with its source span.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Ident {
    /// The identifier name.
    pub name: String,
    /// The source span.
    pub span: Span,
}

impl Ident {
    /// Create a new identifier.
    #[must_use]
    pub fn new(name: impl Into<String>, span: Span) -> Self {
        Self {
            name: name.into(),
            span,
        }
    }

    /// Create a dummy identifier for testing.
    #[must_use]
    pub fn dummy(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            span: Span::dummy(),
        }
    }
}

impl fmt::Debug for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Ident({:?})", self.name)
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_text_and_line_col() {
        let source: Arc<str> = Arc::from("line1\nline2");
        let span = Span::new(6, 11, Arc::clone(&source));
        assert_eq!(span.text(), "line2");
        assert_eq!(span.line_col(), (2, 1));
    }

    #[test]
    fn span_merge_covers_both() {
        let source: Arc<str> = Arc::from("hello world");
        let a = Span::new(0, 5, Arc::clone(&source));
        let b = Span::new(6, 11, Arc::clone(&source));
        let m = a.merge(&b);
        assert_eq!((m.start, m.end), (0, 11));
    }
}
