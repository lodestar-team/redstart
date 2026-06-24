//! Lexer: turns source text into a vector of spanned tokens.
//!
//! Errors are collected rather than fail-fast, so a single pass reports every
//! invalid character at once. Rendering is delegated to `miette`.

use crate::Token;
use logos::Logos;
use miette::{Diagnostic, LabeledSpan, NamedSource, SourceCode};
use std::sync::Arc;
use thiserror::Error;

/// A token paired with its byte span.
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned {
    /// The token kind.
    pub token: Token,
    /// Start byte offset.
    pub start: usize,
    /// End byte offset (exclusive).
    pub end: usize,
}

impl Spanned {
    /// Create a new spanned token.
    #[must_use]
    pub fn new(token: Token, start: usize, end: usize) -> Self {
        Self { token, start, end }
    }

    /// The span as a `(start, end)` tuple.
    #[must_use]
    pub fn span(&self) -> (usize, usize) {
        (self.start, self.end)
    }
}

/// A single lexer error location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexErrorLocation {
    /// Start byte offset of the invalid token.
    pub start: usize,
    /// End byte offset of the invalid token.
    pub end: usize,
    /// The invalid text.
    pub text: String,
}

/// A lexing failure carrying every invalid location found.
#[derive(Error, Debug)]
#[error("failed to lex source: {count} error(s) found")]
pub struct LexError {
    source_code: NamedSource<String>,
    /// All invalid locations found during lexing.
    pub errors: Vec<LexErrorLocation>,
    count: usize,
}

impl LexError {
    fn new(filename: &str, source: String, errors: Vec<LexErrorLocation>) -> Self {
        let count = errors.len();
        Self {
            source_code: NamedSource::new(filename, source),
            errors,
            count,
        }
    }
}

impl Diagnostic for LexError {
    fn code<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        Some(Box::new("redstart::lex::E001"))
    }

    fn help<'a>(&'a self) -> Option<Box<dyn fmt::Display + 'a>> {
        Some(Box::new("remove or replace the invalid character(s)"))
    }

    fn source_code(&self) -> Option<&dyn SourceCode> {
        Some(&self.source_code)
    }

    fn labels(&self) -> Option<Box<dyn Iterator<Item = LabeledSpan> + '_>> {
        let labels = self.errors.iter().map(|e| {
            LabeledSpan::new_with_span(
                Some(format!("invalid token `{}`", e.text)),
                (e.start, e.end - e.start),
            )
        });
        Some(Box::new(labels))
    }
}

use std::fmt;

/// The successful result of lexing.
#[derive(Debug)]
pub struct LexResult {
    /// The lexed tokens.
    pub tokens: Vec<Spanned>,
    /// The source, shared for downstream diagnostics.
    pub source: Arc<str>,
}

impl LexResult {
    /// The tokens as a slice.
    #[must_use]
    pub fn tokens(&self) -> &[Spanned] {
        &self.tokens
    }
}

/// Lex `source`, collecting all invalid-character errors.
///
/// # Errors
/// Returns [`LexError`] if any character cannot be tokenised.
pub fn lex(source: &str) -> Result<LexResult, LexError> {
    lex_named(source, "<input>")
}

/// Lex `source`, attaching `filename` to diagnostics.
///
/// # Errors
/// Returns [`LexError`] if any character cannot be tokenised.
pub fn lex_named(source: &str, filename: &str) -> Result<LexResult, LexError> {
    let source_arc: Arc<str> = Arc::from(source);
    let mut tokens = Vec::new();
    let mut errors = Vec::new();

    for (result, span) in Token::lexer(source).spanned() {
        match result {
            Ok(token) => tokens.push(Spanned::new(token, span.start, span.end)),
            Err(()) => errors.push(LexErrorLocation {
                start: span.start,
                end: span.end,
                text: source[span.start..span.end].to_string(),
            }),
        }
    }

    if errors.is_empty() {
        Ok(LexResult {
            tokens,
            source: source_arc,
        })
    } else {
        Err(LexError::new(filename, source.to_string(), errors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lex_entity_header() {
        let toks = lex("entity Pool {").unwrap();
        let t = toks.tokens();
        assert_eq!(t[0].token, Token::KwEntity);
        assert_eq!(t[1].token, Token::Ident);
        assert_eq!(t[2].token, Token::LBrace);
    }

    #[test]
    fn hex_beats_int() {
        let toks = lex("0x8ad599c3").unwrap();
        assert_eq!(toks.tokens().len(), 1);
        assert_eq!(toks.tokens()[0].token, Token::HexLit);
    }

    #[test]
    fn decimal_beats_int_dot() {
        let toks = lex("12.5").unwrap();
        assert_eq!(toks.tokens().len(), 1);
        assert_eq!(toks.tokens()[0].token, Token::DecimalLit);
    }

    #[test]
    fn method_call_is_int_dot_ident() {
        // `12.toDecimal` must be Int, Dot, Ident — not a decimal literal.
        let toks = lex("12.toDecimal").unwrap();
        let kinds: Vec<_> = toks.tokens().iter().map(|s| s.token).collect();
        assert_eq!(kinds, vec![Token::IntLit, Token::Dot, Token::Ident]);
    }

    #[test]
    fn only_one_equality_operator() {
        // `===` lexes as `==` then `=`, since `===` is not in the grammar.
        let toks = lex("a === b").unwrap();
        let kinds: Vec<_> = toks.tokens().iter().map(|s| s.token).collect();
        assert_eq!(
            kinds,
            vec![Token::Ident, Token::EqEq, Token::Eq, Token::Ident]
        );
    }

    #[test]
    fn comments_are_skipped() {
        let toks = lex("entity // a line comment\n Pool /* block */ {").unwrap();
        let kinds: Vec<_> = toks.tokens().iter().map(|s| s.token).collect();
        assert_eq!(
            kinds,
            vec![Token::KwEntity, Token::Ident, Token::LBrace]
        );
    }

    #[test]
    fn invalid_char_reports() {
        let err = lex("entity # Pool").unwrap_err();
        assert_eq!(err.errors.len(), 1);
        assert_eq!(err.errors[0].text, "#");
    }
}
