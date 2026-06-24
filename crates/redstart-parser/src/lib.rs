//! Lexer and parser for the Redstart subgraph language.
//!
//! Redstart unifies `schema.graphql`, `subgraph.yaml`, and the AssemblyScript
//! mappings into a single language. This crate turns Redstart source text into a
//! typed [`ast::Program`].
//!
//! ```
//! use redstart_parser::{lex, parse};
//! use std::sync::Arc;
//!
//! let src = r#"entity Token { id: Id<Bytes> symbol: String }"#;
//! let lexed = lex(src).expect("lex");
//! let (program, errors) = parse(lexed.tokens(), Arc::from(src));
//! assert!(errors.is_empty());
//! assert_eq!(program.entities.len(), 1);
//! ```

#![forbid(unsafe_code)]

pub mod ast;
mod error;
mod lexer;
mod parser;
pub mod span;
mod token;

pub use ast::*;
pub use error::{ParseError, ParseErrors};
pub use lexer::{lex, lex_named, LexError, LexErrorLocation, LexResult, Spanned};
pub use parser::parse;
pub use span::{Ident, Span};
pub use token::Token;
