//! The compact `slml 0.2` dialect: lexer, parser and inline text markup.
//!
//! ```slml
//! slml 0.2
//! @card{p20 bg#fff r12}
//! page{c.card{#"Hello" "Some *bold* text and a [link](/about)."}}
//! ```
//!
//! The result is the same syntax tree the long dialect produces, with
//! canonical names (`column`, `padding`) and the written spelling kept for
//! error messages. See SPEC.md §2–3.

use crate::ast::*;
use crate::diagnostic::{Diagnostic, Span};
use crate::names;

/// Does `src` start with the `slml 0.2` header (after comments)?
pub fn is_compact(src: &str) -> bool {
    let mut rest = src;
    loop {
        rest = rest.trim_start();
        if let Some(r) = rest.strip_prefix("//") {
            rest = r.split_once('\n').map_or("", |(_, after)| after);
        } else if let Some(r) = rest.strip_prefix("/*") {
            rest = r.split_once("*/").map_or("", |(_, after)| after);
        } else {
            break;
        }
    }
    let Some(r) = rest.strip_prefix("slml") else {
        return false;
    };
    let r = r.trim_start();
    r.starts_with("0.2") && !r[3..].starts_with(|c: char| c.is_ascii_digit())
}

pub fn parse(src: &str, file: u32) -> Result<Document, Diagnostic> {
    let mut lexer = Lexer::new(src, file, 0, 1, 1);
    let tokens = lexer.run()?;
    let mut parser = Parser {
        toks: tokens,
        pos: 0,
    };
    let mut doc = parser.document()?;
    doc.comments = lexer.comments;
    Ok(doc)
}

// ---- lexer --------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum K {
    Word(String),
    Num(f64, String),
    Color(u32),
    Colon,
    /// `raw` is the text between the quotes with escapes unprocessed;
    /// for `"""` blocks it is the final (dedented) text.
    Str {
        raw: String,
        block: bool,
    },
    Param(String),
    Path(String),
    /// `>` followed by its target.
    To(Box<K>),
    Heading(u8),
    Marker(char),
    Tilde,
    Bar,
    At,
    LBrace,
    RBrace,
    LParen,
    RParen,
    Comma,
    Dot,
    Eof,
}

impl K {
    fn describe(&self) -> String {
        match self {
            K::Word(w) => format!("`{w}`"),
            K::Num(_, t) => format!("number `{t}`"),
            K::Color(_) => "a color".into(),
            K::Colon => "`:`".into(),
            K::Str { .. } => "a string".into(),
            K::Param(p) => format!("`${p}`"),
            K::Path(p) => format!("path `{p}`"),
            K::To(_) => "a `>` target".into(),
            K::Heading(n) => format!("`{}`", "#".repeat(*n as usize)),
            K::Marker(m) => format!("`{m}`"),
            K::Tilde => "`~`".into(),
            K::Bar => "`|`".into(),
            K::At => "`@`".into(),
            K::LBrace => "`{`".into(),
            K::RBrace => "`}`".into(),
            K::LParen => "`(`".into(),
            K::RParen => "`)`".into(),
            K::Comma => "`,`".into(),
            K::Dot => "`.`".into(),
            K::Eof => "end of file".into(),
        }
    }
}

#[derive(Debug, Clone)]
struct Tok {
    k: K,
    span: Span,
    /// No whitespace or comment between this token and the previous one.
    glued: bool,
}

struct Lexer<'a> {
    src: &'a str,
    file: u32,
    pos: usize,
    line: u32,
    col: u32,
    comments: Vec<(Span, String)>,
    tokens: Vec<Tok>,
}

fn word_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn path_char(c: char) -> bool {
    !c.is_whitespace() && !matches!(c, '{' | '}' | '"')
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str, file: u32, pos: usize, line: u32, col: u32) -> Self {
        Self {
            src,
            file,
            pos,
            line,
            col,
            comments: Vec::new(),
            tokens: Vec::new(),
        }
    }

    fn rest(&self) -> &str {
        &self.src[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        self.rest().chars().nth(n)
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

    fn span(&self, start: (usize, u32, u32)) -> Span {
        Span {
            file: self.file,
            start: start.0,
            end: self.pos,
            line: start.1,
            col: start.2,
        }
    }

    fn here(&self) -> (usize, u32, u32) {
        (self.pos, self.line, self.col)
    }

    fn err(&self, start: (usize, u32, u32), msg: impl Into<String>) -> Diagnostic {
        Diagnostic::new(self.span(start), msg)
    }

    fn prev_is_name(&self, glued: bool) -> bool {
        glued
            && matches!(
                self.tokens.last().map(|t| &t.k),
                Some(K::Word(_)) | Some(K::Param(_))
            )
    }

    fn run(&mut self) -> Result<Vec<Tok>, Diagnostic> {
        loop {
            let glued = !self.skip_trivia()?;
            let start = self.here();
            let Some(c) = self.peek() else {
                self.tokens.push(Tok {
                    k: K::Eof,
                    span: self.span(start),
                    glued,
                });
                return Ok(std::mem::take(&mut self.tokens));
            };
            let k = match c {
                '{' | '}' | '(' | ')' | ',' | ':' | '~' | '|' | '@' => {
                    self.bump();
                    match c {
                        '{' => K::LBrace,
                        '}' => K::RBrace,
                        '(' => K::LParen,
                        ')' => K::RParen,
                        ',' => K::Comma,
                        ':' => K::Colon,
                        '~' => K::Tilde,
                        '|' => K::Bar,
                        _ => K::At,
                    }
                }
                '.' if self.peek_at(1) == Some('/') => self.path(),
                '.' => {
                    self.bump();
                    K::Dot
                }
                '/' => self.path(),
                '"' => self.string(start)?,
                '$' => {
                    self.bump();
                    if !self.peek().is_some_and(word_start) {
                        return Err(self.err(start, "expected a parameter name after `$`"));
                    }
                    let s = self.pos;
                    self.word_chars();
                    K::Param(self.src[s..self.pos].to_string())
                }
                '#' if self.prev_is_name(glued) => self.color(start)?,
                '#' => {
                    let mut n = 0;
                    while self.peek() == Some('#') {
                        self.bump();
                        n += 1;
                    }
                    if self.peek().is_some_and(|c| c.is_ascii_hexdigit()) {
                        return Err(self
                            .err(start, "a color needs an attribute name")
                            .with_hint("write it after the attribute, e.g. `bg#fff` or `c#333`"));
                    }
                    if n > 4 {
                        return Err(self.err(start, "headings have levels 1 to 4: `#` … `####`"));
                    }
                    K::Heading(n)
                }
                '-' | '+' | '=' => {
                    self.bump();
                    K::Marker(c)
                }
                '>' => {
                    self.bump();
                    let inner_start = self.here();
                    let target = match self.peek() {
                        Some('"') => self.string(inner_start)?,
                        Some('$') => {
                            self.bump();
                            let s = self.pos;
                            self.word_chars();
                            if s == self.pos {
                                return Err(self.err(start, "expected a parameter name after `>$`"));
                            }
                            K::Param(self.src[s..self.pos].to_string())
                        }
                        Some(c) if path_char(c) => {
                            let s = self.pos;
                            while self.peek().is_some_and(path_char) {
                                self.bump();
                            }
                            K::Path(self.src[s..self.pos].to_string())
                        }
                        _ => {
                            return Err(self
                                .err(start, "`>` must be followed by a link target")
                                .with_hint(
                                    "e.g. `>/about.slml`, `>#top` or `>https://example.com`",
                                ));
                        }
                    };
                    K::To(Box::new(target))
                }
                c if c.is_ascii_digit() => {
                    let s = self.pos;
                    while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                        self.bump();
                    }
                    if self.peek() == Some('.')
                        && self.peek_at(1).is_some_and(|c| c.is_ascii_digit())
                    {
                        self.bump();
                        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                            self.bump();
                        }
                    }
                    let text = &self.src[s..self.pos];
                    K::Num(text.parse().unwrap_or(0.0), text.to_string())
                }
                c if word_start(c) => {
                    let s = self.pos;
                    self.word_chars();
                    if self.rest().starts_with("://") {
                        while self.peek().is_some_and(path_char) {
                            self.bump();
                        }
                        K::Path(self.src[s..self.pos].to_string())
                    } else {
                        K::Word(self.src[s..self.pos].to_string())
                    }
                }
                other => {
                    self.bump();
                    let mut d = self.err(start, format!("unexpected character `{other}`"));
                    if other == '<' {
                        d = d.with_hint("SLML has no tags; write `t\"...\"` for text");
                    } else if other == '\'' {
                        d = d.with_hint("strings use double quotes: \"like this\"");
                    }
                    return Err(d);
                }
            };
            self.tokens.push(Tok {
                k,
                span: self.span(start),
                glued,
            });
        }
    }

    /// Letters, `_`, and `-` followed by a letter. No digits: in `p20` the
    /// digits are the value.
    fn word_chars(&mut self) {
        while let Some(c) = self.peek() {
            let hyphen = c == '-' && self.peek_at(1).is_some_and(|n| n.is_ascii_alphabetic());
            if word_start(c) || hyphen {
                self.bump();
            } else {
                break;
            }
        }
    }

    fn path(&mut self) -> K {
        let s = self.pos;
        while self.peek().is_some_and(path_char) {
            self.bump();
        }
        // `./logo.png` is how a relative path is written; it means `logo.png`.
        let p = &self.src[s..self.pos];
        K::Path(p.strip_prefix("./").unwrap_or(p).to_string())
    }

    /// Skips whitespace and comments; returns whether anything was skipped.
    fn skip_trivia(&mut self) -> Result<bool, Diagnostic> {
        let before = self.pos;
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                Some('/') if self.peek_at(1) == Some('/') => {
                    let start = self.here();
                    while self.peek().is_some_and(|c| c != '\n') {
                        self.bump();
                    }
                    self.comments
                        .push((self.span(start), self.src[start.0..self.pos].to_string()));
                }
                Some('/') if self.peek_at(1) == Some('*') => {
                    let start = self.here();
                    self.bump();
                    self.bump();
                    loop {
                        match self.bump() {
                            Some('*') if self.peek() == Some('/') => {
                                self.bump();
                                break;
                            }
                            Some(_) => {}
                            None => {
                                return Err(self
                                    .err(start, "unterminated comment")
                                    .with_hint("block comments end with `*/`"));
                            }
                        }
                    }
                    self.comments
                        .push((self.span(start), self.src[start.0..self.pos].to_string()));
                }
                _ => return Ok(self.pos != before),
            }
        }
    }

    fn string(&mut self, start: (usize, u32, u32)) -> Result<K, Diagnostic> {
        if self.rest().starts_with("\"\"\"") {
            for _ in 0..3 {
                self.bump();
            }
            let body = self.pos;
            while !self.rest().starts_with("\"\"\"") {
                if self.bump().is_none() {
                    return Err(Diagnostic::new(
                        Span {
                            file: self.file,
                            start: start.0,
                            end: start.0 + 3,
                            line: start.1,
                            col: start.2,
                        },
                        "unterminated \"\"\" string",
                    )
                    .with_hint("close it with \"\"\""));
                }
            }
            let text = crate::lexer::dedent(&self.src[body..self.pos]);
            for _ in 0..3 {
                self.bump();
            }
            return Ok(K::Str {
                raw: text,
                block: true,
            });
        }
        self.bump();
        let body = self.pos;
        loop {
            match self.peek() {
                Some('"') => break,
                Some('\\') => {
                    self.bump();
                    if self.peek().is_none_or(|c| c == '\n') {
                        continue;
                    }
                    self.bump();
                }
                Some('\n') | None => {
                    return Err(Diagnostic::new(
                        Span {
                            file: self.file,
                            start: start.0,
                            end: start.0 + 1,
                            line: start.1,
                            col: start.2,
                        },
                        "unterminated string",
                    )
                    .with_hint("strings end on the same line; use `\\n` or a \"\"\" block for several lines"));
                }
                Some(_) => {
                    self.bump();
                }
            }
        }
        let raw = self.src[body..self.pos].to_string();
        self.bump();
        Ok(K::Str { raw, block: false })
    }

    fn color(&mut self, start: (usize, u32, u32)) -> Result<K, Diagnostic> {
        self.bump(); // '#'
        let s = self.pos;
        while self.peek().is_some_and(|c| c.is_ascii_alphanumeric()) {
            self.bump();
        }
        let hex = &self.src[s..self.pos];
        let bad = |l: &Self| {
            l.err(start, format!("invalid color `#{hex}`"))
                .with_hint("colors are #rgb, #rgba, #rrggbb or #rrggbbaa")
        };
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(bad(self));
        }
        let expanded: String = match hex.len() {
            3 | 4 => hex.chars().flat_map(|c| [c, c]).collect(),
            6 | 8 => hex.to_string(),
            _ => return Err(bad(self)),
        };
        let mut value = u32::from_str_radix(&expanded, 16).map_err(|_| bad(self))?;
        if expanded.len() == 6 {
            value = (value << 8) | 0xff;
        }
        Ok(K::Color(value))
    }
}

// ---- escapes and inline markup ----------------------------------------------

/// Span of `len` characters at character `index` inside a one-line string
/// token whose opening quote is at `quote`.
fn inner_span(quote: Span, raw: &str, index: usize, len: usize) -> Span {
    let byte: usize = raw.chars().take(index).map(char::len_utf8).sum();
    let byte_end: usize = raw.chars().take(index + len).map(char::len_utf8).sum();
    Span {
        file: quote.file,
        start: quote.start + 1 + byte,
        end: quote.start + 1 + byte_end,
        line: quote.line,
        col: quote.col + 1 + index as u32,
    }
}

const MARKUP_ESCAPES: &str = "*_`[]$";

/// Resolves escapes in a plain string (attribute values, code).
fn unescape(raw: &str, span: Span) -> Result<String, Diagnostic> {
    let chars: Vec<char> = raw.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '\\' {
            out.push(chars[i]);
            i += 1;
            continue;
        }
        let (c, used) = escape_at(&chars, i, span, raw)?;
        out.push(c);
        i += used;
    }
    Ok(out)
}

/// The escape starting at `chars[i] == '\\'`: the character and how many
/// source characters it used.
fn escape_at(chars: &[char], i: usize, span: Span, raw: &str) -> Result<(char, usize), Diagnostic> {
    let bad = || {
        Diagnostic::new(inner_span(span, raw, i, 2), "unknown escape sequence")
            .with_hint(r#"valid escapes: \" \\ \n \t \u{1F600}, and \* \_ \` \[ \] \$ in text"#)
    };
    match chars.get(i + 1) {
        Some('"') => Ok(('"', 2)),
        Some('\\') => Ok(('\\', 2)),
        Some('n') => Ok(('\n', 2)),
        Some('t') => Ok(('\t', 2)),
        Some(c) if MARKUP_ESCAPES.contains(*c) => Ok((*c, 2)),
        Some('u') => {
            let close = chars[i..].iter().position(|c| *c == '}').map(|p| p + i);
            let invalid = || {
                Diagnostic::new(inner_span(span, raw, i, 2), "invalid unicode escape")
                    .with_hint(r"write it as \u{XXXX} with 1-6 hex digits")
            };
            match (chars.get(i + 2), close) {
                (Some('{'), Some(end)) if end > i + 3 && end - i - 3 <= 6 => {
                    let hex: String = chars[i + 3..end].iter().collect();
                    let c = u32::from_str_radix(&hex, 16)
                        .ok()
                        .and_then(char::from_u32)
                        .ok_or_else(invalid)?;
                    Ok((c, end - i + 1))
                }
                _ => Err(invalid()),
            }
        }
        _ => Err(bad()),
    }
}

/// Parses text with inline markup into children: `*bold*`, `_italic_`,
/// `` `code` ``, `[text](target)`, `[text]{attrs}` and `[text](target){attrs}`.
fn inline(raw: &str, span: Span) -> Result<Vec<Child>, Diagnostic> {
    let chars: Vec<char> = raw.chars().collect();
    let mut p = Inline {
        chars,
        i: 0,
        raw,
        span,
    };
    p.seq(None)
}

struct Inline<'a> {
    chars: Vec<char>,
    i: usize,
    raw: &'a str,
    span: Span,
}

impl Inline<'_> {
    fn at(&self, index: usize, len: usize) -> Span {
        inner_span(self.span, self.raw, index, len)
    }

    fn node(
        &self,
        name: &str,
        written: &str,
        start: usize,
        children: Vec<Child>,
        attrs: Vec<Attr>,
    ) -> Child {
        Child::Node(Node {
            name: name.into(),
            written: Some(written.into()),
            span: self.at(start, 1),
            classes: Vec::new(),
            attrs,
            children,
        })
    }

    fn seq(&mut self, until: Option<char>) -> Result<Vec<Child>, Diagnostic> {
        let mut out = Vec::new();
        let mut buf = String::new();
        let mut buf_start = self.i;
        let opened = self.i.saturating_sub(1);
        loop {
            let Some(c) = self.chars.get(self.i).copied() else {
                if let Some(close) = until {
                    let what = match close {
                        '*' => "*bold*",
                        '_' => "_italic_",
                        _ => "[link]",
                    };
                    return Err(Diagnostic::new(
                        self.at(opened, 1),
                        format!("`{close}` is never closed"),
                    )
                    .with_hint(format!(
                        "{what} needs its closing mark; write `\\{}` for the character itself",
                        if close == ']' { '[' } else { close }
                    )));
                }
                break;
            };
            let flush = |buf: &mut String, out: &mut Vec<Child>, start: usize, me: &Self| {
                if !buf.is_empty() {
                    out.push(Child::Text(std::mem::take(buf), me.at(start, 1)));
                }
            };
            match c {
                '\\' => {
                    let (ch, used) = escape_at(&self.chars, self.i, self.span, self.raw)?;
                    buf.push(ch);
                    self.i += used;
                }
                c if Some(c) == until => {
                    self.i += 1;
                    flush(&mut buf, &mut out, buf_start, self);
                    if out.is_empty() && c != ']' {
                        return Err(
                            Diagnostic::new(self.at(opened, 2), format!("empty `{c}{c}`"))
                                .with_hint(format!("write `\\{c}` for the character itself")),
                        );
                    }
                    return Ok(out);
                }
                '*' | '_' => {
                    flush(&mut buf, &mut out, buf_start, self);
                    let start = self.i;
                    self.i += 1;
                    let inner = self.seq(Some(c))?;
                    let (name, written) = if c == '*' {
                        ("bold", "*…*")
                    } else {
                        ("italic", "_…_")
                    };
                    out.push(self.node(name, written, start, inner, Vec::new()));
                    buf_start = self.i;
                }
                '$' if self
                    .chars
                    .get(self.i + 1)
                    .is_some_and(|c| c.is_ascii_alphabetic() || *c == '_' || *c == '{') =>
                {
                    flush(&mut buf, &mut out, buf_start, self);
                    let start = self.i;
                    let braced = self.chars[self.i + 1] == '{';
                    self.i += if braced { 2 } else { 1 };
                    let name_start = self.i;
                    while self.chars.get(self.i).is_some_and(|c| {
                        c.is_ascii_alphabetic()
                            || *c == '_'
                            || (*c == '-'
                                && self
                                    .chars
                                    .get(self.i + 1)
                                    .is_some_and(|n| n.is_ascii_alphabetic()))
                    }) {
                        self.i += 1;
                    }
                    let name: String = self.chars[name_start..self.i].iter().collect();
                    if braced {
                        if self.chars.get(self.i) != Some(&'}') || name.is_empty() {
                            return Err(Diagnostic::new(self.at(start, 2), "expected `${name}`"));
                        }
                        self.i += 1;
                    }
                    out.push(Child::Param(name, self.at(start, self.i - start)));
                    buf_start = self.i;
                }
                '`' => {
                    flush(&mut buf, &mut out, buf_start, self);
                    let start = self.i;
                    self.i += 1;
                    let mut code = String::new();
                    loop {
                        match self.chars.get(self.i).copied() {
                            None => {
                                return Err(Diagnostic::new(self.at(start, 1), "`` ` `` is never closed")
                                    .with_hint("inline code ends with another `` ` ``; write \\` for the character itself"));
                            }
                            Some('`') => {
                                self.i += 1;
                                break;
                            }
                            Some('\\') => {
                                let (ch, used) =
                                    escape_at(&self.chars, self.i, self.span, self.raw)?;
                                code.push(ch);
                                self.i += used;
                            }
                            Some(ch) => {
                                code.push(ch);
                                self.i += 1;
                            }
                        }
                    }
                    let text = Child::Text(code, self.at(start, 1));
                    out.push(self.node("code", "`…`", start, vec![text], Vec::new()));
                    buf_start = self.i;
                }
                '[' => {
                    flush(&mut buf, &mut out, buf_start, self);
                    let start = self.i;
                    self.i += 1;
                    let inner = self.seq(Some(']'))?;
                    let mut attrs = Vec::new();
                    let mut classes = Vec::new();
                    let mut is_link = false;
                    if self.chars.get(self.i) == Some(&'(') {
                        let t_start = self.i + 1;
                        self.i += 1;
                        let mut target = String::new();
                        loop {
                            match self.chars.get(self.i).copied() {
                                None => {
                                    return Err(Diagnostic::new(
                                        self.at(start, 1),
                                        "link target is never closed",
                                    )
                                    .with_hint("write [text](target) with a closing `)`"));
                                }
                                Some(')') => {
                                    self.i += 1;
                                    break;
                                }
                                Some('\\') if self.chars.get(self.i + 1) == Some(&')') => {
                                    target.push(')');
                                    self.i += 2;
                                }
                                Some(ch) => {
                                    target.push(ch);
                                    self.i += 1;
                                }
                            }
                        }
                        let tspan = self.at(t_start, target.chars().count().max(1));
                        let value = match target.strip_prefix('$') {
                            Some(p)
                                if !p.is_empty()
                                    && p.chars().all(|c| {
                                        c.is_ascii_alphabetic() || c == '_' || c == '-'
                                    }) =>
                            {
                                Value::Param(p.to_string())
                            }
                            _ => Value::Str(target),
                        };
                        attrs.push(Attr {
                            name: "to".into(),
                            written: Some(">".into()),
                            span: tspan,
                            value,
                            value_span: tspan,
                        });
                        is_link = true;
                    }
                    if self.chars.get(self.i) == Some(&'{') {
                        let a_start = self.i + 1;
                        let Some(rel_end) = self.chars[a_start..].iter().position(|c| *c == '}')
                        else {
                            return Err(Diagnostic::new(self.at(self.i, 1), "`{` is never closed")
                                .with_hint("write [text]{attributes}"));
                        };
                        let a_end = a_start + rel_end;
                        let body: String = self.chars[a_start..a_end].iter().collect();
                        let base = self.at(a_start, 1);
                        let (more, cls) = inline_attrs(&body, base)?;
                        attrs.extend(more);
                        classes = cls;
                        self.i = a_end + 1;
                    } else if !is_link {
                        return Err(Diagnostic::new(
                            self.at(start, 1),
                            "`[` starts a link or a styled span",
                        )
                        .with_hint(
                            "write [text](target) or [text]{attributes}; write \\[ for a bracket",
                        ));
                    }
                    let (name, written) = if is_link {
                        ("link", "[…](…)")
                    } else {
                        ("span", "[…]{…}")
                    };
                    let mut node = self.node(name, written, start, inner, attrs);
                    if let Child::Node(n) = &mut node {
                        n.classes = classes;
                    }
                    out.push(node);
                    buf_start = self.i;
                }
                c => {
                    if buf.is_empty() {
                        buf_start = self.i;
                    }
                    buf.push(c);
                    self.i += 1;
                }
            }
        }
        if !buf.is_empty() {
            out.push(Child::Text(buf, self.at(buf_start, 1)));
        }
        Ok(out)
    }
}

/// Attributes and `.styles` inside `[text]{…}`.
#[allow(clippy::type_complexity)]
fn inline_attrs(body: &str, at: Span) -> Result<(Vec<Attr>, Vec<(String, Span)>), Diagnostic> {
    let mut lexer = Lexer::new(body, at.file, 0, at.line, at.col);
    let mut toks = lexer.run()?;
    // Spans from the sub-lexer are relative to `body`; move them into the file.
    for t in &mut toks {
        t.span.start += at.start;
        t.span.end += at.start;
    }
    let mut p = Parser { toks, pos: 0 };
    let mut attrs = Vec::new();
    let mut classes = Vec::new();
    while p.peek().k != K::Eof {
        if p.peek().k == K::Dot {
            p.next();
            match p.next() {
                Tok {
                    k: K::Word(w),
                    span,
                    ..
                } => classes.push((w, span)),
                t => return Err(Diagnostic::new(t.span, "expected a style name after `.`")),
            }
            continue;
        }
        match p.item(Ctx::Text)? {
            Item::Attr(a) => attrs.push(a),
            Item::Flag(word, span) => attrs.push(p.flag(&word, span, "span")),
            other => {
                return Err(Diagnostic::new(
                    other.span(),
                    "only attributes can be written inside `{…}`",
                ));
            }
        }
    }
    Ok((attrs, classes))
}

// ---- parser -------------------------------------------------------------------

/// What a block belongs to; decides how strings and paths are read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ctx {
    Container,
    Text,
    Code,
    List,
    Image,
    Meta,
    Field,
    Option,
    Leaf,
    Component,
    Top,
}

fn ctx_of(canonical: &str) -> Ctx {
    match canonical {
        "page" | "column" | "row" | "grid" | "form" | "slot" => Ctx::Container,
        "text" | "heading" | "button" | "submit" | "item" => Ctx::Text,
        "codeblock" => Ctx::Code,
        "list" => Ctx::List,
        "image" => Ctx::Image,
        "meta" => Ctx::Meta,
        "input" | "textarea" | "checkbox" | "select" => Ctx::Field,
        "option" => Ctx::Option,
        "divider" | "spacer" => Ctx::Leaf,
        _ => Ctx::Component,
    }
}

/// One thing inside a block, before it is turned into the syntax tree.
enum Item {
    Attr(Attr),
    /// `$name"value"` in a component call.
    ParamArg(Attr),
    Flag(String, Span),
    Node(Node),
    /// A node written as `-"label"`: candidates for merging into one list.
    LabelList(Node, char),
    Str {
        raw: String,
        block: bool,
        span: Span,
    },
    Param(String, Span),
    Path(String, Span),
}

impl Item {
    fn span(&self) -> Span {
        match self {
            Item::Attr(a) | Item::ParamArg(a) => a.span,
            Item::Flag(_, s) | Item::Param(_, s) | Item::Path(_, s) => *s,
            Item::Str { span, .. } => *span,
            Item::Node(n) | Item::LabelList(n, _) => n.span,
        }
    }
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

/// Canonical names that are not compact names, so `text{…}` in 0.2 is an
/// unknown element instead of silently meaning the long-form element.
fn shadow(word: &str) -> String {
    let canonical = crate::analyze::element_names();
    if canonical.contains(&word) && names::element(word).is_none() {
        format!("\u{0}{word}")
    } else {
        word.to_string()
    }
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos]
    }

    fn next(&mut self) -> Tok {
        let t = self.toks[self.pos].clone();
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        t
    }

    fn unexpected(&self, expected: &str) -> Diagnostic {
        let t = self.peek();
        Diagnostic::new(
            t.span,
            format!("expected {expected}, found {}", t.k.describe()),
        )
    }

    fn document(&mut self) -> Result<Document, Diagnostic> {
        match &self.peek().k {
            K::Word(w) if w == "slml" => {
                self.next();
            }
            _ => return Err(self.unexpected("`slml 0.2`")),
        }
        let (version, version_span) = match self.peek().k.clone() {
            K::Num(_, t) => (t, self.next().span),
            _ => return Err(self.unexpected("a version number")),
        };
        let mut doc = Document {
            version,
            version_span,
            uses: Vec::new(),
            styles: Vec::new(),
            defines: Vec::new(),
            nodes: Vec::new(),
            comments: Vec::new(),
        };
        loop {
            match self.peek().k.clone() {
                K::Eof => break,
                K::Word(w) if w == "use" => {
                    self.next();
                    match self.peek().k.clone() {
                        K::Str { raw, block: false } => {
                            let t = self.next();
                            doc.uses.push((unescape(&raw, t.span)?, t.span));
                        }
                        _ => {
                            return Err(self
                                .unexpected("a file path after `use`")
                                .with_hint("e.g. `use\"/parts.slml\"`"));
                        }
                    }
                }
                K::At => {
                    self.next();
                    let (name, span) = match self.peek().k.clone() {
                        K::Word(w) => (w, self.next().span),
                        _ => return Err(self.unexpected("a name after `@`")),
                    };
                    if self.peek().k == K::LParen {
                        if names::element(&name).is_some()
                            || names::flag(&name).is_some()
                            || name == "on"
                        {
                            return Err(Diagnostic::new(
                                span,
                                format!("`{name}` is a built-in name in slml 0.2"),
                            )
                            .with_hint("choose another component name"));
                        }
                        doc.defines.push(self.define(name, span)?);
                    } else if self.peek().k == K::LBrace {
                        doc.styles.push(self.style(name, span)?);
                    } else {
                        return Err(self
                            .unexpected("`{` (a style) or `(` (a component)")
                            .with_hint("`@card{p20 bg#fff}` is a style, `@card(title){c{#$title slot}}` a component"));
                    }
                }
                _ => match self.item(Ctx::Top)? {
                    Item::Node(n) | Item::LabelList(n, _) => doc.nodes.push(n),
                    other => {
                        return Err(Diagnostic::new(
                            other.span(),
                            "only `meta`, `page`, `use`, `@` definitions and components belong at the top level",
                        ));
                    }
                },
            }
        }
        Ok(doc)
    }

    fn style(&mut self, name: String, span: Span) -> Result<StyleDef, Diagnostic> {
        let items = self.block(Ctx::Container)?;
        let mut attrs = Vec::new();
        for item in items {
            match item {
                Item::Attr(a) => attrs.push(a),
                Item::Flag(w, s) => attrs.push(self.flag(&w, s, "")),
                other => {
                    return Err(
                        Diagnostic::new(other.span(), "a style contains only attributes")
                            .with_hint(format!(
                                "for reusable content define a component: `@{name}(){{…}}`"
                            )),
                    );
                }
            }
        }
        Ok(StyleDef { name, span, attrs })
    }

    fn define(&mut self, name: String, span: Span) -> Result<Define, Diagnostic> {
        self.next(); // (
        let mut params = Vec::new();
        loop {
            let t = self.next();
            match t.k {
                K::RParen => break,
                K::Comma => continue,
                K::Word(pname) => {
                    // `name` alone is required; `name"x"`, `name#fff`,
                    // `name20`, `name:c` or `name/path` has a default.
                    let n = self.peek().clone();
                    let default = if n.glued {
                        match n.k {
                            K::Num(v, _) => Some(Value::Number(v)),
                            K::Color(c) => Some(Value::Color(c)),
                            K::Path(p) => Some(Value::Str(p)),
                            K::Str { raw, block } => Some(Value::Str(if block {
                                raw
                            } else {
                                unescape(&raw, n.span)?
                            })),
                            K::Colon => {
                                self.next();
                                match self.peek().k.clone() {
                                    K::Word(x) | K::Num(_, x) => Some(Value::Ident(x)),
                                    _ => return Err(self.unexpected("a value after `:`")),
                                }
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };
                    let value_span = if default.is_some() {
                        self.next().span
                    } else {
                        t.span
                    };
                    params.push(ParamDef {
                        name: pname,
                        span: t.span,
                        default: default.map(|d| (d, value_span)),
                    });
                }
                _ => {
                    return Err(Diagnostic::new(
                        t.span,
                        format!("expected a parameter name or `)`, found {}", t.k.describe()),
                    ));
                }
            }
        }
        if self.peek().k != K::LBrace {
            return Err(self.unexpected("`{` with the component's content"));
        }
        let items = self.block(Ctx::Container)?;
        let mut body = Vec::new();
        for item in items {
            if let Some(c) = self.child_of(item, Ctx::Container)? {
                body.push(c);
            }
        }
        Ok(Define {
            name,
            span,
            params,
            body,
        })
    }

    /// `{ items }`
    fn block(&mut self, ctx: Ctx) -> Result<Vec<Item>, Diagnostic> {
        let open = self.next();
        let mut items: Vec<Item> = Vec::new();
        loop {
            match self.peek().k {
                K::RBrace => {
                    self.next();
                    return Ok(items);
                }
                K::Eof => {
                    return Err(Diagnostic::new(open.span, "`{` is never closed")
                        .with_hint("add the matching `}`"));
                }
                _ => {
                    let item = self.item(ctx)?;
                    // `-"a" -"b"` written one after the other form one list.
                    if let Item::LabelList(node, marker) = &item
                        && let Some(Item::LabelList(prev, prev_marker)) = items.last_mut()
                        && prev_marker == marker
                    {
                        prev.children.extend(node.children.iter().cloned());
                        continue;
                    }
                    items.push(item);
                }
            }
        }
    }

    fn attr(&self, key: &str, key_span: Span, value: Value, value_span: Span) -> Attr {
        let canonical = names::attr(key).map(str::to_string).unwrap_or_else(|| {
            if crate::analyze::all_attr_names().contains(&key) {
                format!("\u{0}{key}")
            } else {
                key.to_string()
            }
        });
        Attr {
            name: canonical,
            written: Some(key.to_string()),
            span: key_span,
            value,
            value_span,
        }
    }

    /// A bare word used as a flag (`b`, `wrap`, `post`, `on` …).
    fn flag(&self, word: &str, span: Span, element: &str) -> Attr {
        let (name, value) = match (word, element) {
            ("on", "option") => ("selected".to_string(), "true".to_string()),
            ("on", _) => ("checked".to_string(), "true".to_string()),
            _ => match names::flag(word) {
                Some((a, v)) => (a.to_string(), v.to_string()),
                None => (format!("\u{0}{word}"), "true".to_string()),
            },
        };
        Attr {
            name,
            written: Some(word.to_string()),
            span,
            value: Value::Ident(value),
            value_span: span,
        }
    }

    /// Reads one item: an attribute, a flag, a string, or an element.
    fn item(&mut self, ctx: Ctx) -> Result<Item, Diagnostic> {
        let t = self.next();
        match t.k {
            K::Word(w) => {
                let n = self.peek().clone();
                if n.glued {
                    match n.k {
                        K::Num(v, _) => {
                            self.next();
                            return Ok(Item::Attr(self.attr(&w, t.span, Value::Number(v), n.span)));
                        }
                        K::Color(c) => {
                            self.next();
                            return Ok(Item::Attr(self.attr(&w, t.span, Value::Color(c), n.span)));
                        }
                        K::Colon => {
                            self.next();
                            let v = self.next();
                            let word = match v.k {
                                K::Word(x) => x,
                                K::Num(_, x) => x,
                                _ => {
                                    return Err(Diagnostic::new(
                                        v.span,
                                        format!("expected a value after `{w}:`"),
                                    ));
                                }
                            };
                            let attr = self.attr(&w, t.span, Value::Ident(word.clone()), v.span);
                            return Ok(Item::Attr(colon_value(attr, &w, &word)));
                        }
                        K::Path(p) => {
                            self.next();
                            return Ok(Item::Attr(self.attr(&w, t.span, Value::Str(p), n.span)));
                        }
                        K::Str { ref raw, block } if names::is_attr_key(&w) => {
                            self.next();
                            let s = if block {
                                raw.clone()
                            } else {
                                unescape(raw, n.span)?
                            };
                            return Ok(Item::Attr(self.attr(&w, t.span, Value::Str(s), n.span)));
                        }
                        K::Param(ref p) if names::is_attr_key(&w) => {
                            self.next();
                            return Ok(Item::Attr(self.attr(
                                &w,
                                t.span,
                                Value::Param(p.clone()),
                                n.span,
                            )));
                        }
                        _ => {}
                    }
                }
                let starts_element = matches!(self.peek().k, K::LBrace | K::Dot)
                    || (n.glued && matches!(n.k, K::Str { .. } | K::Param(_)));
                if starts_element {
                    let canonical = names::element(&w)
                        .map(str::to_string)
                        .unwrap_or_else(|| shadow(&w));
                    return self.element(canonical, w, t.span);
                }
                if w == "slot" {
                    return Ok(Item::Node(Node {
                        name: "slot".into(),
                        written: None,
                        span: t.span,
                        classes: Vec::new(),
                        attrs: Vec::new(),
                        children: Vec::new(),
                    }));
                }
                if ctx == Ctx::Top
                    || (ctx == Ctx::Container
                        && names::element(&w).is_some()
                        && names::flag(&w).is_none())
                {
                    // `page` or `c` without a block.
                    let canonical = names::element(&w)
                        .map(str::to_string)
                        .unwrap_or_else(|| shadow(&w));
                    return self.element(canonical, w, t.span);
                }
                Ok(Item::Flag(w, t.span))
            }
            K::Param(p) => {
                let n = self.peek().clone();
                if n.glued {
                    let value = match n.k {
                        K::Num(v, _) => Some(Value::Number(v)),
                        K::Color(c) => Some(Value::Color(c)),
                        K::Path(ref s) => Some(Value::Str(s.clone())),
                        K::Str { ref raw, block } => Some(Value::Str(if block {
                            raw.clone()
                        } else {
                            unescape(raw, n.span)?
                        })),
                        K::Colon => {
                            self.next();
                            let v = self.peek().clone();
                            match v.k {
                                K::Word(x) | K::Num(_, x) => Some(Value::Ident(x)),
                                _ => {
                                    return Err(Diagnostic::new(
                                        v.span,
                                        format!("expected a value after `${p}:`"),
                                    ));
                                }
                            }
                        }
                        _ => None,
                    };
                    if let Some(value) = value {
                        let v = self.next();
                        return Ok(Item::ParamArg(Attr {
                            name: p.clone(),
                            written: Some(format!("${p}")),
                            span: t.span,
                            value,
                            value_span: v.span,
                        }));
                    }
                }
                Ok(Item::Param(p, t.span))
            }
            K::Str { raw, block } => Ok(Item::Str {
                raw,
                block,
                span: t.span,
            }),
            K::Path(p) => Ok(Item::Path(p, t.span)),
            K::To(target) => {
                let value = match *target {
                    K::Path(p) => Value::Str(p),
                    K::Str { raw, block } => {
                        Value::Str(if block { raw } else { unescape(&raw, t.span)? })
                    }
                    K::Param(p) => Value::Param(p),
                    _ => unreachable!("lexer only builds paths, strings and params"),
                };
                Ok(Item::Attr(Attr {
                    name: "to".into(),
                    written: Some(">".into()),
                    span: t.span,
                    value,
                    value_span: t.span,
                }))
            }
            K::Heading(level) => {
                let item = self.element("heading".into(), "#".repeat(level as usize), t.span)?;
                Ok(match item {
                    Item::Node(mut n) => {
                        n.attrs.insert(
                            0,
                            Attr {
                                name: "level".into(),
                                written: Some("#".repeat(level as usize)),
                                span: t.span,
                                value: Value::Number(level as f64),
                                value_span: t.span,
                            },
                        );
                        Item::Node(n)
                    }
                    other => other,
                })
            }
            K::Marker(m) => {
                let label_form = !matches!(self.peek().k, K::LBrace);
                let item = self.element("list".into(), m.to_string(), t.span)?;
                let Item::Node(mut n) = item else {
                    return Ok(item);
                };
                let marker = match m {
                    '+' => "number",
                    '=' => "none",
                    _ => "bullet",
                };
                n.attrs.insert(
                    0,
                    Attr {
                        name: "marker".into(),
                        written: Some(m.to_string()),
                        span: t.span,
                        value: Value::Ident(marker.into()),
                        value_span: t.span,
                    },
                );
                Ok(if label_form {
                    Item::LabelList(n, m)
                } else {
                    Item::Node(n)
                })
            }
            K::Tilde => self.element("spacer".into(), "~".into(), t.span),
            K::Bar => self.element("divider".into(), "|".into(), t.span),
            other => Err(Diagnostic::new(
                t.span,
                format!("unexpected {}", other.describe()),
            )),
        }
    }

    /// An element after its name: optional `.styles`, then a label
    /// (`"…"` or `$param`) or a `{ block }`.
    fn element(
        &mut self,
        canonical: String,
        written: String,
        span: Span,
    ) -> Result<Item, Diagnostic> {
        let mut classes = Vec::new();
        while self.peek().k == K::Dot {
            self.next();
            match self.peek().k.clone() {
                K::Word(w) => classes.push((w, self.next().span)),
                _ => return Err(self.unexpected("a style name after `.`")),
            }
        }
        let ctx = ctx_of(&canonical);
        // `~` and `|` never take text: a string after them is a sibling.
        let leaf = ctx == Ctx::Leaf;
        let items = match self.peek().k.clone() {
            K::Str { .. } | K::Param(_) if leaf => Vec::new(),
            K::Str { raw, block } => {
                let s = self.next().span;
                vec![Item::Str {
                    raw,
                    block,
                    span: s,
                }]
            }
            K::Param(p) => {
                let s = self.next().span;
                vec![Item::Param(p, s)]
            }
            K::LBrace => self.block(ctx)?,
            _ => Vec::new(),
        };
        let mut node = Node {
            written: (written != canonical).then_some(written),
            name: canonical,
            span,
            classes,
            attrs: Vec::new(),
            children: Vec::new(),
        };
        for item in items {
            self.place(&mut node, item, ctx)?;
        }
        if node.name == "option" {
            for a in &mut node.attrs {
                if a.name == "checked" {
                    a.name = "selected".into();
                }
            }
        }
        Ok(Item::Node(node))
    }

    /// Puts one block item into `node` according to the element's kind.
    fn place(&mut self, node: &mut Node, item: Item, ctx: Ctx) -> Result<(), Diagnostic> {
        let shown = node.shown().trim_start_matches('\u{0}').to_string();
        match item {
            Item::Attr(a) => node.attrs.push(a),
            Item::Flag(w, s) => {
                let a = self.flag(&w, s, &node.name);
                node.attrs.push(a);
            }
            Item::ParamArg(a) => {
                if ctx != Ctx::Component {
                    return Err(Diagnostic::new(
                        a.span,
                        format!(
                            "`{}` passes a component parameter, but `{shown}` is not a component",
                            a.shown()
                        ),
                    ));
                }
                node.attrs.push(a);
            }
            Item::Path(p, s) => match ctx {
                Ctx::Image => node.attrs.push(Attr {
                    name: "src".into(),
                    written: Some("path".into()),
                    span: s,
                    value: Value::Str(p),
                    value_span: s,
                }),
                _ => {
                    return Err(Diagnostic::new(
                        s,
                        format!("a path cannot be placed in `{shown}`"),
                    )
                    .with_hint("links are written `>/path`, images `img{/path \"description\"}`"));
                }
            },
            Item::Str { raw, block, span } => match ctx {
                Ctx::Text | Ctx::Code => {
                    if block || ctx == Ctx::Code {
                        let s = if block { raw } else { unescape(&raw, span)? };
                        node.children.push(Child::Text(s, span));
                    } else {
                        node.children.extend(inline(&raw, span)?);
                    }
                }
                Ctx::Container | Ctx::Component | Ctx::Top => {
                    node.children
                        .push(Child::Node(text_node("text", "\"…\"", &raw, block, span)?));
                }
                Ctx::List => {
                    node.children
                        .push(Child::Node(text_node("item", "\"…\"", &raw, block, span)?));
                }
                Ctx::Image | Ctx::Meta | Ctx::Field => {
                    let name = match ctx {
                        Ctx::Image => "alt",
                        Ctx::Meta => "title",
                        _ => "label",
                    };
                    let s = if block { raw } else { unescape(&raw, span)? };
                    node.attrs.push(Attr {
                        name: name.into(),
                        written: Some("\"…\"".into()),
                        span,
                        value: Value::Str(s),
                        value_span: span,
                    });
                }
                Ctx::Option => {
                    let s = if block { raw } else { unescape(&raw, span)? };
                    node.children.push(Child::Text(s, span));
                }
                Ctx::Leaf => {
                    return Err(Diagnostic::new(span, format!("`{shown}` cannot have text")));
                }
            },
            Item::Param(p, s) => match ctx {
                Ctx::Text | Ctx::Code | Ctx::Option => node.children.push(Child::Param(p, s)),
                Ctx::Container | Ctx::Component | Ctx::Top | Ctx::List => {
                    let name = if ctx == Ctx::List { "item" } else { "text" };
                    node.children.push(Child::Node(Node {
                        name: name.into(),
                        written: Some(format!("${p}")),
                        span: s,
                        classes: Vec::new(),
                        attrs: Vec::new(),
                        children: vec![Child::Param(p, s)],
                    }));
                }
                Ctx::Image | Ctx::Meta | Ctx::Field => {
                    let name = match ctx {
                        Ctx::Image => "alt",
                        Ctx::Meta => "title",
                        _ => "label",
                    };
                    node.attrs.push(Attr {
                        name: name.into(),
                        written: Some(format!("${p}")),
                        span: s,
                        value: Value::Param(p),
                        value_span: s,
                    });
                }
                Ctx::Leaf => {
                    return Err(Diagnostic::new(s, format!("`{shown}` cannot have text")));
                }
            },
            Item::Node(n) | Item::LabelList(n, _) => node.children.push(Child::Node(n)),
        }
        Ok(())
    }

    fn child_of(&mut self, item: Item, ctx: Ctx) -> Result<Option<Child>, Diagnostic> {
        let mut holder = Node {
            name: "slot".into(),
            written: None,
            span: item.span(),
            classes: Vec::new(),
            attrs: Vec::new(),
            children: Vec::new(),
        };
        if let Item::Attr(a) | Item::ParamArg(a) = &item {
            return Err(Diagnostic::new(
                a.span,
                "a component's content cannot start with an attribute",
            )
            .with_hint("put attributes on the element inside, e.g. `@card(){c{p20 slot}}`"));
        }
        if let Item::Flag(w, s) = &item {
            return Err(Diagnostic::new(*s, format!("`{w}` is not an element"))
                .with_hint("elements are followed by `{…}` or a label, e.g. `t\"text\"`"));
        }
        self.place(&mut holder, item, ctx)?;
        Ok(holder.children.pop())
    }
}

/// `w:f` → `fill`, `b:0` → `false`, `n:email` → the string "email".
fn colon_value(mut attr: Attr, key: &str, word: &str) -> Attr {
    if let Some((_, long)) = names::BOOL_KEYS.iter().find(|(k, _)| *k == key)
        && (word == "0" || word == "1")
    {
        attr.name = long.to_string();
        attr.value = Value::Ident(if word == "1" { "true" } else { "false" }.into());
        return attr;
    }
    if let Some(v) = names::enum_value(&attr.name, word) {
        attr.value = Value::Ident(v.into());
        return attr;
    }
    if matches!(
        attr.name.as_str(),
        "name" | "id" | "lang" | "description" | "placeholder" | "value" | "src" | "alt" | "icon"
    ) {
        attr.value = Value::Str(word.to_string());
    }
    attr
}

/// A paragraph (or list item) made from a bare string.
fn text_node(
    name: &str,
    written: &str,
    raw: &str,
    block: bool,
    span: Span,
) -> Result<Node, Diagnostic> {
    let children = if block {
        vec![Child::Text(raw.to_string(), span)]
    } else {
        inline(raw, span)?
    };
    Ok(Node {
        name: name.into(),
        written: Some(written.into()),
        span,
        classes: Vec::new(),
        attrs: Vec::new(),
        children,
    })
}

#[cfg(test)]
mod tests {
    use crate::model::*;

    fn compile(src: &str) -> Document {
        crate::compile(src).unwrap_or_else(|e| panic!("{e:#?}"))
    }

    fn errors(src: &str) -> Vec<String> {
        crate::compile(src)
            .unwrap_err()
            .into_iter()
            .map(|d| d.message)
            .collect()
    }

    #[test]
    fn detects_the_header() {
        assert!(super::is_compact("slml 0.2 page{}"));
        assert!(super::is_compact("// hi\n/* x */ slml   0.2\npage{}"));
        assert!(!super::is_compact("slml 0.1 page {}"));
        assert!(!super::is_compact("slml 0.21 page{}"));
    }

    #[test]
    fn the_preview_page_compiles() {
        let doc = compile(
            r#"slml 0.2
            @card{p20 bg#fff r12 g6}

            page{bg#f6f6f9 a:c
             c{w:f mw760 g24
              r{a:c g12
               img{/logo.png w32 >/index.slml "Logo"}
               ~
               t"[O nas](/about.slml)"}
              #"Strony bez zgadywania"
              t"Zwykły tekst, *pogrubienie* i [link](/kontakt.slml)."
              grid{cw220 g14
               c.card{###"Szybko" t"Opis"}}
              -"Pierwszy" -"Drugi"}}"#,
        );
        assert_eq!(doc.version, "0.2");
        assert_eq!(doc.page.flow.align, Align::Center);
        let Node::Container(col) = &doc.page.children[0] else {
            panic!()
        };
        assert_eq!(col.frame.width, Size::Fill);
        assert_eq!(col.frame.max_width, Some(760.0));
        let Node::Container(row) = &col.children[0] else {
            panic!()
        };
        let Node::Image(img) = &row.children[0] else {
            panic!()
        };
        assert_eq!((img.src.as_str(), img.alt.as_str()), ("/logo.png", "Logo"));
        assert_eq!(img.frame.link.as_deref(), Some("/index.slml"));
        let Node::Text(h) = &col.children[1] else {
            panic!()
        };
        assert_eq!(h.kind, TextKind::Heading(1));
        let Node::Text(p) = &col.children[2] else {
            panic!()
        };
        assert_eq!(p.content.len(), 5, "{:?}", p.content);
        let Node::List(list) = &col.children[4] else {
            panic!()
        };
        assert_eq!(list.items.len(), 2, "adjacent -\"…\" form one list");
    }

    #[test]
    fn bare_strings_are_paragraphs_and_lists_nest() {
        let doc = compile(r#"slml 0.2 page{"Hello" +{g8 "one" "two" -{"a" "b"}}}"#);
        assert!(matches!(&doc.page.children[0], Node::Text(t) if t.kind == TextKind::Paragraph));
        let Node::List(l) = &doc.page.children[1] else {
            panic!()
        };
        assert_eq!(l.marker, Marker::Number);
        assert_eq!(l.gap, 8.0);
        assert!(matches!(l.items[2], ListEntry::List(_)));
    }

    #[test]
    fn inline_markup() {
        let doc =
            compile(r#"slml 0.2 page{t"a *b _c_* `d*e` [f](/x.slml){c#f00} [g]{b s20} \*h\*"}"#);
        let Node::Text(t) = &doc.page.children[0] else {
            panic!()
        };
        let kinds: Vec<String> = t
            .content
            .iter()
            .map(|i| match i {
                Inline::Text(s) => format!("text:{s}"),
                Inline::Span(s) => format!("span:{:?}", s.link),
            })
            .collect();
        assert_eq!(kinds.len(), 9, "{kinds:?}");
        assert_eq!(kinds.last().unwrap(), "text: *h*");
        let Inline::Span(link) = &t.content[5] else {
            panic!()
        };
        assert_eq!(link.text.color, Some(Color(0xff0000ff)));
    }

    #[test]
    fn components_styles_and_params() {
        let doc = compile(
            r#"slml 0.2
            @card{p20 r12}
            @tile(title, tone#ffffff){c.card{bg$tone #$title slot}}
            page{tile{$title"Fast" $tone#eeeeee "Body"}}"#,
        );
        let Node::Container(c) = &doc.page.children[0] else {
            panic!()
        };
        assert_eq!(c.frame.fill, Some(Color(0xeeeeeeff)));
        assert_eq!(c.frame.padding_x, 20.0);
        assert_eq!(c.children.len(), 2);
    }

    #[test]
    fn params_inside_text_and_link_styles() {
        let doc = compile(
            r#"slml 0.2
            @nav{c#2b2b38 u:0}
            @navitem(to, label){t{s15 "[$label]($to){.nav} and ${label}s"}}
            page{navitem{$to"/a.slml" $label"Home"}}"#,
        );
        let Node::Text(t) = &doc.page.children[0] else {
            panic!()
        };
        let Inline::Span(link) = &t.content[0] else {
            panic!()
        };
        assert_eq!(link.link.as_deref(), Some("/a.slml"));
        assert_eq!(link.text.underline, Some(false));
        assert_eq!(t.content[1], Inline::Text(" and Homes".into()));
    }

    #[test]
    fn components_cannot_shadow_elements() {
        assert!(
            crate::parse("slml 0.2 @c(){t\"x\"} page{}")
                .unwrap_err()
                .message
                .contains("built-in")
        );
    }

    #[test]
    fn enum_letters_through_parameters() {
        let doc = compile("slml 0.2 @bar(align){r{a$align}} page{bar{$align:c}}");
        let Node::Container(r) = &doc.page.children[0] else {
            panic!()
        };
        assert_eq!(r.flow.align, Align::Center);
    }

    #[test]
    fn forms() {
        let doc = compile(
            r#"slml 0.2 page{form{>https://example.com/x post
              in{n:email "E-mail" k:email req ph"you@example.com"}
              area{n:msg "Message" rows5}
              check{n:news "News" on}
              sel{n:size "Size" opt"S" opt{v"m" on "M"}}
              send"Send"}}"#,
        );
        let Node::Container(f) = &doc.page.children[0] else {
            panic!()
        };
        assert!(matches!(
            f.kind,
            ContainerKind::Form(FormSpec {
                method: Method::Post,
                ..
            })
        ));
        assert_eq!(f.children.len(), 5);
    }

    #[test]
    fn errors_use_compact_names() {
        let e = errors(r#"slml 0.2 page{c{gap4} t{p2 "x"} img{/a.png} text"x"}"#);
        assert!(e.iter().any(|m| m == "`c` has no attribute `gap`"), "{e:?}");
        assert!(e.iter().any(|m| m == "`img` requires `alt`"), "{e:?}");
        assert!(e.iter().any(|m| m == "unknown element `text`"), "{e:?}");
        let e = errors(r#"slml 0.2 page{t"*open"}"#);
        assert!(e[0].contains("never closed"), "{e:?}");
        let e = errors(r#"slml 0.2 page{t"a [b] c"}"#);
        assert!(e[0].contains("starts a link"), "{e:?}");
    }

    #[test]
    fn spans_point_inside_strings() {
        let err = crate::compile("slml 0.2\npage{t\"ok *open\"}").unwrap_err();
        assert_eq!((err[0].span.line, err[0].span.col), (2, 11));
    }
}
