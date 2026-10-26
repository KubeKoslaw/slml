//! Prints a syntax tree in the compact `slml 0.2` dialect.
//!
//! Used by `slml upgrade`: a `slml 0.1` file is parsed with the long-form
//! parser and printed here, and a `slml 0.2` file is printed in canonical
//! formatting. Printing never changes meaning: the compact output compiles
//! to the same document (tests check this on every example and handbook
//! page). Comments are kept and placed before the element that followed
//! them.

use crate::ast::*;
use crate::diagnostic::{Diagnostic, Span};
use crate::names;

/// Prints `doc` as `slml 0.2`. Fails only for the rare constructs that have
/// no compact spelling, naming each one.
pub fn to_compact(doc: &Document) -> Result<String, Vec<Diagnostic>> {
    let mut p = Printer {
        out: String::new(),
        comments: doc.comments.clone(),
        next_comment: 0,
        errors: Vec::new(),
    };
    p.comments.sort_by_key(|(s, _)| s.start);
    p.out.push_str("slml 0.2\n");

    let mut wrote_section = false;
    for (path, span) in &doc.uses {
        p.flush_comments(span.start, 0);
        p.out.push_str(&format!("use{}\n", quote(path)));
        wrote_section = true;
    }
    let blank = |p: &mut Printer, wrote: &mut bool| {
        if *wrote {
            p.out.push('\n');
        }
        *wrote = true;
    };
    if !doc.styles.is_empty() {
        blank(&mut p, &mut wrote_section);
    }
    for style in &doc.styles {
        p.flush_comments(style.span.start, 0);
        let attrs = p.attrs(&style.attrs, "style");
        p.out
            .push_str(&format!("@{}{{{}}}\n", style.name, attrs.join(" ")));
    }
    for def in &doc.defines {
        blank(&mut p, &mut wrote_section);
        p.flush_comments(def.span.start, 0);
        let params: Vec<String> = def
            .params
            .iter()
            .map(|param| match &param.default {
                None => param.name.clone(),
                Some((v, _)) => format!("{}{}", param.name, p.value_glued(v, "", param.span)),
            })
            .collect();
        p.out
            .push_str(&format!("@{}({})", def.name, params.join(", ")));
        p.block(&[], &def.body, Ctx::Container, 0, None);
        p.out.push('\n');
    }
    for node in &doc.nodes {
        blank(&mut p, &mut wrote_section);
        p.flush_comments(node.span.start, 0);
        p.node(node, Ctx::Container, 0);
        p.out.push('\n');
    }
    p.flush_comments(usize::MAX, 0);
    if p.errors.is_empty() {
        Ok(p.out)
    } else {
        Err(p.errors)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ctx {
    Container,
    List,
}

struct Printer {
    out: String,
    comments: Vec<(Span, String)>,
    next_comment: usize,
    errors: Vec<Diagnostic>,
}

const INDENT: &str = "  ";

/// Characters that may appear in an unquoted path or `>` target.
fn path_safe(s: &str) -> bool {
    !s.is_empty()
        && !s
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '{' | '}' | '"'))
}

/// Words that can follow `key:` unquoted.
fn word_safe(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

fn color(c: u32) -> String {
    let [r, g, b, a] = c.to_be_bytes();
    let short = |x: u8| x >> 4 == x & 0xf;
    if a == 0xff {
        if short(r) && short(g) && short(b) {
            format!("#{:x}{:x}{:x}", r & 0xf, g & 0xf, b & 0xf)
        } else {
            format!("#{r:02x}{g:02x}{b:02x}")
        }
    } else if short(r) && short(g) && short(b) && short(a) {
        format!("#{:x}{:x}{:x}{:x}", r & 0xf, g & 0xf, b & 0xf, a & 0xf)
    } else {
        format!("#{c:08x}")
    }
}

/// Escapes text for an inline-markup string.
fn escape_text(s: &str, next_is_word: bool) -> String {
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '*' | '_' | '`' | '[' | ']' => {
                out.push('\\');
                out.push(*c);
            }
            '$' => {
                let next = chars.get(i + 1).copied();
                let starts_param = match next {
                    Some(n) => n.is_ascii_alphabetic() || n == '_' || n == '{',
                    None => next_is_word,
                };
                if starts_param {
                    out.push_str("\\$");
                } else {
                    out.push('$');
                }
            }
            c => out.push(*c),
        }
    }
    out
}

impl Printer {
    fn fail(&mut self, span: Span, msg: impl Into<String>) {
        self.errors.push(
            Diagnostic::new(span, msg).with_hint(
                "rewrite this part by hand in `slml 0.2`, then run `slml upgrade` again",
            ),
        );
    }

    fn indent(&mut self, depth: usize) {
        for _ in 0..depth {
            self.out.push_str(INDENT);
        }
    }

    fn flush_comments(&mut self, before: usize, depth: usize) {
        while let Some((span, text)) = self.comments.get(self.next_comment).cloned() {
            if span.file != 0 || span.start >= before {
                break;
            }
            self.next_comment += 1;
            for line in text.lines() {
                self.indent(depth);
                self.out.push_str(line.trim_start());
                self.out.push('\n');
            }
        }
    }

    /// A value written right after its key (`20`, `#fff`, `"x"`, `:c`, `$p`).
    fn value_glued(&mut self, v: &Value, canonical: &str, _span: Span) -> String {
        match v {
            Value::Number(n) => number(*n),
            Value::Color(c) => color(*c),
            Value::Str(s) if matches!(canonical, "name" | "id" | "lang") && word_safe(s) => {
                format!(":{s}")
            }
            Value::Str(s) => quote(s),
            Value::Ident(i) => {
                let short = names::ENUMS
                    .iter()
                    .find(|(a, _, long)| *a == canonical && long == i)
                    .map(|(_, letter, _)| *letter);
                format!(":{}", short.unwrap_or(i))
            }
            Value::Param(p) => format!("${p}"),
        }
    }

    /// One attribute in compact form, or `None` when it is printed
    /// elsewhere (`level`, `marker`, positional `src`, `alt`, `label` …).
    fn attr(&mut self, a: &Attr, element: &str) -> Option<String> {
        let name = a.name.trim_start_matches('\u{0}');
        let flag = |on: &str, off: &str, v: &Value| -> Option<String> {
            match v {
                Value::Ident(i) if i == "true" => Some(on.to_string()),
                Value::Ident(i) if i == "false" => Some(off.to_string()),
                _ => None,
            }
        };
        let printed = match name {
            "level" | "marker" => return None,
            "bold" => flag("b", "b:0", &a.value),
            "italic" => flag("i", "i:0", &a.value),
            "underline" => flag("u", "u:0", &a.value),
            "wrap" => flag("wrap", "wrap:0", &a.value),
            "required" => flag("req", "req:0", &a.value),
            "checked" | "selected" => flag("on", "on:0", &a.value),
            "font" => match &a.value {
                Value::Ident(f) if f == "mono" || f == "sans" => Some(f.clone()),
                _ => None,
            },
            "method" => match &a.value {
                Value::Ident(m) if m == "post" || m == "get" => Some(m.clone()),
                Value::Param(p) => Some(format!("method${p}")),
                _ => None,
            },
            "to" => match &a.value {
                Value::Str(s) if path_safe(s) => Some(format!(">{s}")),
                Value::Str(s) => Some(format!(">{}", quote(s))),
                Value::Param(p) => Some(format!(">${p}")),
                _ => None,
            },
            "icon" | "src" => match &a.value {
                Value::Str(s) if path_safe(s) && (s.starts_with('/') || s.contains("://")) => {
                    Some(format!("{name}{s}"))
                }
                Value::Str(s) => Some(format!("{name}{}", quote(s))),
                Value::Param(p) => Some(format!("{name}${p}")),
                _ => None,
            },
            other => {
                let Some(key) = names::ATTRS
                    .iter()
                    .find(|(_, l)| *l == other)
                    .map(|(k, _)| *k)
                else {
                    // Component parameters and unknown attributes keep their name.
                    if element_is_component(element) {
                        let v = self.value_glued(&a.value, other, a.span);
                        return Some(format!("${other}{v}"));
                    }
                    self.fail(
                        a.span,
                        format!("attribute `{other}` has no compact spelling"),
                    );
                    return None;
                };
                let v = self.value_glued(&a.value, other, a.span);
                Some(format!("{key}{v}"))
            }
        };
        if printed.is_none() {
            self.fail(
                a.span,
                format!("cannot write `{name}` with this value in `slml 0.2`"),
            );
        }
        printed
    }

    fn attrs(&mut self, attrs: &[Attr], element: &str) -> Vec<String> {
        attrs.iter().filter_map(|a| self.attr(a, element)).collect()
    }

    fn classes(n: &Node) -> String {
        n.classes.iter().map(|(c, _)| format!(".{c}")).collect()
    }

    fn get<'a>(n: &'a Node, name: &str) -> Option<&'a Value> {
        n.attrs.iter().find(|a| a.name == name).map(|a| &a.value)
    }

    /// Prints a node at the current position (the caller indented already).
    fn node(&mut self, n: &Node, ctx: Ctx, depth: usize) {
        let name = n.name.trim_start_matches('\u{0}');
        match name {
            "slot" => self.out.push_str("slot"),
            "heading" => {
                let level = match Self::get(n, "level") {
                    Some(Value::Number(l)) => *l as usize,
                    _ => 1,
                };
                let head = format!("{}{}", "#".repeat(level.clamp(1, 4)), Self::classes(n));
                self.text_element(&head, n, depth);
            }
            "text" if n.attrs.is_empty() && n.classes.is_empty() && ctx == Ctx::Container => {
                let s = self.markup(&n.children, n.span);
                self.out.push_str(&s);
            }
            "text" | "button" | "submit" | "codeblock" => {
                let head = format!("{}{}", names::element_compact(name), Self::classes(n));
                self.text_element(&head, n, depth);
            }
            "list" => self.list(n, depth),
            "divider" | "spacer" => {
                let head = names::element_compact(name).to_string();
                let attrs = self.attrs(&n.attrs, name);
                self.out.push_str(&head);
                if !attrs.is_empty() {
                    self.out.push_str(&format!("{{{}}}", attrs.join(" ")));
                }
            }
            "image" => {
                let mut parts = Vec::new();
                match Self::get(n, "src") {
                    Some(Value::Str(s))
                        if path_safe(s) && (s.starts_with('/') || s.contains("://")) =>
                    {
                        parts.push(s.clone())
                    }
                    Some(Value::Str(s)) if path_safe(s) && !s.starts_with('.') => {
                        parts.push(format!("./{s}"))
                    }
                    Some(Value::Str(s)) => parts.push(format!("src{}", quote(s))),
                    Some(Value::Param(p)) => parts.push(format!("src${p}")),
                    _ => {}
                }
                let rest: Vec<Attr> = n
                    .attrs
                    .iter()
                    .filter(|a| a.name != "src" && a.name != "alt")
                    .cloned()
                    .collect();
                parts.extend(self.attrs(&rest, name));
                match Self::get(n, "alt") {
                    Some(Value::Str(s)) => parts.push(quote(s)),
                    Some(Value::Param(p)) => parts.push(format!("${p}")),
                    _ => {}
                }
                self.out
                    .push_str(&format!("img{}{{{}}}", Self::classes(n), parts.join(" ")));
            }
            "meta" => {
                let mut parts = Vec::new();
                match Self::get(n, "title") {
                    Some(Value::Str(s)) => parts.push(quote(s)),
                    Some(Value::Param(p)) => parts.push(format!("${p}")),
                    _ => {}
                }
                let rest: Vec<Attr> = n
                    .attrs
                    .iter()
                    .filter(|a| a.name != "title")
                    .cloned()
                    .collect();
                parts.extend(self.attrs(&rest, name));
                self.out.push_str(&format!("meta{{{}}}", parts.join(" ")));
            }
            "input" | "textarea" | "checkbox" | "select" | "option" => {
                let mut parts = Vec::new();
                let positional = if name == "option" {
                    None
                } else {
                    Some("label")
                };
                let rest: Vec<Attr> = n
                    .attrs
                    .iter()
                    .filter(|a| Some(a.name.as_str()) != positional)
                    .cloned()
                    .collect();
                // `n:` first, then the label, then everything else.
                let (first, others): (Vec<Attr>, Vec<Attr>) =
                    rest.into_iter().partition(|a| a.name == "name");
                parts.extend(self.attrs(&first, name));
                if let Some(label) = positional.and_then(|l| Self::get(n, l)) {
                    match label {
                        Value::Str(s) => parts.push(quote(s)),
                        Value::Param(p) => parts.push(format!("${p}")),
                        _ => {}
                    }
                }
                parts.extend(self.attrs(&others, name));
                let head = format!("{}{}", names::element_compact(name), Self::classes(n));
                if name == "option" {
                    let label = self.plain_children(&n.children, n.span);
                    if parts.is_empty() {
                        self.out.push_str(&format!("{head}{label}"));
                    } else {
                        self.out
                            .push_str(&format!("{head}{{{} {label}}}", parts.join(" ")));
                    }
                    return;
                }
                let children: Vec<Child> = n.children.clone();
                if children.is_empty() {
                    self.out.push_str(&format!("{head}{{{}}}", parts.join(" ")));
                } else {
                    // select: options inline
                    self.out.push_str(&format!("{head}{{{}", parts.join(" ")));
                    for child in &children {
                        self.out.push(' ');
                        if let Child::Node(o) = child {
                            self.node(o, Ctx::Container, depth);
                        }
                    }
                    self.out.push('}');
                }
            }
            _ => {
                // Containers, page, form and component calls.
                let component = element_is_component(name);
                let head = if component {
                    format!("{name}{}", Self::classes(n))
                } else {
                    format!("{}{}", names::element_compact(name), Self::classes(n))
                };
                self.out.push_str(&head);
                self.block(&n.attrs, &n.children, Ctx::Container, depth, Some(name));
            }
        }
        let _ = ctx;
    }

    /// `{attrs children}` with children on their own lines.
    fn block(
        &mut self,
        attrs: &[Attr],
        children: &[Child],
        ctx: Ctx,
        depth: usize,
        element: Option<&str>,
    ) {
        let element = element.unwrap_or("define");
        let attrs = self.attrs(attrs, element);
        let comment_inside = |me: &Self, n: &Node| {
            me.comments
                .get(me.next_comment)
                .is_some_and(|(s, _)| s.file == 0 && s.start < n.span.start)
        };
        let simple = children.is_empty()
            || (children.len() == 1
                && matches!(&children[0], Child::Node(n) if is_short(n) && !comment_inside(self, n)));
        if simple {
            self.out.push('{');
            self.out.push_str(&attrs.join(" "));
            if let [Child::Node(n)] = children {
                if !attrs.is_empty() {
                    self.out.push(' ');
                }
                self.node(n, ctx, depth);
            }
            self.out.push('}');
            return;
        }
        self.out.push('{');
        self.out.push_str(&attrs.join(" "));
        self.out.push('\n');
        let siblings: Vec<&Child> = children.iter().collect();
        for (i, child) in siblings.iter().enumerate() {
            let start = match child {
                Child::Node(n) => n.span.start,
                Child::Text(_, s) | Child::Param(_, s) => s.start,
            };
            self.flush_comments(start, depth + 1);
            self.indent(depth + 1);
            match child {
                Child::Node(n) => {
                    // Two one-item lists written `-"a" -"b"` would merge;
                    // keep them apart with the block form.
                    let next_merges = n.name == "list"
                        && siblings.get(i + 1).is_some_and(|next| matches!(next, Child::Node(m) if m.name == "list" && marker_of(m) == marker_of(n)));
                    if next_merges {
                        self.list_block(n, depth + 1);
                    } else {
                        self.node(n, ctx, depth + 1);
                    }
                }
                Child::Text(t, _) => {
                    let s = format!("t{}", quote(t));
                    self.out.push_str(&s);
                }
                Child::Param(p, _) => self.out.push_str(&format!("t${p}")),
            }
            self.out.push('\n');
        }
        self.indent(depth);
        self.out.push('}');
    }

    /// `t"…"`, `#"…"`, `btn{>x "…"}`, `code"""…"""`.
    fn text_element(&mut self, head: &str, n: &Node, depth: usize) {
        let attrs = self.attrs(&n.attrs, &n.name);
        let body = if n.name == "codeblock" {
            self.code_body(&n.children, n.span, depth)
        } else {
            self.markup(&n.children, n.span)
        };
        if attrs.is_empty() {
            self.out.push_str(&format!("{head}{body}"));
        } else {
            self.out
                .push_str(&format!("{head}{{{} {body}}}", attrs.join(" ")));
        }
    }

    fn code_body(&mut self, children: &[Child], span: Span, depth: usize) -> String {
        if let [Child::Param(p, _)] = children {
            return format!("${p}");
        }
        let text = self.plain_text(children, span);
        if text.contains('\n') && !text.contains("\"\"\"") {
            let pad = INDENT.repeat(depth + 2);
            let mut s = String::from("\"\"\"\n");
            for line in text.split('\n') {
                if !line.is_empty() {
                    s.push_str(&pad);
                    s.push_str(line);
                }
                s.push('\n');
            }
            s.push_str(&pad);
            s.push_str("\"\"\"");
            s
        } else {
            quote(&text)
        }
    }

    fn plain_text(&mut self, children: &[Child], span: Span) -> String {
        let mut out = String::new();
        for c in children {
            match c {
                Child::Text(t, _) => out.push_str(t),
                Child::Param(_, s) | Child::Node(Node { span: s, .. }) => {
                    self.fail(*s, "a code block can only contain plain text in `slml 0.2`");
                }
            }
        }
        let _ = span;
        out
    }

    fn plain_children(&mut self, children: &[Child], span: Span) -> String {
        match children {
            [Child::Param(p, _)] => format!("${p}"),
            _ => {
                let t = self.plain_text(children, span);
                quote(&t)
            }
        }
    }

    /// Inline content as a markup string: `"a *b* [c](d)"`, or `$p`.
    fn markup(&mut self, children: &[Child], span: Span) -> String {
        if let [Child::Param(p, _)] = children {
            return format!("${p}");
        }
        let inner = self.inline(children, false, false);
        let _ = span;
        format!("\"{inner}\"")
    }

    fn inline(&mut self, children: &[Child], in_bold: bool, in_italic: bool) -> String {
        let mut out = String::new();
        for (i, child) in children.iter().enumerate() {
            let next_is_word = match children.get(i + 1) {
                Some(Child::Text(t, _)) => {
                    t.starts_with(|c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                }
                _ => false,
            };
            match child {
                Child::Text(t, _) => out.push_str(&escape_text(t, next_is_word)),
                Child::Param(p, _) => {
                    if next_is_word {
                        out.push_str(&format!("${{{p}}}"));
                    } else {
                        out.push_str(&format!("${p}"));
                    }
                }
                Child::Node(n) => {
                    let name = n.name.trim_start_matches('\u{0}');
                    let plain = n.attrs.is_empty() && n.classes.is_empty();
                    match name {
                        "bold" if plain && in_bold => {
                            out.push_str(&self.inline(&n.children, true, in_italic))
                        }
                        "italic" if plain && in_italic => {
                            out.push_str(&self.inline(&n.children, in_bold, true))
                        }
                        "bold" if plain => {
                            let s = self.inline(&n.children, true, in_italic);
                            out.push_str(&format!("*{s}*"));
                        }
                        "italic" if plain => {
                            let s = self.inline(&n.children, in_bold, true);
                            out.push_str(&format!("_{s}_"));
                        }
                        "code"
                            if plain && n.children.iter().all(|c| matches!(c, Child::Text(..))) =>
                        {
                            let t = self.plain_text(&n.children, n.span);
                            let escaped: String = t
                                .chars()
                                .flat_map(|c| match c {
                                    '`' => vec!['\\', '`'],
                                    '\\' => vec!['\\', '\\'],
                                    '"' => vec!['\\', '"'],
                                    '\n' => vec!['\\', 'n'],
                                    c => vec![c],
                                })
                                .collect();
                            out.push_str(&format!("`{escaped}`"));
                        }
                        _ => {
                            // link / span, or styled bold/italic/code.
                            let extra_flag = match name {
                                "bold" => Some("b"),
                                "italic" => Some("i"),
                                "code" => Some("mono"),
                                _ => None,
                            };
                            let inner = self.inline(
                                &n.children,
                                in_bold || name == "bold",
                                in_italic || name == "italic",
                            );
                            let target = n.attrs.iter().find(|a| a.name == "to");
                            let rest: Vec<Attr> =
                                n.attrs.iter().filter(|a| a.name != "to").cloned().collect();
                            let mut attrs: Vec<String> =
                                n.classes.iter().map(|(c, _)| format!(".{c}")).collect();
                            attrs.extend(extra_flag.map(str::to_string));
                            attrs.extend(self.attrs(&rest, name));
                            out.push_str(&format!("[{inner}]"));
                            if name == "link" {
                                match target.map(|a| &a.value) {
                                    Some(Value::Str(s)) => {
                                        let t = s.replace(')', "\\)").replace('"', "\\\"");
                                        out.push_str(&format!("({t})"));
                                    }
                                    Some(Value::Param(p)) => out.push_str(&format!("(${p})")),
                                    _ => {
                                        self.fail(n.span, "a link needs a target");
                                    }
                                }
                            }
                            if name != "link" || !attrs.is_empty() {
                                out.push_str(&format!("{{{}}}", attrs.join(" ")));
                            }
                        }
                    }
                }
            }
        }
        out
    }

    fn list(&mut self, n: &Node, depth: usize) {
        let marker = marker_of(n);
        let items: Vec<&Child> = n.children.iter().collect();
        let attrs: Vec<Attr> = n
            .attrs
            .iter()
            .filter(|a| a.name != "marker")
            .cloned()
            .collect();
        let single = items.len() == 1
            && attrs.is_empty()
            && n.classes.is_empty()
            && matches!(items[0], Child::Node(i) if i.name == "item" && i.attrs.is_empty() && i.classes.is_empty());
        if single {
            let Child::Node(item) = items[0] else {
                unreachable!()
            };
            let s = self.markup(&item.children, item.span);
            self.out.push_str(&format!("{marker}{s}"));
        } else {
            self.list_block(n, depth);
        }
    }

    fn list_block(&mut self, n: &Node, depth: usize) {
        let marker = marker_of(n);
        let attrs: Vec<Attr> = n
            .attrs
            .iter()
            .filter(|a| a.name != "marker")
            .cloned()
            .collect();
        let mut parts = self.attrs(&attrs, "list");
        for child in &n.children {
            match child {
                Child::Node(i) if i.name == "item" => {
                    if !i.attrs.is_empty() || !i.classes.is_empty() {
                        // Text attributes of an item become a styled span.
                        let inner = self.inline(&i.children, false, false);
                        let mut a: Vec<String> =
                            i.classes.iter().map(|(c, _)| format!(".{c}")).collect();
                        a.extend(self.attrs(&i.attrs, "span"));
                        parts.push(format!("\"[{inner}]{{{}}}\"", a.join(" ")));
                    } else {
                        let s = self.markup(&i.children, i.span);
                        parts.push(s);
                    }
                }
                Child::Node(sub) if sub.name == "list" => {
                    let mut tmp = Printer {
                        out: String::new(),
                        comments: Vec::new(),
                        next_comment: 0,
                        errors: Vec::new(),
                    };
                    tmp.list_block(sub, depth);
                    self.errors.append(&mut tmp.errors);
                    parts.push(tmp.out);
                }
                Child::Node(other) => {
                    let mut tmp = Printer {
                        out: String::new(),
                        comments: Vec::new(),
                        next_comment: 0,
                        errors: Vec::new(),
                    };
                    tmp.node(other, Ctx::List, depth);
                    self.errors.append(&mut tmp.errors);
                    parts.push(tmp.out);
                }
                Child::Text(t, _) => parts.push(quote(t)),
                Child::Param(p, _) => parts.push(format!("${p}")),
            }
        }
        let one_line = parts.iter().map(String::len).sum::<usize>() < 70;
        if one_line {
            self.out
                .push_str(&format!("{marker}{{{}}}", parts.join(" ")));
        } else {
            self.out.push_str(&format!("{marker}{{\n"));
            for part in parts {
                self.indent(depth + 1);
                self.out.push_str(&part);
                self.out.push('\n');
            }
            self.indent(depth);
            self.out.push('}');
        }
    }
}

fn marker_of(n: &Node) -> char {
    match n
        .attrs
        .iter()
        .find(|a| a.name == "marker")
        .map(|a| &a.value)
    {
        Some(Value::Ident(m)) if m == "number" => '+',
        Some(Value::Ident(m)) if m == "none" => '=',
        _ => '-',
    }
}

fn element_is_component(name: &str) -> bool {
    !crate::analyze::element_names().contains(&name)
        && name != "slot"
        && name != "define"
        && name != "style"
}

/// Nodes short enough to keep inside their parent's braces on one line.
fn is_short(n: &Node) -> bool {
    n.children
        .iter()
        .all(|c| matches!(c, Child::Text(t, _) if t.len() < 40))
        && n.children.len() <= 1
        && !matches!(n.name.as_str(), "list" | "codeblock")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(src: &str) {
        let long = crate::compile(src).unwrap_or_else(|e| panic!("{e:#?}"));
        let ast = crate::parse(src).unwrap();
        let compact = to_compact(&ast).unwrap_or_else(|e| panic!("{e:#?}"));
        let short = crate::compile(&compact).unwrap_or_else(|e| panic!("{compact}\n{e:#?}"));
        assert_eq!(
            crate::model::Document {
                version: String::new(),
                ..long
            },
            crate::model::Document {
                version: String::new(),
                ..short
            },
            "\n{compact}"
        );
        // Printing the compact form again changes nothing.
        let again = to_compact(&crate::parse(&compact).unwrap()).unwrap();
        assert_eq!(again, compact);
    }

    #[test]
    fn upgrades_a_page() {
        round_trip(
            r##"slml 0.1
            // A comment survives.
            meta(title: "Hi", lang: "pl", icon: "/i.png")
            style card(padding: 20, fill: #ffffff, radius: 12)
            define tile(title, tone: #eeeeee) {
              column.card(fill: $tone) { heading(level: 3) $title slot }
            }
            page(fill: #f6f6f9, align: center) {
              column(width: fill, max-width: 760, gap: 24) {
                row(align: center, gap: 12) {
                  image(src: "/logo.png", alt: "Logo", width: 32, to: "/index.slml")
                  spacer
                  text { link(to: "about.slml", underline: false) "About" }
                }
                heading(id: "top") "Title"
                text { "Plain, " bold "bold " italic "both" ", " code "x*y" " and a " link(to: "#top") "link" ". 5 * 3 $5" }
                text(color: #555, size: 14) "Small"
                tile(title: "Fast") { text "Body" }
                list(marker: number, gap: 8) { item "one" item { bold "two" } list { item "a" } }
                list { item "single" }
                list { item "other" }
                codeblock "line 1\n  line 2"
                divider(color: #dddddd)
                grid(cell-width: 200, gap: 8) { text "a" text "b" }
                form(to: "https://e.com/f", method: post) {
                  input(name: "email", label: "E-mail", kind: email, required: true, placeholder: "you@x.y")
                  select(name: "s", label: "Size") { option "S" option(value: "m", selected: true) "M" }
                  checkbox(name: "c", label: "C", checked: true)
                  submit "Send"
                }
                button(to: "/a.slml") "Go"
              }
            }"##,
        );
    }

    #[test]
    fn keeps_comments() {
        let src = "slml 0.1\n// top\npage {\n  // inside\n  text \"x\"\n}\n";
        let out = to_compact(&crate::parse(src).unwrap()).unwrap();
        assert!(out.contains("// top\npage"), "{out}");
        assert!(out.contains("  // inside\n"), "{out}");
    }
}
