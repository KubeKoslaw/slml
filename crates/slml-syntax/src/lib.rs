//! # slml-syntax
//!
//! Front end of the SLML document language: source text → tokens → syntax
//! tree → validated, typed [`model::Document`].
//!
//! ```
//! let doc = slml_syntax::compile(r#"
//!     slml 0.1
//!     page { text "Hello" }
//! "#).unwrap();
//! assert_eq!(doc.page.children.len(), 1);
//! ```

pub mod analyze;
pub mod ast;
pub mod diagnostic;
pub mod lexer;
pub mod model;
pub mod parser;

pub use diagnostic::{Diagnostic, Span};

/// Parses source text into an untyped syntax tree.
pub fn parse(src: &str) -> Result<ast::Document, Diagnostic> {
    parser::parse(src)
}

/// Parses and validates a document. Returns every error found.
pub fn compile(src: &str) -> Result<model::Document, Vec<Diagnostic>> {
    let ast = parse(src).map_err(|d| vec![d])?;
    analyze::analyze(&ast)
}
