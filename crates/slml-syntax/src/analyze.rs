//! Validation: turns the (component-expanded) [`ast`] into the typed
//! [`model`].
//!
//! This is where SLML's strictness lives. Unknown elements, unknown or
//! misplaced attributes, wrong value types, missing required attributes
//! (like an image's `alt` or a field's `label`), content in the wrong place,
//! nested links, duplicate ids and links to missing anchors are all errors.
//! All errors in a document are collected, not just the first one.

use std::collections::{HashMap, HashSet};

use crate::ast::{self, Child, Value};
use crate::diagnostic::{Diagnostic, Span, suggest};
use crate::model::*;
use crate::names::{self, Dialect};

/// The newest language version (written by `slml upgrade`).
pub const SUPPORTED_VERSION: &str = "0.2";
/// Every version this build reads: 0.1 (long syntax) and 0.2 (compact).
pub const SUPPORTED_VERSIONS: &[&str] = &["0.1", "0.2"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Meta,
    Page,
    Column,
    Row,
    Grid,
    Form,
    List,
    Item,
    Text,
    Heading,
    Button,
    Submit,
    Codeblock,
    Image,
    Divider,
    Spacer,
    Input,
    Textarea,
    Checkbox,
    Select,
    Option,
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
    ("grid", Kind::Grid),
    ("form", Kind::Form),
    ("list", Kind::List),
    ("item", Kind::Item),
    ("text", Kind::Text),
    ("heading", Kind::Heading),
    ("button", Kind::Button),
    ("submit", Kind::Submit),
    ("codeblock", Kind::Codeblock),
    ("image", Kind::Image),
    ("divider", Kind::Divider),
    ("spacer", Kind::Spacer),
    ("input", Kind::Input),
    ("textarea", Kind::Textarea),
    ("checkbox", Kind::Checkbox),
    ("select", Kind::Select),
    ("option", Kind::Option),
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
    List,
    TextBlock,
    Leaf,
    Field,
    Option,
    Inline,
}

const FRAME: &[&str] = &[
    "width",
    "height",
    "min-width",
    "max-width",
    "padding",
    "padding-x",
    "padding-y",
    "fill",
    "radius",
    "border",
    "border-color",
    "shadow",
    "id",
];
const FLOW: &[&str] = &["gap", "align", "justify"];
const TEXT: &[&str] = &["color", "size", "bold", "italic", "font", "underline"];
const FIELD: &[&str] = &["name", "label", "required"];

impl Kind {
    fn category(self) -> Category {
        match self {
            Kind::Meta => Category::Meta,
            Kind::Page | Kind::Column | Kind::Row | Kind::Grid | Kind::Form => Category::Container,
            Kind::List => Category::List,
            Kind::Item
            | Kind::Text
            | Kind::Heading
            | Kind::Button
            | Kind::Submit
            | Kind::Codeblock => Category::TextBlock,
            Kind::Image | Kind::Divider | Kind::Spacer => Category::Leaf,
            Kind::Input | Kind::Textarea | Kind::Checkbox | Kind::Select => Category::Field,
            Kind::Option => Category::Option,
            Kind::Bold | Kind::Italic | Kind::Code | Kind::Link | Kind::Span => Category::Inline,
        }
    }

    fn attr_groups(self) -> &'static [&'static [&'static str]] {
        match self {
            Kind::Meta => &[&["title", "lang", "description", "icon"]],
            Kind::Page => &[FRAME, FLOW, TEXT],
            Kind::Column => &[FRAME, FLOW, TEXT, &["to"]],
            Kind::Row => &[FRAME, FLOW, TEXT, &["to", "wrap"]],
            Kind::Grid => &[FRAME, FLOW, TEXT, &["columns", "cell-width"]],
            Kind::Form => &[FRAME, FLOW, TEXT, &["to", "method"]],
            Kind::List => &[FRAME, TEXT, &["gap", "marker"]],
            Kind::Item => &[TEXT, &["align", "id"]],
            Kind::Text | Kind::Submit | Kind::Codeblock => &[FRAME, TEXT, &["align"]],
            Kind::Heading => &[FRAME, TEXT, &["align", "level"]],
            Kind::Button => &[FRAME, TEXT, &["align", "to"]],
            Kind::Image => &[FRAME, &["src", "alt", "to"]],
            Kind::Divider => &[&["width", "color", "thickness", "id"]],
            Kind::Spacer => &[&["width", "height"]],
            Kind::Input => &[FRAME, TEXT, FIELD, &["kind", "placeholder", "value"]],
            Kind::Textarea => &[FRAME, TEXT, FIELD, &["placeholder", "value", "rows"]],
            Kind::Checkbox => &[TEXT, FIELD, &["checked", "id"]],
            Kind::Select => &[FRAME, TEXT, FIELD],
            Kind::Option => &[&["value", "selected"]],
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

pub fn all_attr_names() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = ELEMENTS.iter().flat_map(|(_, k)| k.attr_names()).collect();
    names.sort_unstable();
    names.dedup();
    names
}

/// Names of all built-in elements (components may not reuse them).
pub fn element_names() -> Vec<&'static str> {
    ELEMENTS.iter().map(|(n, _)| *n).collect()
}

/// Attribute values gathered from styles and the element itself.
#[derive(Default)]
struct Props {
    width: Option<Size>,
    height: Option<Size>,
    min_width: Option<f32>,
    max_width: Option<f32>,
    padding: Option<f32>,
    padding_x: Option<f32>,
    padding_y: Option<f32>,
    fill: Option<Color>,
    radius: Option<f32>,
    border: Option<f32>,
    border_color: Option<Color>,
    shadow: Option<Shadow>,
    id: Option<String>,
    gap: Option<f32>,
    align: Option<Align>,
    text_align: Option<TextAlign>,
    justify: Option<Justify>,
    wrap: Option<bool>,
    columns: Option<u32>,
    cell_width: Option<f32>,
    method: Option<Method>,
    marker: Option<Marker>,
    text: TextStyle,
    level: Option<u8>,
    to: Option<String>,
    src: Option<String>,
    alt: Option<String>,
    thickness: Option<f32>,
    title: Option<String>,
    lang: Option<String>,
    description: Option<String>,
    icon: Option<String>,
    name: Option<String>,
    label: Option<String>,
    required: Option<bool>,
    input_kind: Option<InputKind>,
    placeholder: Option<String>,
    value: Option<String>,
    rows: Option<u32>,
    checked: Option<bool>,
    selected: Option<bool>,
    /// Attributes that were written, even if their value was invalid.
    given: HashSet<String>,
    /// Where `id` and `to` were written, for later checks.
    id_span: Span,
    to_span: Span,
}

impl Props {
    fn frame(&self, mut f: Frame) -> Frame {
        if let Some(v) = self.width {
            f.width = v;
        }
        if let Some(v) = self.height {
            f.height = v;
        }
        if self.min_width.is_some() {
            f.min_width = self.min_width;
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
        if let Some(v) = self.shadow {
            f.shadow = v;
        }
        f.id = self.id.clone();
        if self.to.is_some() {
            f.link = self.to.clone();
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
        if let Some(v) = self.wrap {
            f.wrap = v;
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

/// An element's name as written (`c` in 0.2), for messages.
fn shown(n: &ast::Node) -> &str {
    n.shown().trim_start_matches('\u{0}')
}

/// An attribute's name as written (`p` in `p20`), for messages.
fn shown_attr(a: &ast::Attr) -> &str {
    a.shown().trim_start_matches('\u{0}')
}

// ---- value conversion -------------------------------------------------------

fn type_error(a: &ast::Attr, expected: &str) -> Diagnostic {
    Diagnostic::new(
        a.value_span,
        format!(
            "`{}` expects {expected}, found {}",
            shown_attr(a),
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

fn whole(a: &ast::Attr, min: u32, max: u32) -> Result<u32, Diagnostic> {
    let n = number(a)?;
    if n.fract() != 0.0 || n < min as f32 || n > max as f32 {
        return Err(Diagnostic::new(
            a.value_span,
            format!(
                "`{}` must be a whole number from {min} to {max}",
                shown_attr(a)
            ),
        ));
    }
    Ok(n as u32)
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
                format!("`{i}` is not a valid value for `{}`", shown_attr(a)),
            )
            .with_hint(format!("use one of {list}"))
        }),
        _ => Err(type_error(a, &format!("one of {list}"))),
    }
}

fn valid_id(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Checks a local file path: relative to the document, or starting with `/`
/// for the site root. It may never leave the site (`..`).
pub fn check_path(path: &str) -> Result<(), &'static str> {
    if path.is_empty() {
        return Err("the path is empty");
    }
    if path.contains('\\') {
        return Err("use `/` as the path separator");
    }
    if path.starts_with("//") || path.get(1..2) == Some(":") {
        return Err("only paths inside the site are allowed");
    }
    if path.split('/').any(|c| c == "..") {
        return Err("`..` is not allowed; start the path with `/` to begin at the site root");
    }
    Ok(())
}

const SCHEMES: &[&str] = &["https", "http", "mailto", "tel"];

/// Is `to` an external URL (`https:`, `mailto:` …)?
pub fn is_external(to: &str) -> bool {
    to.split_once(':')
        .is_some_and(|(scheme, _)| !scheme.contains('/') && !scheme.contains('#'))
}

fn link_target(a: &ast::Attr) -> Result<String, Diagnostic> {
    let s = string(a)?;
    let err = |msg: &str| Diagnostic::new(a.value_span, format!("invalid link target: {msg}"));
    if s.is_empty() {
        return Err(err("it is empty"));
    }
    if is_external(&s) {
        let scheme = s.split(':').next().unwrap_or("");
        if !SCHEMES.contains(&scheme) {
            return Err(err(&format!("`{scheme}:` links are not allowed"))
                .with_hint("use https:, http:, mailto: or tel:, or a path to a .slml file"));
        }
        return Ok(s);
    }
    let (path, fragment) = match s.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (s.as_str(), None),
    };
    if let Some(f) = fragment
        && !valid_id(f)
    {
        return Err(err("`#` must be followed by an element id"));
    }
    if !path.is_empty() {
        check_path(path).map_err(err)?;
    }
    Ok(s)
}

fn file_path(a: &ast::Attr) -> Result<String, Diagnostic> {
    let s = string(a)?;
    if s.starts_with("https://") || s.starts_with("http://") {
        return Ok(s);
    }
    check_path(&s).map_err(|msg| {
        Diagnostic::new(a.value_span, format!("invalid path: {msg}"))
            .with_hint("use a path relative to this file, or one starting with `/`")
    })?;
    Ok(s)
}

fn non_empty(a: &ast::Attr) -> Result<String, Diagnostic> {
    let s = string(a)?;
    if s.trim().is_empty() {
        return Err(Diagnostic::new(
            a.value_span,
            format!("`{}` cannot be empty", shown_attr(a)),
        ));
    }
    Ok(s)
}

fn apply_attr(p: &mut Props, kind: Kind, a: &ast::Attr) -> Result<(), Diagnostic> {
    match a.name.as_str() {
        "width" => p.width = Some(size(a)?),
        "height" => p.height = Some(size(a)?),
        "min-width" => p.min_width = Some(number(a)?),
        "max-width" => p.max_width = Some(number(a)?),
        "padding" => p.padding = Some(number(a)?),
        "padding-x" => p.padding_x = Some(number(a)?),
        "padding-y" => p.padding_y = Some(number(a)?),
        "fill" => p.fill = Some(color(a)?),
        "radius" => p.radius = Some(number(a)?),
        "border" => p.border = Some(number(a)?),
        "border-color" => p.border_color = Some(color(a)?),
        "shadow" => {
            p.shadow = Some(match choice(a, &["none", "small", "large"])? {
                "small" => Shadow::Small,
                "large" => Shadow::Large,
                _ => Shadow::None,
            })
        }
        "id" => {
            let s = string(a)?;
            if !valid_id(&s) {
                return Err(
                    Diagnostic::new(a.value_span, format!("`{s}` is not a valid id"))
                        .with_hint("ids start with a letter and use letters, digits, `-` and `_`"),
                );
            }
            p.id = Some(s);
            p.id_span = a.value_span;
        }
        "gap" => p.gap = Some(number(a)?),
        "align" if matches!(kind.category(), Category::TextBlock) => {
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
        "wrap" => p.wrap = Some(boolean(a)?),
        "columns" => p.columns = Some(whole(a, 1, 12)?),
        "cell-width" => {
            let n = number(a)?;
            if n <= 0.0 {
                return Err(Diagnostic::new(
                    a.value_span,
                    format!("`{}` must be greater than 0", shown_attr(a)),
                ));
            }
            p.cell_width = Some(n);
        }
        "method" => {
            p.method = Some(match choice(a, &["get", "post"])? {
                "post" => Method::Post,
                _ => Method::Get,
            })
        }
        "marker" => {
            p.marker = Some(match choice(a, &["bullet", "number", "none"])? {
                "number" => Marker::Number,
                "none" => Marker::None,
                _ => Marker::Bullet,
            })
        }
        "color" => p.text.color = Some(color(a)?),
        "size" => {
            let n = number(a)?;
            if n <= 0.0 {
                return Err(Diagnostic::new(
                    a.value_span,
                    format!("`{}` must be greater than 0", shown_attr(a)),
                ));
            }
            p.text.size = Some(n);
        }
        "bold" => p.text.bold = Some(boolean(a)?),
        "italic" => p.text.italic = Some(boolean(a)?),
        "underline" => p.text.underline = Some(boolean(a)?),
        "font" => p.text.mono = Some(choice(a, &["sans", "mono"])? == "mono"),
        "level" => p.level = Some(whole(a, 1, 4)? as u8),
        "to" if kind == Kind::Form => {
            p.to = Some(match string(a)? {
                s if is_external(&s) => link_target(a).map(|_| s)?,
                s => file_path(a).map(|_| s)?,
            });
            p.to_span = a.value_span;
        }
        "to" => {
            p.to = Some(link_target(a)?);
            p.to_span = a.value_span;
        }
        "src" => p.src = Some(file_path(a)?),
        "icon" => p.icon = Some(file_path(a)?),
        "alt" => p.alt = Some(string(a)?),
        "thickness" => p.thickness = Some(number(a)?),
        "title" => p.title = Some(string(a)?),
        "lang" => p.lang = Some(string(a)?),
        "description" => p.description = Some(string(a)?),
        "name" => {
            let s = non_empty(a)?;
            if s.chars().any(|c| c.is_whitespace()) {
                return Err(Diagnostic::new(
                    a.value_span,
                    "field names cannot contain spaces",
                ));
            }
            p.name = Some(s);
        }
        "label" => p.label = Some(non_empty(a)?),
        "required" => p.required = Some(boolean(a)?),
        "kind" => {
            let k = choice(a, &InputKind::NAMES)?;
            p.input_kind = InputKind::NAMES.iter().position(|n| *n == k).map(|i| {
                [
                    InputKind::Text,
                    InputKind::Email,
                    InputKind::Password,
                    InputKind::Number,
                    InputKind::Tel,
                    InputKind::Url,
                    InputKind::Date,
                    InputKind::Search,
                ][i]
            });
        }
        "placeholder" => p.placeholder = Some(string(a)?),
        "value" => p.value = Some(string(a)?),
        "rows" => p.rows = Some(whole(a, 1, 50)?),
        "checked" => p.checked = Some(boolean(a)?),
        "selected" => p.selected = Some(boolean(a)?),
        other => unreachable!("attribute `{other}` passed validation but has no handler"),
    }
    Ok(())
}

// ---- tree walk -------------------------------------------------------------

/// Everything the analyzer needs: the document header, styles from the
/// document and its imports, and component-expanded top-level nodes.
pub struct Input<'a> {
    pub version: &'a str,
    pub version_span: Span,
    pub styles: Vec<&'a ast::StyleDef>,
    pub nodes: &'a [ast::Node],
}

/// Where we are in the tree.
#[derive(Debug, Clone, Copy, Default)]
struct Ctx {
    /// Inside a link, button or linked container.
    interactive: bool,
    in_form: bool,
}

pub fn analyze(input: Input<'_>, mut errors: Vec<Diagnostic>) -> Result<Document, Vec<Diagnostic>> {
    let mut an = Analyzer {
        dialect: Dialect::of_version(input.version),
        styles: HashMap::new(),
        errors: Vec::new(),
        ids: HashMap::new(),
        anchor_links: Vec::new(),
    };
    an.errors.append(&mut errors);

    if !SUPPORTED_VERSIONS.contains(&input.version) {
        an.errors.push(
            Diagnostic::new(
                input.version_span,
                format!("unsupported SLML version `{}`", input.version),
            )
            .with_hint("this build understands `slml 0.1` and `slml 0.2`"),
        );
    }

    an.register_styles(&input.styles);

    let mut meta = Meta::default();
    let mut page: Option<Container> = None;
    let mut seen_meta = false;
    for node in input.nodes {
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
                    page = Some(an.container(node, Kind::Page, Ctx::default()));
                }
            }
            Some(_) => an.errors.push(
                Diagnostic::new(
                    node.span,
                    format!("`{}` must be inside `page`", shown(node)),
                )
                .with_hint(
                    "the top level may only contain `meta`, `use`, `style`, `define` and `page`",
                ),
            ),
            None => {}
        }
    }

    let page = page.unwrap_or_else(|| {
        an.errors.push(
            Diagnostic::new(input.version_span, "document has no `page`")
                .with_hint("add `page { text \"Hello\" }`"),
        );
        Container {
            kind: ContainerKind::Page,
            frame: Frame::default(),
            flow: Flow::default(),
            text: TextStyle::default(),
            children: Vec::new(),
        }
    });

    for (target, span) in std::mem::take(&mut an.anchor_links) {
        if !an.ids.contains_key(&target) {
            let names: Vec<&str> = an.ids.keys().map(String::as_str).collect();
            let mut d = Diagnostic::new(span, format!("no element has `id: \"{target}\"`"));
            if let Some(s) = suggest(&target, &names) {
                d = d.with_hint(format!("did you mean `#{s}`?"));
            }
            an.errors.push(d);
        }
    }

    if an.errors.is_empty() {
        Ok(Document {
            version: input.version.to_string(),
            meta,
            page,
        })
    } else {
        an.errors.sort_by_key(|d| (d.span.file, d.span.start));
        an.errors.dedup();
        Err(an.errors)
    }
}

struct Analyzer<'a> {
    dialect: Dialect,
    styles: HashMap<&'a str, &'a ast::StyleDef>,
    errors: Vec<Diagnostic>,
    ids: HashMap<String, Span>,
    /// `#id` links to check once all ids are known.
    anchor_links: Vec<(String, Span)>,
}

impl<'a> Analyzer<'a> {
    fn err(&mut self, span: Span, msg: impl Into<String>) {
        self.errors.push(Diagnostic::new(span, msg));
    }

    fn register_styles(&mut self, defs: &[&'a ast::StyleDef]) {
        let known = all_attr_names();
        for def in defs {
            if let Some(prev) = self.styles.insert(&def.name, def) {
                self.errors.push(
                    Diagnostic::new(def.span, format!("style `{}` is defined twice", def.name))
                        .with_hint(format!(
                            "the first definition is on line {}",
                            prev.span.line
                        )),
                );
            }
            let mut seen = HashSet::new();
            for a in &def.attrs {
                if !known.contains(&a.name.as_str()) {
                    let mut d =
                        Diagnostic::new(a.span, format!("unknown attribute `{}`", shown_attr(a)));
                    let keys = if self.dialect == Dialect::Compact {
                        names::all_attr_keys()
                    } else {
                        known.clone()
                    };
                    if let Some(s) = suggest(shown_attr(a), &keys) {
                        d = d.with_hint(format!("did you mean `{s}`?"));
                    }
                    self.errors.push(d);
                } else if !seen.insert(a.name.as_str()) {
                    self.err(a.span, format!("`{}` is set twice", shown_attr(a)));
                }
                if let Value::Param(p) = &a.value {
                    self.err(a.value_span, format!("`${p}` cannot be used in a style"));
                }
            }
        }
    }

    fn lookup(&mut self, node: &ast::Node) -> Option<Kind> {
        if let Some((_, k)) = ELEMENTS.iter().find(|(n, _)| *n == node.name) {
            return Some(*k);
        }
        let word = shown(node);
        let mut d = Diagnostic::new(node.span, format!("unknown element `{word}`"));
        let compact = self.dialect == Dialect::Compact;
        let candidates = if compact {
            names::all_elements()
        } else {
            element_names()
        };
        let html = if compact {
            html_equivalent_compact(word)
        } else {
            html_equivalent(word)
        };
        if matches!(word, "style" | "define" | "use") {
            d = d.with_hint(format!("`{word}` is only allowed at the top level"));
        } else if let Some(s) = suggest(word, &candidates) {
            d = d.with_hint(format!("did you mean `{s}`?"));
        } else if let Some(k) = html {
            d = d.with_hint(format!("in SLML this is `{k}`"));
        } else if let Some((short, _)) = names::ELEMENTS
            .iter()
            .chain(names::SYMBOLS)
            .find(|(_, long)| compact && *long == word)
        {
            d = d.with_hint(format!("in `slml 0.2` this is `{short}`"));
        } else if compact {
            d = d.with_hint("components must be defined with `@name(…){…}` before they are used");
        } else {
            d = d.with_hint("components must be defined with `define` before they can be used");
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
                                shown_attr(a),
                                shown(node)
                            ),
                        )
                        .with_hint(format!(
                            "`{}` is set on line {}",
                            shown_attr(a),
                            a.span.line
                        )),
                    );
                } else if !matches!(a.value, Value::Param(_)) {
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
                self.err(a.span, format!("`{}` is set twice", shown_attr(a)));
                continue;
            }
            if !kind.accepts(&a.name) {
                let mut d = Diagnostic::new(
                    a.span,
                    format!("`{}` has no attribute `{}`", shown(node), shown_attr(a)),
                );
                let mut own = kind.attr_names();
                if self.dialect == Dialect::Compact {
                    own = own.into_iter().map(names::attr_compact).collect();
                    own.dedup();
                }
                if let Some(s) = suggest(shown_attr(a), &own) {
                    d = d.with_hint(format!("did you mean `{s}`?"));
                } else if !own.is_empty() {
                    d = d.with_hint(format!("`{}` accepts: {}", shown(node), own.join(", ")));
                } else {
                    d = d.with_hint(format!("`{}` takes no attributes", shown(node)));
                }
                self.errors.push(d);
                continue;
            }
            p.given.insert(a.name.clone());
            if let Err(e) = apply_attr(&mut p, kind, a) {
                self.errors.push(e);
            }
        }
        if let Some(id) = &p.id
            && let Some(prev) = self.ids.insert(id.clone(), p.id_span)
        {
            self.errors.push(
                Diagnostic::new(p.id_span, format!("id `{id}` is used twice")).with_hint(format!(
                    "ids must be unique; the first one is on line {}",
                    prev.line
                )),
            );
        }
        if let Some(to) = &p.to
            && let Some(fragment) = to.strip_prefix('#')
        {
            self.anchor_links.push((fragment.to_string(), p.to_span));
        }
        p
    }

    /// Reports a missing required attribute. An attribute that was written
    /// with an invalid value has already been reported, so it is not
    /// reported again.
    fn require<T>(
        &mut self,
        value: Option<T>,
        p: &Props,
        node: &ast::Node,
        attr: &str,
        hint: &str,
    ) -> Option<T> {
        if value.is_none() && !p.given.contains(attr) {
            let hint = match self.dialect {
                Dialect::Compact => compact_require_hint(shown(node), attr).unwrap_or(hint),
                Dialect::Long => hint,
            };
            self.errors.push(
                Diagnostic::new(node.span, format!("`{}` requires `{attr}`", shown(node)))
                    .with_hint(hint.to_string()),
            );
        }
        value
    }

    fn no_children(&mut self, node: &ast::Node, hint: &str) {
        if let Some(first) = node.children.first() {
            let span = match first {
                Child::Node(n) => n.span,
                Child::Text(_, s) | Child::Param(_, s) => *s,
            };
            self.errors.push(
                Diagnostic::new(span, format!("`{}` cannot have content", shown(node)))
                    .with_hint(hint.to_string()),
            );
        }
    }

    /// Links, buttons and fields cannot be placed inside each other.
    fn check_interactive(&mut self, node: &ast::Node, ctx: Ctx, is_interactive: bool) {
        if ctx.interactive && is_interactive {
            self.err(
                node.span,
                format!("`{}` cannot be placed inside a link or button", shown(node)),
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
            icon: p.icon,
        }
    }

    fn container(&mut self, node: &ast::Node, kind: Kind, ctx: Ctx) -> Container {
        let p = self.props(node, kind);
        let mut inner = ctx;
        // `to` on a form is where it is sent; anywhere else it makes the
        // container a link.
        if p.to.is_some() && kind != Kind::Form {
            self.check_interactive(node, ctx, true);
            inner.interactive = true;
        }
        let ckind = match kind {
            Kind::Page => ContainerKind::Page,
            Kind::Row => ContainerKind::Row,
            Kind::Grid => {
                if p.columns.is_none()
                    && p.cell_width.is_none()
                    && !p.given.contains("columns")
                    && !p.given.contains("cell-width")
                {
                    self.errors.push(
                        Diagnostic::new(node.span, "`grid` requires `columns` or `cell-width`")
                            .with_hint("e.g. `grid(columns: 3)` or `grid(cell-width: 240)` for a responsive grid"),
                    );
                }
                ContainerKind::Grid(GridSpec {
                    columns: p.columns,
                    cell_width: p.cell_width,
                })
            }
            Kind::Form => {
                if ctx.in_form {
                    self.err(node.span, "forms cannot be nested");
                }
                self.check_interactive(node, ctx, true);
                inner.in_form = true;
                let to = self.require(
                    p.to.clone(),
                    &p,
                    node,
                    "to",
                    "where the form is sent, e.g. `form(to: \"https://formspree.io/f/abc\", method: post)`",
                );
                ContainerKind::Form(FormSpec {
                    to: to.unwrap_or_default(),
                    method: p.method.unwrap_or(Method::Get),
                })
            }
            _ => ContainerKind::Column,
        };
        let (frame, flow, text) = match kind {
            Kind::Page => (
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
                // to a backend's taste.
                p.text_or(TextStyle {
                    color: Some(Color(0x1b1b1fff)),
                    size: Some(16.0),
                    bold: Some(false),
                    italic: Some(false),
                    mono: Some(false),
                    underline: Some(false),
                }),
            ),
            Kind::Form => (
                p.frame(Frame::default()),
                p.flow(Flow {
                    gap: 16.0,
                    ..Flow::default()
                }),
                p.text_or(TextStyle::default()),
            ),
            _ => (
                p.frame(Frame::default()),
                p.flow(Flow::default()),
                p.text_or(TextStyle::default()),
            ),
        };
        // `to` on a form is its action, not a link around it.
        let frame = if kind == Kind::Form {
            Frame {
                link: None,
                ..frame
            }
        } else {
            frame
        };

        let mut children = Vec::new();
        for child in &node.children {
            match child {
                Child::Text(_, span) | Child::Param(_, span) => self.errors.push(
                    Diagnostic::new(
                        *span,
                        format!("text cannot be placed directly in `{}`", shown(node)),
                    )
                    .with_hint("wrap it in an element: `text \"...\"`"),
                ),
                Child::Node(n) => {
                    if let Some(c) = self.block(n, shown(node), inner) {
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
    fn recover(&mut self, n: &ast::Node, ctx: Ctx) {
        let guess = match self.dialect {
            Dialect::Compact => suggest(shown(n), &names::all_elements()).and_then(names::element),
            Dialect::Long => suggest(shown(n), &element_names())
                .or_else(|| html_equivalent(shown(n)).map(|h| h.split('(').next().unwrap_or(h))),
        }
        .and_then(|name| ELEMENTS.iter().find(|(e, _)| *e == name))
        .map(|(_, k)| *k);
        let Some(k) = guess else { return };
        let ctx = Ctx {
            in_form: true,
            ..ctx
        };
        match k.category() {
            Category::Container => {
                self.container(n, k, ctx);
            }
            Category::List => {
                self.list(n, ctx);
            }
            Category::TextBlock => {
                self.text_block(n, k, ctx);
            }
            Category::Leaf => {
                self.leaf(n, k, ctx);
            }
            Category::Field => {
                self.field(n, k, ctx);
            }
            Category::Inline => {
                self.inlines(&n.children, shown(n), ctx);
            }
            Category::Meta | Category::Option => {}
        }
    }

    fn block(&mut self, n: &ast::Node, parent: &str, ctx: Ctx) -> Option<Node> {
        let Some(kind) = self.lookup(n) else {
            self.recover(n, ctx);
            return None;
        };
        match kind.category() {
            Category::Container if kind != Kind::Page => {
                Some(Node::Container(self.container(n, kind, ctx)))
            }
            Category::List => Some(Node::List(self.list(n, ctx))),
            Category::TextBlock if kind == Kind::Item => {
                self.errors.push(
                    Diagnostic::new(n.span, "`item` can only be placed inside `list`")
                        .with_hint("e.g. `list { item \"one\" item \"two\" }`"),
                );
                None
            }
            Category::TextBlock => Some(Node::Text(self.text_block(n, kind, ctx))),
            Category::Leaf => Some(self.leaf(n, kind, ctx)),
            Category::Field => Some(Node::Field(self.field(n, kind, ctx))),
            Category::Inline => {
                self.errors.push(
                    Diagnostic::new(
                        n.span,
                        format!("`{}` cannot be placed directly in `{parent}`", shown(n)),
                    )
                    .with_hint(format!(
                        "inline elements go inside text: `text {{ {} \"...\" }}`",
                        n.name
                    )),
                );
                None
            }
            Category::Option => {
                self.err(n.span, "`option` can only be placed inside `select`");
                None
            }
            _ => {
                self.err(
                    n.span,
                    format!("`{}` can only appear at the top level", shown(n)),
                );
                None
            }
        }
    }

    fn list(&mut self, n: &ast::Node, ctx: Ctx) -> List {
        let p = self.props(n, Kind::List);
        let mut items = Vec::new();
        for child in &n.children {
            match child {
                Child::Text(_, span) | Child::Param(_, span) => self.errors.push(
                    Diagnostic::new(*span, "text cannot be placed directly in `list`")
                        .with_hint("write each entry as `item \"...\"`"),
                ),
                Child::Node(c) => match self.lookup(c) {
                    Some(Kind::Item) => {
                        items.push(ListEntry::Item(self.text_block(c, Kind::Item, ctx)))
                    }
                    Some(Kind::List) => {
                        if items.is_empty() {
                            self.err(c.span, "a nested list must follow an `item`");
                        }
                        items.push(ListEntry::List(self.list(c, ctx)));
                    }
                    Some(_) => self.errors.push(
                        Diagnostic::new(
                            c.span,
                            format!("`{}` cannot be placed inside `list`", shown(c)),
                        )
                        .with_hint("a list contains `item`s and nested `list`s"),
                    ),
                    None => self.recover(c, ctx),
                },
            }
        }
        List {
            frame: p.frame(Frame::default()),
            text: p.text_or(TextStyle::default()),
            gap: p.gap.unwrap_or(6.0),
            marker: p.marker.unwrap_or(Marker::Bullet),
            items,
        }
    }

    fn text_block(&mut self, n: &ast::Node, kind: Kind, ctx: Ctx) -> TextBlock {
        let p = self.props(n, kind);
        let mut inner = ctx;
        let button_frame = || Frame {
            padding_x: 18.0,
            padding_y: 10.0,
            fill: Some(Color(0x2457d6ff)),
            radius: 8.0,
            ..Frame::default()
        };
        let button_text = TextStyle {
            color: Some(Color::WHITE),
            underline: Some(false),
            ..TextStyle::default()
        };
        let (tkind, frame, text) = match kind {
            Kind::Heading => {
                let level = p.level.unwrap_or(1);
                let size = match level {
                    1 => 32.0,
                    2 => 24.0,
                    3 => 20.0,
                    _ => 17.0,
                };
                (
                    TextKind::Heading(level),
                    p.frame(Frame::default()),
                    p.text_or(TextStyle {
                        size: Some(size),
                        bold: Some(true),
                        ..TextStyle::default()
                    }),
                )
            }
            Kind::Button => {
                self.check_interactive(n, ctx, true);
                inner.interactive = true;
                self.require(
                    p.to.clone(),
                    &p,
                    n,
                    "to",
                    "e.g. `button(to: \"contact.slml\") \"Contact\"`",
                );
                (
                    TextKind::Button,
                    p.frame(button_frame()),
                    p.text_or(button_text),
                )
            }
            Kind::Submit => {
                if !ctx.in_form {
                    self.err(n.span, "`submit` must be inside a `form`");
                }
                self.check_interactive(n, ctx, true);
                inner.interactive = true;
                (
                    TextKind::Submit,
                    p.frame(button_frame()),
                    p.text_or(button_text),
                )
            }
            Kind::Codeblock => (
                TextKind::Code,
                p.frame(Frame {
                    padding_x: 16.0,
                    padding_y: 14.0,
                    fill: Some(Color(0xf4f4f7ff)),
                    radius: 8.0,
                    ..Frame::default()
                }),
                p.text_or(TextStyle {
                    mono: Some(true),
                    size: Some(14.0),
                    ..TextStyle::default()
                }),
            ),
            Kind::Item => (
                TextKind::Item,
                p.frame(Frame::default()),
                p.text_or(TextStyle::default()),
            ),
            _ => (
                TextKind::Paragraph,
                p.frame(Frame::default()),
                p.text_or(TextStyle::default()),
            ),
        };
        let align = p
            .text_align
            .unwrap_or(if matches!(kind, Kind::Button | Kind::Submit) {
                TextAlign::Center
            } else {
                TextAlign::Start
            });
        let content = self.inlines(&n.children, shown(n), inner);
        TextBlock {
            kind: tkind,
            frame,
            text,
            align,
            content,
        }
    }

    fn inlines(&mut self, children: &[Child], parent: &str, ctx: Ctx) -> Vec<Inline> {
        let mut out = Vec::new();
        for child in children {
            match child {
                // Adjacent strings are one run of text, whichever way they
                // were written (`"a " "b"` or `"a b"`).
                Child::Text(s, _) => match out.last_mut() {
                    Some(Inline::Text(prev)) => prev.push_str(s),
                    _ => out.push(Inline::Text(s.clone())),
                },
                Child::Param(_, span) => {
                    self.err(*span, "parameters can only be used inside `define`")
                }
                Child::Node(n) => {
                    let Some(kind) = self.lookup(n) else {
                        self.recover(n, ctx);
                        continue;
                    };
                    if kind.category() != Category::Inline {
                        self.errors.push(
                            Diagnostic::new(
                                n.span,
                                format!("`{}` cannot be placed inside `{parent}`", shown(n)),
                            )
                            .with_hint(
                                "text may only contain strings and bold, italic, code, link, span",
                            ),
                        );
                        continue;
                    }
                    let mut inner = ctx;
                    if kind == Kind::Link {
                        self.check_interactive(n, ctx, true);
                        inner.interactive = true;
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
                            &p,
                            n,
                            "to",
                            "e.g. `link(to: \"about.slml\") \"About\"`",
                        )
                    } else {
                        None
                    };
                    let content = self.inlines(&n.children, shown(n), inner);
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

    fn leaf(&mut self, n: &ast::Node, kind: Kind, ctx: Ctx) -> Node {
        let p = self.props(n, kind);
        match kind {
            Kind::Image => {
                self.no_children(n, "describe the image with `alt: \"...\"` instead");
                if p.to.is_some() {
                    self.check_interactive(n, ctx, true);
                }
                let src = self.require(
                    p.src.clone(),
                    &p,
                    n,
                    "src",
                    "e.g. `image(src: \"logo.png\", alt: \"Logo\")`",
                );
                let alt = self.require(
                    p.alt.clone(),
                    &p,
                    n,
                    "alt",
                    "describe the image for people who cannot see it; use `alt: \"\"` only for decoration",
                );
                if p.to.is_some() && alt.as_deref() == Some("") {
                    self.err(
                        n.span,
                        "a linked image needs a non-empty `alt`: it is the link's text",
                    );
                }
                Node::Image(Image {
                    frame: p.frame(Frame::default()),
                    src: src.unwrap_or_default(),
                    alt: alt.unwrap_or_default(),
                })
            }
            Kind::Divider => {
                self.no_children(n, "a divider is just a line");
                Node::Divider(Divider {
                    id: p.id.clone(),
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

    fn field(&mut self, n: &ast::Node, kind: Kind, ctx: Ctx) -> Field {
        let p = self.props(n, kind);
        if !ctx.in_form {
            self.errors.push(
                Diagnostic::new(n.span, format!("`{}` must be inside a `form`", shown(n)))
                    .with_hint(
                        "e.g. `form(to: \"...\") { input(name: \"email\", label: \"E-mail\") }`",
                    ),
            );
        }
        self.check_interactive(n, ctx, true);
        let name = self.require(
            p.name.clone(),
            &p,
            n,
            "name",
            "the name the value is sent under, e.g. `name: \"email\"`",
        );
        let label = self.require(
            p.label.clone(),
            &p,
            n,
            "label",
            "every field needs a visible label, e.g. `label: \"Your e-mail\"`",
        );
        let field_kind = match kind {
            Kind::Input => {
                self.no_children(n, "use `value: \"...\"` or `placeholder: \"...\"`");
                FieldKind::Input {
                    kind: p.input_kind.unwrap_or(InputKind::Text),
                    placeholder: p.placeholder.clone(),
                    value: p.value.clone(),
                }
            }
            Kind::Textarea => {
                self.no_children(n, "use `value: \"...\"` or `placeholder: \"...\"`");
                FieldKind::TextArea {
                    rows: p.rows.unwrap_or(4),
                    placeholder: p.placeholder.clone(),
                    value: p.value.clone(),
                }
            }
            Kind::Checkbox => {
                self.no_children(n, "the checkbox text is its `label`");
                FieldKind::Checkbox {
                    checked: p.checked.unwrap_or(false),
                }
            }
            _ => {
                let mut options = Vec::new();
                for child in &n.children {
                    match child {
                        Child::Node(o) if o.name == "option" => {
                            let op = self.props(o, Kind::Option);
                            let label = match o.children.as_slice() {
                                [Child::Text(s, _)] if !s.trim().is_empty() => s.clone(),
                                _ => {
                                    self.errors.push(
                                        Diagnostic::new(o.span, "`option` needs a text label")
                                            .with_hint("e.g. `option \"Poland\"`"),
                                    );
                                    String::new()
                                }
                            };
                            options.push(SelectOption {
                                label,
                                value: op.value,
                                selected: op.selected.unwrap_or(false),
                            });
                        }
                        Child::Node(o) => self.errors.push(
                            Diagnostic::new(
                                o.span,
                                format!("`{}` cannot be placed inside `select`", shown(o)),
                            )
                            .with_hint("a select contains `option \"...\"` entries"),
                        ),
                        Child::Text(_, span) | Child::Param(_, span) => self.errors.push(
                            Diagnostic::new(*span, "text cannot be placed directly in `select`")
                                .with_hint("write it as `option \"...\"`"),
                        ),
                    }
                }
                if options.is_empty() {
                    self.errors.push(
                        Diagnostic::new(n.span, "`select` needs at least one `option`")
                            .with_hint("e.g. `select(name: \"size\", label: \"Size\") { option \"S\" option \"M\" }`"),
                    );
                }
                if options.iter().filter(|o| o.selected).count() > 1 {
                    self.err(n.span, "only one `option` can be `selected`");
                }
                FieldKind::Select { options }
            }
        };
        Field {
            frame: p.frame(Frame::default()),
            text: p.text_or(TextStyle::default()),
            name: name.unwrap_or_default(),
            label: label.unwrap_or_default(),
            required: p.required.unwrap_or(false),
            kind: field_kind,
        }
    }
}

/// Hints for missing required attributes in the compact dialect.
fn compact_require_hint(element: &str, attr: &str) -> Option<&'static str> {
    Some(match (element, attr) {
        ("img", "src") => "write the path first: `img{/logo.png \"Logo\"}`",
        ("img", "alt") => {
            "describe the image for people who cannot see it: `img{/a.png \"Description\"}`; use \"\" only for decoration"
        }
        ("btn", "to") => "e.g. `btn{>/contact.slml \"Contact\"}`",
        ("form", "to") => "where the form is sent, e.g. `form{>https://formspree.io/f/abc post …}`",
        (_, "name") => "the name the value is sent under, e.g. `n:email`",
        (_, "label") => "every field needs a visible label string, e.g. `in{n:email \"E-mail\"}`",
        _ => return None,
    })
}

/// HTML names → the compact SLML spelling.
fn html_equivalent_compact(name: &str) -> Option<&'static str> {
    Some(match name {
        "div" | "section" | "main" | "article" | "body" | "header" | "footer" | "nav" | "aside" => {
            "c{…} or r{…}; SLML has no landmark elements, layout is just boxes"
        }
        "p" | "label" | "span" => "t\"…\" or just a string",
        "h1" => "#\"…\"",
        "h2" => "##\"…\"",
        "h3" => "###\"…\"",
        "h4" | "h5" | "h6" => "####\"…\"",
        "a" => "[text](target) inside text, or >target on a box",
        "hr" => "|",
        "b" | "strong" => "*bold* inside text",
        "i" | "em" => "_italic_ inside text",
        "ul" => "-{\"one\" \"two\"}",
        "ol" => "+{\"one\" \"two\"}",
        "li" => "a string inside -{…}",
        "pre" => "code\"\"\"…\"\"\"",
        "table" => "grid{cols3 …}",
        "button" => "btn{>target \"…\"}",
        "input" => "in{n:name \"Label\"}",
        "textarea" => "area{n:name \"Label\"}",
        "select" => "sel{n:name \"Label\" opt\"…\"}",
        _ => return None,
    })
}

/// Helps people coming from HTML find the SLML equivalent.
fn html_equivalent(name: &str) -> Option<&'static str> {
    Some(match name {
        "div" | "section" | "main" | "article" | "body" | "header" | "footer" | "nav" | "aside" => {
            "column"
        }
        "p" | "label" => "text",
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => "heading(level: N)",
        "a" => "link(to: \"...\")",
        "img" => "image(src: \"...\", alt: \"...\")",
        "hr" => "divider",
        "b" | "strong" => "bold",
        "i" | "em" => "italic",
        "br" => "\\n inside a string",
        "head" | "title" => "meta(title: \"...\")",
        "ul" | "ol" => "list",
        "li" => "item",
        "pre" => "codeblock",
        "table" => "grid(columns: N)",
        "fieldset" => "form",
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
    fn no_nested_interactive_elements() {
        let e = errors(r#"slml 0.1 page { button(to: "a") { link(to: "b") "x" } }"#);
        assert!(e[0].contains("inside a link"), "{e:?}");
        let e = errors(r#"slml 0.1 page { column(to: "a") { button(to: "b") "x" } }"#);
        assert!(e[0].contains("inside a link"), "{e:?}");
    }

    #[test]
    fn paths_stay_inside_the_site() {
        for bad in [
            "../secret.png",
            "a/../../b.png",
            "C:/x.png",
            "//evil.com/x.png",
        ] {
            let src = format!("slml 0.1 page {{ image(src: \"{bad}\", alt: \"x\") }}");
            assert!(errors(&src)[0].contains("invalid path"), "{bad}");
        }
        for good in ["img/a.png", "/img/a.png", "https://example.com/a.png"] {
            let src = format!("slml 0.1 page {{ image(src: \"{good}\", alt: \"x\") }}");
            assert!(compile(&src).is_ok(), "{good}");
        }
    }

    #[test]
    fn link_targets_are_checked() {
        assert!(compile(
            r##"slml 0.1 page { heading(id: "top") "x" text { link(to: "#top") "a" link(to: "mailto:a@b.c") "m" link(to: "/docs/x.slml#y") "d" } }"##
        )
        .is_ok());
        let e = errors(
            r##"slml 0.1 page { text { link(to: "#nope") "a" link(to: "javascript:alert(1)") "b" } }"##,
        );
        assert_eq!(e.len(), 2, "{e:?}");
        let e = errors(r#"slml 0.1 page { text(id: "a") "x" text(id: "a") "y" }"#);
        assert!(e[0].contains("used twice"), "{e:?}");
    }

    #[test]
    fn lists_and_grids() {
        let doc = compile(
            r#"slml 0.1 page {
              list(marker: number) { item "a" list { item "nested" } item { bold "b" } }
              grid(cell-width: 200, gap: 16) { text "1" text "2" }
            }"#,
        )
        .unwrap();
        let Node::List(l) = &doc.page.children[0] else {
            panic!()
        };
        assert_eq!(l.items.len(), 3);
        assert!(matches!(l.items[1], ListEntry::List(_)));
        let e = errors(r#"slml 0.1 page { item "x" list { text "y" } grid { } }"#);
        assert_eq!(e.len(), 3, "{e:?}");
    }

    #[test]
    fn forms_require_labels_and_context() {
        let doc = compile(
            r#"slml 0.1 page {
              form(to: "https://example.com/send", method: post) {
                input(name: "email", label: "E-mail", kind: email, required: true)
                textarea(name: "msg", label: "Message", rows: 5)
                checkbox(name: "news", label: "Newsletter")
                select(name: "size", label: "Size") { option "S" option(value: "m", selected: true) "M" }
                submit "Send"
              }
            }"#,
        )
        .unwrap();
        let Node::Container(form) = &doc.page.children[0] else {
            panic!()
        };
        assert!(matches!(
            form.kind,
            ContainerKind::Form(FormSpec {
                method: Method::Post,
                ..
            })
        ));
        assert_eq!(form.children.len(), 5);
        let e = errors(
            r#"slml 0.1 page { input(name: "a") form { select(name: "s", label: "S") { } } submit "x" }"#,
        );
        assert!(
            e.iter().any(|m| m.contains("must be inside a `form`")),
            "{e:?}"
        );
        assert!(e.iter().any(|m| m.contains("requires `label`")), "{e:?}");
        assert!(e.iter().any(|m| m.contains("requires `to`")), "{e:?}");
        assert!(
            e.iter().any(|m| m.contains("at least one `option`")),
            "{e:?}"
        );
    }
}
