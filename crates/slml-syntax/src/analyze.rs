//! Validation: turns the untyped [`ast`] into the typed [`model`].
//!
//! This is where SLML's strictness lives. Unknown elements, unknown or
//! misplaced attributes, wrong value types, missing required attributes
//! (like an image's `alt`) and content in the wrong place are all errors.
//! All errors in a document are collected, not just the first one.

use std::collections::{HashMap, HashSet};

use crate::ast::{self, Child, Value};
use crate::diagnostic::{Diagnostic, Span, suggest};
use crate::model::*;

pub const SUPPORTED_VERSION: &str = "0.1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Meta,
    Page,
    Column,
    Row,
    Text,
    Heading,
    Button,
    Image,
    Divider,
    Spacer,
    Bold,
    Italic,
    Code,
    Link,
    Span,
}

const ELEMENTS: &[(&str, Kind)] = &[
    ("meta", Kind::Meta),
    ("page", Kind::Page),
    ("column", Kind::Column),
    ("row", Kind::Row),
    ("text", Kind::Text),
    ("heading", Kind::Heading),
    ("button", Kind::Button),
    ("image", Kind::Image),
    ("divider", Kind::Divider),
    ("spacer", Kind::Spacer),
    ("bold", Kind::Bold),
    ("italic", Kind::Italic),
    ("code", Kind::Code),
    ("link", Kind::Link),
    ("span", Kind::Span),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Category {
    Meta,
    Container,
    TextBlock,
    Leaf,
    Inline,
}

const FRAME: &[&str] = &[
    "width",
    "height",
    "max-width",
    "padding",
    "padding-x",
    "padding-y",
    "fill",
    "radius",
    "border",
    "border-color",
];
const FLOW: &[&str] = &["gap", "align", "justify"];
const TEXT: &[&str] = &["color", "size", "bold", "italic", "font", "underline"];

impl Kind {
    fn name(self) -> &'static str {
        ELEMENTS
            .iter()
            .find(|(_, k)| *k == self)
            .map(|(n, _)| *n)
            .unwrap_or("?")
    }

    fn category(self) -> Category {
        match self {
            Kind::Meta => Category::Meta,
            Kind::Page | Kind::Column | Kind::Row => Category::Container,
            Kind::Text | Kind::Heading | Kind::Button => Category::TextBlock,
            Kind::Image | Kind::Divider | Kind::Spacer => Category::Leaf,
            Kind::Bold | Kind::Italic | Kind::Code | Kind::Link | Kind::Span => Category::Inline,
        }
    }

    fn attr_groups(self) -> &'static [&'static [&'static str]] {
        match self {
            Kind::Meta => &[&["title", "lang", "description"]],
            Kind::Page | Kind::Column | Kind::Row => &[FRAME, FLOW, TEXT],
            Kind::Text => &[FRAME, TEXT, &["align"]],
            Kind::Heading => &[FRAME, TEXT, &["align", "level"]],
            Kind::Button => &[FRAME, TEXT, &["align", "to"]],
            Kind::Image => &[FRAME, &["src", "alt"]],
            Kind::Divider => &[&["width", "color", "thickness"]],
            Kind::Spacer => &[&["width", "height"]],
            Kind::Bold | Kind::Italic | Kind::Code | Kind::Span => &[TEXT],
            Kind::Link => &[TEXT, &["to"]],
        }
    }

    fn accepts(self, attr: &str) -> bool {
        self.attr_groups().iter().any(|g| g.contains(&attr))
    }

    fn attr_names(self) -> Vec<&'static str> {
        self.attr_groups()
            .iter()
            .flat_map(|g| g.iter().copied())
            .collect()
    }
}

fn all_attr_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = ELEMENTS.iter().flat_map(|(_, k)| k.attr_names()).collect();
    names.sort_unstable();
    names.dedup();
    names
}

fn element_names() -> Vec<&'static str> {
    ELEMENTS.iter().map(|(n, _)| *n).collect()
}

/// Attribute values gathered from styles and the element itself.
#[derive(Default)]
struct Props {
    width: Option<Size>,
    height: Option<Size>,
    max_width: Option<f32>,
    padding: Option<f32>,
    padding_x: Option<f32>,
    padding_y: Option<f32>,
    fill: Option<Color>,
    radius: Option<f32>,
    border: Option<f32>,
    border_color: Option<Color>,
    gap: Option<f32>,
    align: Option<Align>,
    text_align: Option<TextAlign>,
    justify: Option<Justify>,
    text: TextStyle,
    level: Option<u8>,
    to: Option<String>,
    src: Option<String>,
    alt: Option<String>,
    thickness: Option<f32>,
    title: Option<String>,
    lang: Option<String>,
    description: Option<String>,
    /// Attributes that were written, even if their value was invalid.
    given: HashSet<String>,
}

impl Props {
    fn frame(&self, mut f: Frame) -> Frame {
        if let Some(v) = self.width {
            f.width = v;
        }
        if let Some(v) = self.height {
            f.height = v;
        }
        if self.max_width.is_some() {
            f.max_width = self.max_width;
        }
        if let Some(v) = self.padding {
            f.padding_x = v;
            f.padding_y = v;
        }
        if let Some(v) = self.padding_x {
            f.padding_x = v;
        }
        if let Some(v) = self.padding_y {
            f.padding_y = v;
        }
        if self.fill.is_some() {
            f.fill = self.fill;
        }
        if let Some(v) = self.radius {
            f.radius = v;
        }
        if let Some(v) = self.border {
            f.border = v;
        }
        if self.border_color.is_some() {
            f.border_color = self.border_color;
        }
        f
    }

    fn flow(&self, mut f: Flow) -> Flow {
        if let Some(v) = self.gap {
            f.gap = v;
        }
        if let Some(v) = self.align {
            f.align = v;
        }
        if let Some(v) = self.justify {
            f.justify = v;
        }
        f
    }

    /// The element's own text style, falling back to the element's defaults.
    fn text_or(&self, d: TextStyle) -> TextStyle {
        let t = &self.text;
        TextStyle {
            color: t.color.or(d.color),
            size: t.size.or(d.size),
            bold: t.bold.or(d.bold),
            italic: t.italic.or(d.italic),
            mono: t.mono.or(d.mono),
            underline: t.underline.or(d.underline),
        }
    }
}

// ---- value conversion -------------------------------------------------------

fn type_error(a: &ast::Attr, expected: &str) -> Diagnostic {
    Diagnostic::new(
        a.value_span,
        format!(
            "`{}` expects {expected}, found {}",
            a.name,
            a.value.describe()
        ),
    )
}

fn number(a: &ast::Attr) -> Result<f32, Diagnostic> {
    match a.value {
        Value::Number(n) => Ok(n as f32),
        _ => Err(type_error(a, "a number")),
    }
}

fn size(a: &ast::Attr) -> Result<Size, Diagnostic> {
    match &a.value {
        Value::Number(n) => Ok(Size::Px(*n as f32)),
        Value::Ident(i) if i == "fill" => Ok(Size::Fill),
        Value::Ident(i) if i == "auto" => Ok(Size::Auto),
        _ => Err(type_error(a, "a number, `fill` or `auto`")),
    }
}

fn color(a: &ast::Attr) -> Result<Color, Diagnostic> {
    match a.value {
        Value::Color(c) => Ok(Color(c)),
        _ => {
            Err(type_error(a, "a color like #1e40af")
                .with_hint("SLML has no named colors; use hex"))
        }
    }
}

fn boolean(a: &ast::Attr) -> Result<bool, Diagnostic> {
    match &a.value {
        Value::Ident(i) if i == "true" => Ok(true),
        Value::Ident(i) if i == "false" => Ok(false),
        _ => Err(type_error(a, "`true` or `false`")),
    }
}

fn string(a: &ast::Attr) -> Result<String, Diagnostic> {
    match &a.value {
        Value::Str(s) => Ok(s.clone()),
        _ => Err(type_error(a, "a string in double quotes")),
    }
}

fn choice(a: &ast::Attr, options: &[&'static str]) -> Result<&'static str, Diagnostic> {
    let list = options
        .iter()
        .map(|o| format!("`{o}`"))
        .collect::<Vec<_>>()
        .join(", ");
    match &a.value {
        Value::Ident(i) => options.iter().find(|o| **o == i).copied().ok_or_else(|| {
            Diagnostic::new(
                a.value_span,
                format!("`{i}` is not a valid value for `{}`", a.name),
            )
            .with_hint(format!("use one of {list}"))
        }),
        _ => Err(type_error(a, &format!("one of {list}"))),
    }
}

fn apply_attr(p: &mut Props, kind: Kind, a: &ast::Attr) -> Result<(), Diagnostic> {
    match a.name.as_str() {
        "width" => p.width = Some(size(a)?),
        "height" => p.height = Some(size(a)?),
        "max-width" => p.max_width = Some(number(a)?),
        "padding" => p.padding = Some(number(a)?),
        "padding-x" => p.padding_x = Some(number(a)?),
        "padding-y" => p.padding_y = Some(number(a)?),
        "fill" => p.fill = Some(color(a)?),
        "radius" => p.radius = Some(number(a)?),
        "border" => p.border = Some(number(a)?),
        "border-color" => p.border_color = Some(color(a)?),
        "gap" => p.gap = Some(number(a)?),
        "align" if kind.category() == Category::TextBlock => {
            p.text_align = Some(match choice(a, &["start", "center", "end"])? {
                "center" => TextAlign::Center,
                "end" => TextAlign::End,
                _ => TextAlign::Start,
            })
        }
        "align" => {
            p.align = Some(match choice(a, &["start", "center", "end", "stretch"])? {
                "start" => Align::Start,
                "center" => Align::Center,
                "end" => Align::End,
                _ => Align::Stretch,
            })
        }
        "justify" => {
            p.justify = Some(match choice(a, &["start", "center", "end", "between"])? {
                "center" => Justify::Center,
                "end" => Justify::End,
                "between" => Justify::Between,
                _ => Justify::Start,
            })
        }
        "color" => p.text.color = Some(color(a)?),
        "size" => {
            let n = number(a)?;
            if n <= 0.0 {
                return Err(Diagnostic::new(
                    a.value_span,
                    "`size` must be greater than 0",
                ));
            }
            p.text.size = Some(n);
        }
        "bold" => p.text.bold = Some(boolean(a)?),
        "italic" => p.text.italic = Some(boolean(a)?),
        "underline" => p.text.underline = Some(boolean(a)?),
        "font" => p.text.mono = Some(choice(a, &["sans", "mono"])? == "mono"),
        "level" => {
            let n = number(a)?;
            if n.fract() != 0.0 || !(1.0..=3.0).contains(&n) {
                return Err(Diagnostic::new(a.value_span, "`level` must be 1, 2 or 3"));
            }
            p.level = Some(n as u8);
        }
        "to" => {
            let s = string(a)?;
            if s.is_empty() {
                return Err(Diagnostic::new(a.value_span, "`to` cannot be empty"));
            }
            p.to = Some(s);
        }
        "src" => {
            let s = string(a)?;
            // Documents may only read files inside their own folder.
            let escapes = s.starts_with('/')
                || s.starts_with('\\')
                || s.split(['/', '\\']).any(|c| c == "..");
            if !s.contains("://") && (escapes || s.get(1..2) == Some(":")) {
                return Err(Diagnostic::new(
                    a.value_span,
                    "image path leaves the document's folder",
                )
                .with_hint(
                    "use a path relative to this file without `..`, e.g. \"img/logo.png\"",
                ));
            }
            p.src = Some(s);
        }
        "alt" => p.alt = Some(string(a)?),
        "thickness" => p.thickness = Some(number(a)?),
        "title" => p.title = Some(string(a)?),
        "lang" => p.lang = Some(string(a)?),
        "description" => p.description = Some(string(a)?),
        other => unreachable!("attribute `{other}` passed validation but has no handler"),
    }
    Ok(())
}

// ---- tree walk -------------------------------------------------------------

pub fn analyze(doc: &ast::Document) -> Result<Document, Vec<Diagnostic>> {
    let mut an = Analyzer {
        styles: HashMap::new(),
        errors: Vec::new(),
    };

    if doc.version != SUPPORTED_VERSION {
        an.errors.push(
            Diagnostic::new(
                doc.version_span,
                format!("unsupported SLML version `{}`", doc.version),
            )
            .with_hint(format!("this build understands `slml {SUPPORTED_VERSION}`")),
        );
    }

    an.register_styles(&doc.styles);

    let mut meta = Meta::default();
    let mut page: Option<Container> = None;
    let mut seen_meta = false;
    for node in &doc.nodes {
        match an.lookup(node) {
            Some(Kind::Meta) => {
                if std::mem::replace(&mut seen_meta, true) {
                    an.err(node.span, "a document can have only one `meta`");
                }
                meta = an.meta(node);
            }
            Some(Kind::Page) => {
                if page.is_some() {
                    an.err(node.span, "a document can have only one `page`");
                } else {
                    page = Some(an.container(node, Kind::Page));
                }
            }
            Some(k) => an.errors.push(
                Diagnostic::new(node.span, format!("`{}` must be inside `page`", k.name()))
                    .with_hint("the top level may only contain `meta`, `style` and `page`"),
            ),
            None => {}
        }
    }

    let page = page.unwrap_or_else(|| {
        an.errors.push(
            Diagnostic::new(doc.version_span, "document has no `page`")
                .with_hint("add `page { text \"Hello\" }`"),
        );
        an.empty_page()
    });

    if an.errors.is_empty() {
        Ok(Document {
            version: doc.version.clone(),
            meta,
            page,
        })
    } else {
        an.errors.sort_by_key(|d| d.span.start);
        Err(an.errors)
    }
}

struct Analyzer<'a> {
    styles: HashMap<&'a str, &'a ast::StyleDef>,
    errors: Vec<Diagnostic>,
}

impl<'a> Analyzer<'a> {
    fn err(&mut self, span: Span, msg: impl Into<String>) {
        self.errors.push(Diagnostic::new(span, msg));
    }

    fn register_styles(&mut self, defs: &'a [ast::StyleDef]) {
        let known = all_attr_names();
        for def in defs {
            if self.styles.insert(&def.name, def).is_some() {
                self.err(def.span, format!("style `{}` is defined twice", def.name));
            }
            let mut seen = HashSet::new();
            for a in &def.attrs {
                if !known.contains(&a.name.as_str()) {
                    let mut d = Diagnostic::new(a.span, format!("unknown attribute `{}`", a.name));
                    if let Some(s) = suggest(&a.name, &known) {
                        d = d.with_hint(format!("did you mean `{s}`?"));
                    }
                    self.errors.push(d);
                } else if !seen.insert(a.name.as_str()) {
                    self.err(a.span, format!("`{}` is set twice", a.name));
                }
            }
        }
    }

    fn lookup(&mut self, node: &ast::Node) -> Option<Kind> {
        if let Some((_, k)) = ELEMENTS.iter().find(|(n, _)| *n == node.name) {
            return Some(*k);
        }
        let mut d = Diagnostic::new(node.span, format!("unknown element `{}`", node.name));
        if node.name == "style" {
            d = d.with_hint("styles are defined at the top level: `style name(...)`");
        } else if let Some(s) = suggest(&node.name, &element_names()) {
            d = d.with_hint(format!("did you mean `{s}`?"));
        } else if let Some(k) = html_equivalent(&node.name) {
            d = d.with_hint(format!("in SLML this is `{k}`"));
        }
        self.errors.push(d);
        None
    }

    /// Gathers attributes from `.style` classes, then the element's own
    /// attributes (which win).
    fn props(&mut self, node: &ast::Node, kind: Kind) -> Props {
        let mut p = Props::default();
        for (class, cspan) in &node.classes {
            let Some(def) = self.styles.get(class.as_str()).copied() else {
                let names: Vec<&str> = self.styles.keys().copied().collect();
                let mut d = Diagnostic::new(*cspan, format!("unknown style `{class}`"));
                if let Some(s) = suggest(class, &names) {
                    d = d.with_hint(format!("did you mean `{s}`?"));
                }
                self.errors.push(d);
                continue;
            };
            for a in &def.attrs {
                if !kind.accepts(&a.name) {
                    self.errors.push(
                        Diagnostic::new(
                            *cspan,
                            format!(
                                "style `{class}` sets `{}`, which `{}` does not accept",
                                a.name,
                                kind.name()
                            ),
                        )
                        .with_hint(format!("`{}` is set on line {}", a.name, a.span.line)),
                    );
                } else {
                    p.given.insert(a.name.clone());
                    if let Err(e) = apply_attr(&mut p, kind, a) {
                        self.errors.push(e);
                    }
                }
            }
        }
        let mut seen = HashSet::new();
        for a in &node.attrs {
            if !seen.insert(a.name.as_str()) {
                self.err(a.span, format!("`{}` is set twice", a.name));
                continue;
            }
            if !kind.accepts(&a.name) {
                let mut d = Diagnostic::new(
                    a.span,
                    format!("`{}` has no attribute `{}`", kind.name(), a.name),
                );
                let own = kind.attr_names();
                if let Some(s) = suggest(&a.name, &own) {
                    d = d.with_hint(format!("did you mean `{s}`?"));
                } else if !own.is_empty() {
                    d = d.with_hint(format!("`{}` accepts: {}", kind.name(), own.join(", ")));
                }
                self.errors.push(d);
                continue;
            }
            p.given.insert(a.name.clone());
            if let Err(e) = apply_attr(&mut p, kind, a) {
                self.errors.push(e);
            }
        }
        p
    }

    /// Reports a missing required attribute. An attribute that was written
    /// with an invalid value has already been reported, so it is not
    /// reported again.
    fn require<T>(
        &mut self,
        value: Option<T>,
        given: &HashSet<String>,
        node: &ast::Node,
        attr: &str,
        hint: &str,
    ) -> Option<T> {
        if value.is_none() && !given.contains(attr) {
            self.errors.push(
                Diagnostic::new(node.span, format!("`{}` requires `{attr}`", node.name))
                    .with_hint(hint.to_string()),
            );
        }
        value
    }

    fn no_children(&mut self, node: &ast::Node, hint: &str) {
        if let Some(first) = node.children.first() {
            let span = match first {
                Child::Node(n) => n.span,
                Child::Text(_, s) => *s,
            };
            self.errors.push(
                Diagnostic::new(span, format!("`{}` cannot have content", node.name))
                    .with_hint(hint.to_string()),
            );
        }
    }

    fn meta(&mut self, node: &ast::Node) -> Meta {
        let p = self.props(node, Kind::Meta);
        self.no_children(node, "write `meta(title: \"...\")`");
        Meta {
            title: p.title,
            lang: p.lang,
            description: p.description,
        }
    }

    fn empty_page(&self) -> Container {
        Container {
            kind: ContainerKind::Page,
            frame: Frame::default(),
            flow: Flow::default(),
            text: TextStyle::default(),
            children: Vec::new(),
        }
    }

    fn container(&mut self, node: &ast::Node, kind: Kind) -> Container {
        let p = self.props(node, kind);
        let (ckind, frame, flow, text) = match kind {
            Kind::Page => (
                ContainerKind::Page,
                p.frame(Frame {
                    padding_x: 32.0,
                    padding_y: 32.0,
                    fill: Some(Color::WHITE),
                    ..Frame::default()
                }),
                p.flow(Flow {
                    gap: 16.0,
                    ..Flow::default()
                }),
                // The page fixes every inherited property, so nothing is left
                // to a renderer's taste.
                p.text_or(TextStyle {
                    color: Some(Color(0x1b1b1fff)),
                    size: Some(16.0),
                    bold: Some(false),
                    italic: Some(false),
                    mono: Some(false),
                    underline: Some(false),
                }),
            ),
            Kind::Row => (
                ContainerKind::Row,
                p.frame(Frame::default()),
                p.flow(Flow::default()),
                p.text_or(TextStyle::default()),
            ),
            _ => (
                ContainerKind::Column,
                p.frame(Frame::default()),
                p.flow(Flow::default()),
                p.text_or(TextStyle::default()),
            ),
        };

        let mut children = Vec::new();
        for child in &node.children {
            match child {
                Child::Text(_, span) => self.errors.push(
                    Diagnostic::new(
                        *span,
                        format!("text cannot be placed directly in `{}`", node.name),
                    )
                    .with_hint("wrap it in an element: `text \"...\"`"),
                ),
                Child::Node(n) => {
                    if let Some(c) = self.block(n, &node.name) {
                        children.push(c);
                    }
                }
            }
        }
        Container {
            kind: ckind,
            frame,
            flow,
            text,
            children,
        }
    }

    /// After reporting an unknown element, keep checking its content as the
    /// element it most likely was, so one typo doesn't hide other errors.
    fn recover(&mut self, n: &ast::Node) {
        let guess = suggest(&n.name, &element_names())
            .or_else(|| html_equivalent(&n.name).map(|h| h.split('(').next().unwrap_or(h)))
            .and_then(|name| ELEMENTS.iter().find(|(e, _)| *e == name))
            .map(|(_, k)| *k);
        match guess.map(|k| (k, k.category())) {
            Some((k, Category::Container)) => {
                self.container(n, k);
            }
            Some((k, Category::TextBlock)) => {
                self.text_block(n, k);
            }
            Some((k, Category::Leaf)) => {
                self.leaf(n, k);
            }
            Some((_, Category::Inline)) => {
                self.inlines(&n.children, &n.name, false);
            }
            _ => {}
        }
    }

    fn block(&mut self, n: &ast::Node, parent: &str) -> Option<Node> {
        let Some(kind) = self.lookup(n) else {
            self.recover(n);
            return None;
        };
        match kind.category() {
            Category::Container if kind != Kind::Page => {
                Some(Node::Container(self.container(n, kind)))
            }
            Category::TextBlock => Some(Node::Text(self.text_block(n, kind))),
            Category::Leaf => Some(self.leaf(n, kind)),
            Category::Inline => {
                self.errors.push(
                    Diagnostic::new(
                        n.span,
                        format!("`{}` cannot be placed directly in `{parent}`", n.name),
                    )
                    .with_hint(format!(
                        "inline elements go inside text: `text {{ {} \"...\" }}`",
                        n.name
                    )),
                );
                None
            }
            _ => {
                self.err(
                    n.span,
                    format!("`{}` can only appear at the top level", n.name),
                );
                None
            }
        }
    }

    fn text_block(&mut self, n: &ast::Node, kind: Kind) -> TextBlock {
        let p = self.props(n, kind);
        let (tkind, frame, text, link) = match kind {
            Kind::Heading => {
                let level = p.level.unwrap_or(1);
                let size = match level {
                    1 => 32.0,
                    2 => 24.0,
                    _ => 20.0,
                };
                (
                    TextKind::Heading(level),
                    p.frame(Frame::default()),
                    p.text_or(TextStyle {
                        size: Some(size),
                        bold: Some(true),
                        ..TextStyle::default()
                    }),
                    None,
                )
            }
            Kind::Button => {
                let to = self.require(
                    p.to.clone(),
                    &p.given,
                    n,
                    "to",
                    "e.g. `button(to: \"contact.slml\") \"Contact\"`",
                );
                (
                    TextKind::Button,
                    p.frame(Frame {
                        padding_x: 18.0,
                        padding_y: 10.0,
                        fill: Some(Color(0x2457d6ff)),
                        radius: 8.0,
                        ..Frame::default()
                    }),
                    p.text_or(TextStyle {
                        color: Some(Color::WHITE),
                        underline: Some(false),
                        ..TextStyle::default()
                    }),
                    to,
                )
            }
            _ => (
                TextKind::Paragraph,
                p.frame(Frame::default()),
                p.text_or(TextStyle::default()),
                None,
            ),
        };
        let align = p.text_align.unwrap_or(if kind == Kind::Button {
            TextAlign::Center
        } else {
            TextAlign::Start
        });
        let content = self.inlines(&n.children, &n.name, kind == Kind::Button);
        TextBlock {
            kind: tkind,
            frame,
            text,
            align,
            link,
            content,
        }
    }

    fn inlines(&mut self, children: &[Child], parent: &str, in_link: bool) -> Vec<Inline> {
        let mut out = Vec::new();
        for child in children {
            match child {
                Child::Text(s, _) => out.push(Inline::Text(s.clone())),
                Child::Node(n) => {
                    let Some(kind) = self.lookup(n) else {
                        self.recover(n);
                        continue;
                    };
                    if kind.category() != Category::Inline {
                        self.errors.push(
                            Diagnostic::new(
                                n.span,
                                format!("`{}` cannot be placed inside `{parent}`", n.name),
                            )
                            .with_hint(
                                "text may only contain strings and bold, italic, code, link, span",
                            ),
                        );
                        continue;
                    }
                    if kind == Kind::Link && in_link {
                        self.err(n.span, "links cannot be nested inside links or buttons");
                    }
                    let p = self.props(n, kind);
                    let defaults = match kind {
                        Kind::Bold => TextStyle {
                            bold: Some(true),
                            ..TextStyle::default()
                        },
                        Kind::Italic => TextStyle {
                            italic: Some(true),
                            ..TextStyle::default()
                        },
                        Kind::Code => TextStyle {
                            mono: Some(true),
                            ..TextStyle::default()
                        },
                        Kind::Link => TextStyle {
                            color: Some(Color(0x2457d6ff)),
                            underline: Some(true),
                            ..TextStyle::default()
                        },
                        _ => TextStyle::default(),
                    };
                    let link = if kind == Kind::Link {
                        self.require(
                            p.to.clone(),
                            &p.given,
                            n,
                            "to",
                            "e.g. `link(to: \"about.slml\") \"About\"`",
                        )
                    } else {
                        None
                    };
                    let content = self.inlines(&n.children, &n.name, in_link || kind == Kind::Link);
                    out.push(Inline::Span(InlineSpan {
                        text: p.text_or(defaults),
                        link,
                        content,
                    }));
                }
            }
        }
        out
    }

    fn leaf(&mut self, n: &ast::Node, kind: Kind) -> Node {
        let p = self.props(n, kind);
        match kind {
            Kind::Image => {
                self.no_children(n, "describe the image with `alt: \"...\"` instead");
                let src = self.require(
                    p.src.clone(),
                    &p.given,
                    n,
                    "src",
                    "e.g. `image(src: \"logo.png\", alt: \"Logo\")`",
                );
                let alt = self.require(
                    p.alt.clone(),
                    &p.given,
                    n,
                    "alt",
                    "describe the image for people who cannot see it; use `alt: \"\"` only for decoration",
                );
                Node::Image(Image {
                    frame: p.frame(Frame::default()),
                    src: src.unwrap_or_default(),
                    alt: alt.unwrap_or_default(),
                })
            }
            Kind::Divider => {
                self.no_children(n, "a divider is just a line");
                Node::Divider(Divider {
                    width: p.width.unwrap_or(Size::Auto),
                    thickness: p.thickness.unwrap_or(1.0),
                    color: p.text.color.unwrap_or(Color(0xd4d4d8ff)),
                })
            }
            _ => {
                self.no_children(n, "a spacer is empty space");
                Node::Spacer(Spacer {
                    width: p.width.unwrap_or(Size::Auto),
                    height: p.height.unwrap_or(Size::Auto),
                })
            }
        }
    }
}

/// Helps people coming from HTML find the SLML equivalent.
fn html_equivalent(name: &str) -> Option<&'static str> {
    Some(match name {
        "div" | "section" | "main" | "article" | "body" => "column",
        "p" | "label" => "text",
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => "heading(level: N)",
        "a" => "link(to: \"...\")",
        "img" => "image(src: \"...\", alt: \"...\")",
        "hr" => "divider",
        "b" | "strong" => "bold",
        "i" | "em" => "italic",
        "br" => "\\n inside a string",
        "head" | "title" => "meta(title: \"...\")",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use crate::compile;
    use crate::model::*;

    fn errors(src: &str) -> Vec<String> {
        compile(src)
            .unwrap_err()
            .into_iter()
            .map(|d| d.message)
            .collect()
    }

    #[test]
    fn compiles_and_applies_defaults() {
        let doc = compile(
            r#"slml 0.1
            meta(title: "Test")
            style card(padding: 12, fill: #eee)
            page {
              heading(level: 2) "Hi"
              row.card(padding-x: 20) { text { "a " link(to: "b.slml") "b" } }
            }"#,
        )
        .unwrap();
        assert_eq!(doc.meta.title.as_deref(), Some("Test"));
        let Node::Text(h) = &doc.page.children[0] else {
            panic!()
        };
        assert_eq!(h.text.size, Some(24.0));
        assert_eq!(h.text.bold, Some(true));
        let Node::Container(row) = &doc.page.children[1] else {
            panic!()
        };
        assert_eq!(row.frame.padding_x, 20.0, "own attribute overrides style");
        assert_eq!(row.frame.padding_y, 12.0);
        assert_eq!(row.frame.fill, Some(Color(0xeeeeeeff)));
    }

    #[test]
    fn image_requires_alt() {
        assert_eq!(
            errors(r#"slml 0.1 page { image(src: "a.png") }"#),
            vec!["`image` requires `alt`"]
        );
    }

    #[test]
    fn rejects_bare_text_and_misplaced_elements() {
        let e = errors(r#"slml 0.1 page { "hi" text { row { } } bold "x" }"#);
        assert_eq!(e.len(), 3, "{e:?}");
    }

    #[test]
    fn unknown_names_get_suggestions() {
        let d = crate::compile("slml 0.1 page { colum { } div { } text(colr: #fff) \"x\" }")
            .unwrap_err();
        assert_eq!(d[0].hint.as_deref(), Some("did you mean `column`?"));
        assert_eq!(d[1].hint.as_deref(), Some("in SLML this is `column`"));
        assert_eq!(d[2].hint.as_deref(), Some("did you mean `color`?"));
    }

    #[test]
    fn errors_inside_unknown_elements_are_still_reported() {
        let e = errors(r#"slml 0.1 page { colum { text(colr: #fff) "x" image(src: "a.png") } }"#);
        assert_eq!(e.len(), 3, "{e:?}");
    }

    #[test]
    fn type_checks_values() {
        let e = errors(r#"slml 0.1 page(gap: "big", align: middle, fill: red) { }"#);
        assert_eq!(e.len(), 3, "{e:?}");
    }

    #[test]
    fn style_attributes_must_fit_element() {
        let e = errors("slml 0.1 style s(gap: 4) page { text.s \"x\" }");
        assert!(e[0].contains("does not accept"), "{e:?}");
    }

    #[test]
    fn version_and_structure() {
        assert!(errors("slml 9.9 page { }")[0].contains("unsupported"));
        assert!(errors("slml 0.1")[0].contains("no `page`"));
        assert!(errors("slml 0.1 page { } page { }")[0].contains("only one"));
    }

    #[test]
    fn image_paths_stay_in_document_folder() {
        for bad in ["/etc/passwd", "../secret.png", "a/../../b.png", "C:/x.png"] {
            let src = format!("slml 0.1 page {{ image(src: \"{bad}\", alt: \"x\") }}");
            assert!(errors(&src)[0].contains("leaves"), "{bad}");
        }
        assert!(compile(r#"slml 0.1 page { image(src: "img/a.png", alt: "x") }"#).is_ok());
    }

    #[test]
    fn no_nested_links() {
        let e = errors(r#"slml 0.1 page { button(to: "a") { link(to: "b") "x" } }"#);
        assert!(e[0].contains("nested"), "{e:?}");
    }
}
