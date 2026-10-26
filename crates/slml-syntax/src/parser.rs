//! Recursive-descent parser. The grammar is LL(1); see SPEC.md §3.
//!
//! The parser stops at the first error. There is deliberately no error
//! recovery: a document is either valid or rejected with a precise message.

use crate::ast::*;
use crate::diagnostic::{Diagnostic, Span};
use crate::lexer::{Token, TokenKind, tokenize};

pub fn parse(src: &str) -> Result<Document, Diagnostic> {
    let tokens = tokenize(src)?;
    Parser { tokens, pos: 0 }.document()
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.pos]
    }

    fn next(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if &self.peek().kind == kind {
            self.next();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind, context: &str) -> Result<Token, Diagnostic> {
        if self.peek().kind == kind {
            Ok(self.next())
        } else {
            Err(self.unexpected(&format!("{} {context}", kind.describe())))
        }
    }

    fn unexpected(&self, expected: &str) -> Diagnostic {
        let t = self.peek();
        Diagnostic::new(
            t.span,
            format!("expected {expected}, found {}", t.kind.describe()),
        )
    }

    fn ident(&mut self, what: &str) -> Result<(String, Span), Diagnostic> {
        match self.peek().kind.clone() {
            TokenKind::Ident(name) => {
                let span = self.next().span;
                Ok((name, span))
            }
            _ => Err(self.unexpected(what)),
        }
    }

    fn document(&mut self) -> Result<Document, Diagnostic> {
        let header_hint = "every SLML document starts with its language version, e.g. `slml 0.1`";
        match &self.peek().kind {
            TokenKind::Ident(k) if k == "slml" => {
                self.next();
            }
            _ => {
                return Err(
                    Diagnostic::new(self.peek().span, "missing `slml` version line")
                        .with_hint(header_hint),
                );
            }
        }
        let (version, version_span) = match self.peek().kind.clone() {
            TokenKind::Number(_, text) => (text, self.next().span),
            _ => return Err(self.unexpected("a version number").with_hint(header_hint)),
        };

        let mut styles = Vec::new();
        let mut nodes = Vec::new();
        loop {
            match &self.peek().kind {
                TokenKind::Eof => break,
                TokenKind::Ident(k) if k == "style" => styles.push(self.style_def()?),
                TokenKind::Ident(_) => nodes.push(self.node()?),
                TokenKind::Str(_) => {
                    return Err(Diagnostic::new(self.peek().span, "text outside of `page`")
                        .with_hint("put text inside an element, e.g. `text \"...\"`"));
                }
                _ => return Err(self.unexpected("`meta`, `style` or `page`")),
            }
        }
        Ok(Document {
            version,
            version_span,
            styles,
            nodes,
        })
    }

    fn style_def(&mut self) -> Result<StyleDef, Diagnostic> {
        self.next(); // `style`
        let (name, span) = self.ident("a style name after `style`")?;
        if self.peek().kind != TokenKind::LParen {
            return Err(self
                .unexpected("`(` with the style's attributes")
                .with_hint("e.g. `style card(padding: 16, fill: #f4f4f8)`"));
        }
        let attrs = self.attrs()?;
        Ok(StyleDef { name, span, attrs })
    }

    fn node(&mut self) -> Result<Node, Diagnostic> {
        let (name, span) = self.ident("an element name")?;
        let mut classes = Vec::new();
        while self.eat(&TokenKind::Dot) {
            classes.push(self.ident("a style name after `.`")?);
        }
        let attrs = if self.peek().kind == TokenKind::LParen {
            self.attrs()?
        } else {
            Vec::new()
        };
        let mut children = Vec::new();
        if let TokenKind::Str(text) = self.peek().kind.clone() {
            let tspan = self.next().span;
            children.push(Child::Text(text, tspan));
            if self.peek().kind == TokenKind::LBrace {
                return Err(Diagnostic::new(
                    self.peek().span,
                    "an element takes either a text label or a `{ }` block, not both",
                ));
            }
        } else if self.eat(&TokenKind::LBrace) {
            loop {
                match self.peek().kind.clone() {
                    TokenKind::RBrace => {
                        self.next();
                        break;
                    }
                    TokenKind::Eof => {
                        return Err(Diagnostic::new(
                            span,
                            format!("`{name}` block is never closed"),
                        )
                        .with_hint("add the matching `}`"));
                    }
                    TokenKind::Str(text) => {
                        let tspan = self.next().span;
                        children.push(Child::Text(text, tspan));
                    }
                    TokenKind::Ident(_) => children.push(Child::Node(self.node()?)),
                    _ => return Err(self.unexpected("an element, a string or `}`")),
                }
            }
        }
        Ok(Node {
            name,
            span,
            classes,
            attrs,
            children,
        })
    }

    fn attrs(&mut self) -> Result<Vec<Attr>, Diagnostic> {
        self.expect(TokenKind::LParen, "to start attributes")?;
        let mut attrs = Vec::new();
        loop {
            if self.eat(&TokenKind::RParen) {
                return Ok(attrs);
            }
            let (name, span) = self.ident("an attribute name or `)`")?;
            if self.peek().kind != TokenKind::Colon {
                return Err(self
                    .unexpected(&format!("`:` after `{name}`"))
                    .with_hint(format!("attributes are written `{name}: value`")));
            }
            self.next();
            let tok = self.next();
            let value = match tok.kind {
                TokenKind::Number(n, _) => Value::Number(n),
                TokenKind::Str(s) => Value::Str(s),
                TokenKind::Color(c) => Value::Color(c),
                TokenKind::Ident(i) => Value::Ident(i),
                other => {
                    return Err(Diagnostic::new(
                        tok.span,
                        format!("expected a value for `{name}`, found {}", other.describe()),
                    ));
                }
            };
            attrs.push(Attr {
                name,
                span,
                value,
                value_span: tok.span,
            });
            // Commas between attributes are optional; newlines work too.
            self.eat(&TokenKind::Comma);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_document() {
        let doc = parse(
            r#"slml 0.1
            style card(padding: 16 fill: #eee)
            page {
              row.card(gap: 8) {
                text "Hi " bold "there"
                image(src: "a.png", alt: "A")
              }
            }"#,
        )
        .unwrap();
        assert_eq!(doc.version, "0.1");
        assert_eq!(doc.styles[0].attrs.len(), 2);
        let page = &doc.nodes[0];
        let Child::Node(row) = &page.children[0] else {
            panic!()
        };
        assert_eq!(row.classes[0].0, "card");
        assert_eq!(row.children.len(), 3, "text, bold, image");
    }

    #[test]
    fn rejects_label_and_block() {
        let err = parse("slml 0.1 page { text \"a\" { } }").unwrap_err();
        assert!(err.message.contains("not both"));
    }

    #[test]
    fn requires_header() {
        assert!(parse("page { }").unwrap_err().message.contains("version"));
    }

    #[test]
    fn unclosed_block_points_at_opener() {
        let err = parse("slml 0.1\npage {\n  column {\n").unwrap_err();
        assert_eq!(err.span.line, 3);
    }
}
