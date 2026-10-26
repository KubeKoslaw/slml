//! Untyped syntax tree, exactly as written in the source.
//! [`crate::analyze`] turns it into the typed [`crate::model`].

use crate::diagnostic::Span;

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// The `0.1` in `slml 0.1`.
    pub version: String,
    pub version_span: Span,
    pub styles: Vec<StyleDef>,
    /// Top-level nodes: `meta` and `page`.
    pub nodes: Vec<Node>,
}

/// `style card(padding: 16, fill: #eee)`
#[derive(Debug, Clone, PartialEq)]
pub struct StyleDef {
    pub name: String,
    pub span: Span,
    pub attrs: Vec<Attr>,
}

/// `row.card(gap: 8) { ... }` or `text "Hello"`.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub name: String,
    pub span: Span,
    /// Style names applied with `.name`, in order.
    pub classes: Vec<(String, Span)>,
    pub attrs: Vec<Attr>,
    pub children: Vec<Child>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Child {
    Node(Node),
    Text(String, Span),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attr {
    pub name: String,
    pub span: Span,
    pub value: Value,
    pub value_span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    Str(String),
    /// 0xRRGGBBAA
    Color(u32),
    Ident(String),
}

impl Value {
    pub fn describe(&self) -> &'static str {
        match self {
            Value::Number(_) => "a number",
            Value::Str(_) => "a string",
            Value::Color(_) => "a color",
            Value::Ident(_) => "a name",
        }
    }
}
