//! Turns source text into tokens. See SPEC.md §2 (Lexical structure).

use crate::diagnostic::{Diagnostic, Span};

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    /// `column`, `max-width`, `padding-x` …
    Ident(String),
    /// `$title`: a component parameter (without the `$`).
    Param(String),
    /// `16`, `0.5`. The original text is kept so `slml 0.1` can be read exactly.
    Number(f64, String),
    /// `"Hello"` with escapes already resolved, or a raw `"""` block.
    Str(String),
    /// `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`, normalized to 0xRRGGBBAA.
    Color(u32),
    LBrace,
    RBrace,
    LParen,
    RParen,
    Colon,
    Comma,
    Dot,
    Eof,
}

impl TokenKind {
    pub fn describe(&self) -> String {
        match self {
            TokenKind::Ident(s) => format!("`{s}`"),
            TokenKind::Param(s) => format!("`${s}`"),
            TokenKind::Number(_, s) => format!("number `{s}`"),
            TokenKind::Str(_) => "a string".into(),
            TokenKind::Color(_) => "a color".into(),
            TokenKind::LBrace => "`{`".into(),
            TokenKind::RBrace => "`}`".into(),
            TokenKind::LParen => "`(`".into(),
            TokenKind::RParen => "`)`".into(),
            TokenKind::Colon => "`:`".into(),
            TokenKind::Comma => "`,`".into(),
            TokenKind::Dot => "`.`".into(),
            TokenKind::Eof => "end of file".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// Tokenizes `src`; `file` is recorded in every span (see [`Span::file`]).
pub fn tokenize(src: &str, file: u32) -> Result<Vec<Token>, Diagnostic> {
    tokenize_with_comments(src, file).map(|(tokens, _)| tokens)
}

/// Comments with their positions.
pub type Comments = Vec<(Span, String)>;

/// Like [`tokenize`], also returning the comments.
pub fn tokenize_with_comments(src: &str, file: u32) -> Result<(Vec<Token>, Comments), Diagnostic> {
    let mut lexer = Lexer {
        src,
        file,
        pos: 0,
        line: 1,
        col: 1,
        comments: Vec::new(),
    };
    let tokens = lexer.run()?;
    Ok((tokens, lexer.comments))
}

struct Lexer<'a> {
    src: &'a str,
    file: u32,
    pos: usize,
    line: u32,
    col: u32,
    comments: Vec<(Span, String)>,
}

impl Lexer<'_> {
    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn peek2(&self) -> Option<char> {
        let mut it = self.src[self.pos..].chars();
        it.next();
        it.next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn span(&self, start: usize, end: usize, line: u32, col: u32) -> Span {
        Span {
            file: self.file,
            start,
            end,
            line,
            col,
        }
    }

    fn span_from(&self, start: usize, line: u32, col: u32) -> Span {
        self.span(start, self.pos, line, col)
    }

    fn run(&mut self) -> Result<Vec<Token>, Diagnostic> {
        let mut tokens = Vec::new();
        loop {
            self.skip_trivia()?;
            let (start, line, col) = (self.pos, self.line, self.col);
            let Some(c) = self.bump() else {
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    span: self.span_from(start, line, col),
                });
                return Ok(tokens);
            };
            let kind = match c {
                '{' => TokenKind::LBrace,
                '}' => TokenKind::RBrace,
                '(' => TokenKind::LParen,
                ')' => TokenKind::RParen,
                ':' => TokenKind::Colon,
                ',' => TokenKind::Comma,
                '.' => TokenKind::Dot,
                '"' if self.src[self.pos..].starts_with("\"\"") => {
                    self.bump();
                    self.bump();
                    self.raw_string(start, line, col)?
                }
                '"' => self.string(start, line, col)?,
                '#' => self.color(start, line, col)?,
                '$' => {
                    if !self
                        .peek()
                        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
                    {
                        return Err(Diagnostic::new(
                            self.span_from(start, line, col),
                            "expected a parameter name after `$`",
                        ));
                    }
                    let name_start = self.pos;
                    match self.ident(name_start) {
                        TokenKind::Ident(name) => TokenKind::Param(name),
                        other => other,
                    }
                }
                c if c.is_ascii_digit() => self.number(start),
                c if c.is_ascii_alphabetic() || c == '_' => self.ident(start),
                other => {
                    let span = self.span_from(start, line, col);
                    let mut d = Diagnostic::new(span, format!("unexpected character `{other}`"));
                    if other == '<' {
                        d = d.with_hint(
                            "SLML has no tags; write `text \"...\"` instead of `<p>...</p>`",
                        );
                    } else if other == '\'' {
                        d = d.with_hint("strings use double quotes: \"like this\"");
                    }
                    return Err(d);
                }
            };
            tokens.push(Token {
                kind,
                span: self.span_from(start, line, col),
            });
        }
    }

    fn skip_trivia(&mut self) -> Result<(), Diagnostic> {
        loop {
            match self.peek() {
                // Whitespace (including newlines and tabs) never carries
                // meaning, so reformatting cannot change a document.
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                Some('/') if self.peek2() == Some('/') => {
                    let (start, line, col) = (self.pos, self.line, self.col);
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                    let span = self.span_from(start, line, col);
                    self.comments
                        .push((span, self.src[start..self.pos].to_string()));
                }
                Some('/') if self.peek2() == Some('*') => {
                    let (start, line, col) = (self.pos, self.line, self.col);
                    self.bump();
                    self.bump();
                    loop {
                        match self.bump() {
                            Some('*') if self.peek() == Some('/') => {
                                self.bump();
                                let span = self.span_from(start, line, col);
                                self.comments
                                    .push((span, self.src[start..self.pos].to_string()));
                                break;
                            }
                            Some(_) => {}
                            None => {
                                return Err(Diagnostic::new(
                                    self.span_from(start, line, col),
                                    "unterminated comment",
                                )
                                .with_hint("block comments end with `*/`"));
                            }
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    fn ident(&mut self, start: usize) -> TokenKind {
        while let Some(c) = self.peek() {
            // A hyphen is part of a name only when followed by a letter, so
            // `max-width` is one identifier.
            let hyphen_word = c == '-' && self.peek2().is_some_and(|n| n.is_ascii_alphabetic());
            if c.is_ascii_alphanumeric() || c == '_' || hyphen_word {
                self.bump();
            } else {
                break;
            }
        }
        TokenKind::Ident(self.src[start..self.pos].to_string())
    }

    fn number(&mut self, start: usize) -> TokenKind {
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.bump();
        }
        if self.peek() == Some('.') && self.peek2().is_some_and(|c| c.is_ascii_digit()) {
            self.bump();
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.bump();
            }
        }
        let text = &self.src[start..self.pos];
        TokenKind::Number(text.parse().unwrap_or(0.0), text.to_string())
    }

    fn string(&mut self, start: usize, line: u32, col: u32) -> Result<TokenKind, Diagnostic> {
        let mut out = String::new();
        loop {
            let esc_start = (self.pos, self.line, self.col);
            match self.bump() {
                Some('"') => return Ok(TokenKind::Str(out)),
                Some('\n') | None => {
                    return Err(Diagnostic::new(
                        self.span(start, start + 1, line, col),
                        "unterminated string",
                    )
                    .with_hint(
                        "strings end on the same line; use `\\n` for a line break or a \"\"\" block for several lines",
                    ));
                }
                Some('\\') => match self.bump() {
                    Some('"') => out.push('"'),
                    Some('\\') => out.push('\\'),
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('u') => out.push(self.unicode_escape(esc_start)?),
                    _ => {
                        return Err(Diagnostic::new(
                            self.span_from(esc_start.0, esc_start.1, esc_start.2),
                            "unknown escape sequence",
                        )
                        .with_hint(r#"valid escapes: \" \\ \n \t \u{1F600}"#));
                    }
                },
                Some(c) => out.push(c),
            }
        }
    }

    /// `"""` block string: raw (no escapes), may span lines. The line break
    /// after the opening `"""` is dropped, and if the closing `"""` stands on
    /// its own line, that line is dropped and its indentation is removed
    /// from every line. See SPEC.md §2.
    fn raw_string(&mut self, start: usize, line: u32, col: u32) -> Result<TokenKind, Diagnostic> {
        let body_start = self.pos;
        loop {
            if self.src[self.pos..].starts_with("\"\"\"") {
                break;
            }
            if self.bump().is_none() {
                return Err(Diagnostic::new(
                    self.span(start, start + 3, line, col),
                    "unterminated \"\"\" string",
                )
                .with_hint("close it with \"\"\""));
            }
        }
        let body = &self.src[body_start..self.pos];
        self.bump();
        self.bump();
        self.bump();
        Ok(TokenKind::Str(dedent(body)))
    }

    fn unicode_escape(&mut self, esc: (usize, u32, u32)) -> Result<char, Diagnostic> {
        let err = |l: &Self| {
            Diagnostic::new(l.span_from(esc.0, esc.1, esc.2), "invalid unicode escape")
                .with_hint(r"write it as \u{XXXX} with 1-6 hex digits")
        };
        if self.bump() != Some('{') {
            return Err(err(self));
        }
        let hex_start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
            self.bump();
        }
        let hex = &self.src[hex_start..self.pos];
        if self.bump() != Some('}') || hex.is_empty() || hex.len() > 6 {
            return Err(err(self));
        }
        u32::from_str_radix(hex, 16)
            .ok()
            .and_then(char::from_u32)
            .ok_or_else(|| err(self))
    }

    fn color(&mut self, start: usize, line: u32, col: u32) -> Result<TokenKind, Diagnostic> {
        let hex_start = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_alphanumeric()) {
            self.bump();
        }
        let hex = &self.src[hex_start..self.pos];
        let bad = || {
            Diagnostic::new(
                self.span(start, hex_start + hex.len(), line, col),
                format!("invalid color `#{hex}`"),
            )
            .with_hint("colors are #rgb, #rgba, #rrggbb or #rrggbbaa")
        };
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(bad());
        }
        let expanded: String = match hex.len() {
            3 | 4 => hex.chars().flat_map(|c| [c, c]).collect(),
            6 | 8 => hex.to_string(),
            _ => return Err(bad()),
        };
        let mut value = u32::from_str_radix(&expanded, 16).map_err(|_| bad())?;
        if expanded.len() == 6 {
            value = (value << 8) | 0xff;
        }
        Ok(TokenKind::Color(value))
    }
}

pub(crate) fn dedent(body: &str) -> String {
    let body = body
        .strip_prefix("\r\n")
        .or_else(|| body.strip_prefix('\n'))
        .unwrap_or(body);
    let mut lines: Vec<&str> = body
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .collect();
    let indent = match lines.last() {
        Some(last) if last.chars().all(|c| c == ' ' || c == '\t') && lines.len() > 1 => {
            let n = last.chars().count();
            lines.pop();
            n
        }
        Some(last) if last.chars().all(|c| c == ' ' || c == '\t') => {
            lines.pop();
            0
        }
        _ => 0,
    };
    lines
        .iter()
        .map(|l| {
            let strip = l
                .chars()
                .take(indent)
                .take_while(|c| *c == ' ' || *c == '\t')
                .map(char::len_utf8)
                .sum::<usize>();
            &l[strip..]
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        tokenize(src, 0)
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn basic_tokens() {
        assert_eq!(
            kinds(r##"row(max-width: 12.5, fill: #fff) { "a\"b" $t } // hi"##),
            vec![
                TokenKind::Ident("row".into()),
                TokenKind::LParen,
                TokenKind::Ident("max-width".into()),
                TokenKind::Colon,
                TokenKind::Number(12.5, "12.5".into()),
                TokenKind::Comma,
                TokenKind::Ident("fill".into()),
                TokenKind::Colon,
                TokenKind::Color(0xffffffff),
                TokenKind::RParen,
                TokenKind::LBrace,
                TokenKind::Str("a\"b".into()),
                TokenKind::Param("t".into()),
                TokenKind::RBrace,
                TokenKind::Eof,
            ]
        );
    }

    #[test]
    fn unicode_and_positions() {
        let toks = tokenize("\"zażółć\"\n  \"\\u{1F600}\"", 3).unwrap();
        assert_eq!(toks[0].kind, TokenKind::Str("zażółć".into()));
        assert_eq!(toks[1].kind, TokenKind::Str("😀".into()));
        assert_eq!(
            (toks[1].span.line, toks[1].span.col, toks[1].span.file),
            (2, 3, 3)
        );
    }

    #[test]
    fn raw_block_strings_are_dedented() {
        let src = "codeblock \"\"\"\n    page {\n      text \"hi\"\n    }\n    \"\"\" x";
        let toks = kinds(src);
        assert_eq!(toks[1], TokenKind::Str("page {\n  text \"hi\"\n}".into()));
        assert_eq!(toks[2], TokenKind::Ident("x".into()));
        assert_eq!(kinds(r#""""a\nb""""#)[0], TokenKind::Str("a\\nb".into()));
    }

    #[test]
    fn errors() {
        assert!(
            tokenize("\"open", 0)
                .unwrap_err()
                .message
                .contains("unterminated")
        );
        assert!(
            tokenize("\"\"\"open", 0)
                .unwrap_err()
                .message
                .contains("unterminated")
        );
        assert!(
            tokenize("#12345", 0)
                .unwrap_err()
                .message
                .contains("invalid color")
        );
        assert!(tokenize("<p>", 0).unwrap_err().hint.is_some());
        assert!(tokenize("/* x", 0).is_err());
        assert!(tokenize("$ x", 0).is_err());
    }
}
