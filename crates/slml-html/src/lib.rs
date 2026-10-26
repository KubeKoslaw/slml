//! # slml-html
//!
//! Compiles a validated SLML [`Document`] to semantic HTML with inline
//! styles plus one shared stylesheet. The output follows the layout rules
//! of SPEC.md §5 using CSS flexbox and grid, uses the same bundled fonts as
//! the native renderer and contains no JavaScript.
//!
//! ```
//! let doc = slml_syntax::compile(r#"slml 0.1 page { text "Hi" }"#).unwrap();
//! let html = slml_html::page(&doc, &slml_html::Options::default());
//! assert!(html.contains("<p class=\"s-text\">Hi</p>"));
//! ```

use std::fmt::Write as _;

use slml_syntax::analyze::is_external;
use slml_syntax::model::*;

/// Folder (next to the site's pages) holding the stylesheet and fonts.
pub const ASSET_DIR: &str = "_slml";
pub const STYLESHEET: &str = "slml.css";

#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Relative path from the page to the site root, e.g. `""` for a page
    /// in the root or `"../"` for a page one folder deep. Root paths (`/x`)
    /// become `prefix + x`, so the output also works from `file://` and
    /// from a sub-folder of a web server.
    pub root_prefix: String,
    /// Extra HTML placed before `</body>` (the dev server's reload script).
    pub body_end: String,
}

/// A bundled font as listed by the renderer: (file, family, weight, italic).
pub type FontFace<'a> = (&'a str, &'a str, u16, bool);

/// The shared stylesheet. `fonts` are served from `fonts/` next to it.
pub fn stylesheet(fonts: &[FontFace]) -> String {
    let mut css = String::new();
    for (file, family, weight, italic) in fonts {
        let _ = writeln!(
            css,
            "@font-face{{font-family:\"{family}\";src:url(\"fonts/{file}\") format(\"truetype\");font-weight:{weight};font-style:{};font-display:swap}}",
            if *italic { "italic" } else { "normal" }
        );
    }
    css.push_str(BASE_CSS);
    css
}

const FALLBACK_FAMILIES: &str = "\"SLML Symbols 2\",\"SLML Symbols\",\"SLML Math\",\"SLML Emoji\"";

const BASE_CSS: &str = r#"*,*::before,*::after{box-sizing:border-box}
html,body{margin:0;padding:0}
body{font-family:"SLML Sans","SLML Symbols 2","SLML Symbols","SLML Math","SLML Emoji",sans-serif;-webkit-text-size-adjust:100%;text-rendering:optimizeLegibility}
.s-page{display:flex;flex-direction:column;min-height:100vh}
.s-col{display:flex;flex-direction:column;min-width:0}
.s-row{display:flex;flex-direction:row;min-width:0}
.s-grid{display:grid;min-width:0}
.s-v>*{flex-shrink:0}
.s-text{margin:0;white-space:pre-wrap;line-height:1.45;font-size:inherit;font-weight:inherit}
.s-mono{font-family:"SLML Mono","SLML Symbols 2","SLML Symbols","SLML Math","SLML Emoji",monospace}
.s-text strong{font-weight:700}.s-text em{font-style:italic}.s-text code{font-size:inherit}
a.s-linkbox{color:inherit;text-decoration:none}
.s-button{display:flex;align-items:center;cursor:pointer;border:0;font:inherit;text-decoration:none}
.s-img{display:block;max-width:100%}
.s-list{margin:0;padding:0;list-style:none;display:flex;flex-direction:column}
.s-li{display:flex;gap:.6em;line-height:1.45}
.s-marker{flex:none;text-align:end;white-space:nowrap}
.s-li>.s-text,.s-li>.s-list{flex:1 1 0;min-width:0}
.s-divider{border:0;margin:0;flex:none}
.s-field{display:flex;flex-direction:column;gap:6px;min-width:0}
.s-label{font-weight:700;font-size:.875em;line-height:1.45}
.s-box{font:inherit;color:inherit;line-height:1.45;padding:9px 12px;border:1px solid #c4c4cc;border-radius:6px;background:#fff;width:100%;margin:0}
textarea.s-box{resize:vertical}
select.s-box{appearance:none;background-image:linear-gradient(45deg,transparent 50%,#8a8a94 50%),linear-gradient(135deg,#8a8a94 50%,transparent 50%);background-position:calc(100% - 18px) 50%,calc(100% - 13px) 50%;background-size:5px 5px;background-repeat:no-repeat;padding-right:32px}
.s-box::placeholder{color:#8a8a94}
.s-check{display:flex;align-items:flex-start;gap:.5em;line-height:1.45}
.s-check input{width:1.1em;height:1.1em;margin:.18em 0 0;accent-color:#2457d6;flex:none}
"#;

/// Escapes text for HTML content and attribute values.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

fn color(c: Color) -> String {
    if c.a() == 255 {
        format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b())
    } else {
        format!("#{:08x}", c.0)
    }
}

fn px(n: f32) -> String {
    let rounded = (n * 100.0).round() / 100.0;
    format!("{rounded}px")
}

/// Turns an SLML link target into an `href`: `.slml` pages become `.html`,
/// root paths become relative to the page.
pub fn href(to: &str, opts: &Options) -> String {
    if is_external(to) || to.starts_with('#') {
        return to.to_string();
    }
    let (path, fragment) = match to.split_once('#') {
        Some((p, f)) => (p, Some(f)),
        None => (to, None),
    };
    let mut out = asset(path, opts);
    if let Some(stem) = out.strip_suffix(".slml") {
        out = format!("{stem}.html");
    }
    if let Some(f) = fragment {
        out.push('#');
        out.push_str(f);
    }
    out
}

/// Resolves an image or icon path for the page.
pub fn asset(path: &str, opts: &Options) -> String {
    if is_external(path) {
        return path.to_string();
    }
    let (prefix, path) = match path.strip_prefix('/') {
        Some(rest) => (opts.root_prefix.as_str(), rest),
        None => ("", path),
    };
    let mut url = prefix.to_string();
    // SLML stores literal file names, not already-escaped URL paths.
    for byte in path.bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
            url.push(char::from(byte));
        } else {
            let _ = write!(url, "%{byte:02X}");
        }
    }
    url
}

/// Compiles a whole page to an HTML document.
pub fn page(doc: &Document, opts: &Options) -> String {
    let mut w = Writer {
        out: String::new(),
        opts,
        depth: 0,
    };
    let m = &doc.meta;
    w.out.push_str("<!doctype html>\n");
    let _ = writeln!(
        w.out,
        "<html lang=\"{}\">",
        escape(m.lang.as_deref().unwrap_or("en"))
    );
    w.out.push_str("<head>\n<meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    let _ = writeln!(
        w.out,
        "<title>{}</title>",
        escape(m.title.as_deref().unwrap_or("Untitled"))
    );
    if let Some(d) = &m.description {
        let _ = writeln!(
            w.out,
            "<meta name=\"description\" content=\"{}\">",
            escape(d)
        );
    }
    if let Some(icon) = &m.icon {
        let _ = writeln!(
            w.out,
            "<link rel=\"icon\" href=\"{}\">",
            escape(&asset(icon, opts))
        );
    }
    let _ = writeln!(
        w.out,
        "<meta name=\"generator\" content=\"SLML {}\">\n<link rel=\"stylesheet\" href=\"{}{ASSET_DIR}/{STYLESHEET}\">\n</head>",
        escape(&doc.version),
        opts.root_prefix
    );
    let bg = doc.page.frame.fill.unwrap_or(Color::WHITE);
    let _ = writeln!(w.out, "<body style=\"background:{}\">", color(bg));
    w.page(&doc.page);
    w.out.push_str(&opts.body_end);
    w.out.push_str("</body>\n</html>\n");
    w.out
}

/// HTML attributes; `None` values are left out.
type Attrs<'a> = Vec<(&'a str, Option<String>)>;

/// What a child's parent is, which decides how sizes translate to CSS.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Parent {
    /// Vertical flex container; `true` when its `align` is `stretch`.
    Column(bool),
    Row,
    Grid,
    ListItem,
}

struct Writer<'a> {
    out: String,
    opts: &'a Options,
    depth: usize,
}

impl Writer<'_> {
    fn indent(&mut self) {
        for _ in 0..self.depth {
            self.out.push_str("  ");
        }
    }

    fn page(&mut self, c: &Container) {
        let mut style = Vec::new();
        frame_css(&c.frame, &mut style, true);
        flow_css(&c.flow, &mut style, false);
        text_css(&c.text, &mut style);
        self.open(
            "main",
            &["s-page", "s-col", "s-v"],
            &style,
            &[("id", c.frame.id.clone())],
        );
        self.children(c);
        self.close("main");
    }

    fn open(
        &mut self,
        tag: &str,
        classes: &[&str],
        style: &[String],
        attrs: &[(&str, Option<String>)],
    ) {
        self.indent();
        let _ = write!(self.out, "<{tag}");
        if !classes.is_empty() {
            let _ = write!(self.out, " class=\"{}\"", classes.join(" "));
        }
        for (name, value) in attrs {
            if let Some(v) = value {
                let _ = write!(self.out, " {name}=\"{}\"", escape(v));
            }
        }
        if !style.is_empty() {
            let _ = write!(self.out, " style=\"{}\"", escape(&style.join(";")));
        }
        self.out.push_str(">\n");
        self.depth += 1;
    }

    fn close(&mut self, tag: &str) {
        self.depth -= 1;
        self.indent();
        let _ = writeln!(self.out, "</{tag}>");
    }

    fn children(&mut self, c: &Container) {
        let parent = match &c.kind {
            ContainerKind::Row => Parent::Row,
            ContainerKind::Grid(_) => Parent::Grid,
            _ => Parent::Column(c.flow.align == Align::Stretch),
        };
        for child in &c.children {
            self.node(child, parent, c.flow.align);
        }
    }

    fn node(&mut self, n: &Node, parent: Parent, parent_align: Align) {
        match n {
            Node::Container(c) => self.container(c, parent),
            Node::Text(t) => self.text_block(t, parent),
            Node::Image(i) => self.image(i, parent, parent_align),
            Node::List(l) => self.list(l, parent),
            Node::Field(f) => self.field(f, parent),
            Node::Divider(d) => {
                let mut style = vec![format!("background:{}", color(d.color))];
                match parent {
                    Parent::Row => {
                        style.push(format!("width:{}", px(d.thickness)));
                        style.push("align-self:stretch".into());
                    }
                    _ => {
                        style.push(format!("height:{}", px(d.thickness)));
                        style.push("align-self:stretch".into());
                        if let Size::Px(w) = d.width {
                            style.push(format!("width:{}", px(w)));
                            style.push("max-width:100%".into());
                            style.push("align-self:flex-start".into());
                        }
                    }
                }
                self.indent();
                let id =
                    d.id.as_ref()
                        .map(|i| format!(" id=\"{}\"", escape(i)))
                        .unwrap_or_default();
                let _ = writeln!(
                    self.out,
                    "<hr class=\"s-divider\"{id} style=\"{}\">",
                    escape(&style.join(";"))
                );
            }
            Node::Spacer(s) => {
                let mut style = Vec::new();
                let main = if parent == Parent::Row {
                    s.width
                } else {
                    s.height
                };
                match main {
                    Size::Px(p) => style.push(format!("flex:0 0 {}", px(p))),
                    _ => style.push("flex:1 1 0".into()),
                }
                if let (Parent::Row, Size::Px(h)) = (parent, s.height) {
                    style.push(format!("height:{}", px(h)));
                }
                if let (Parent::Column(_), Size::Px(w)) = (parent, s.width) {
                    style.push(format!("width:{}", px(w)));
                }
                self.indent();
                let _ = writeln!(
                    self.out,
                    "<div aria-hidden=\"true\" style=\"{}\"></div>",
                    escape(&style.join(";"))
                );
            }
        }
    }

    fn container(&mut self, c: &Container, parent: Parent) {
        let mut style = Vec::new();
        sizing_css(&c.frame, parent, &mut style, None);
        frame_css(&c.frame, &mut style, false);
        text_css(&c.text, &mut style);
        let (tag, mut classes, mut attrs): (&str, Vec<&str>, Attrs) = match &c.kind {
            ContainerKind::Row => {
                flow_css(&c.flow, &mut style, true);
                if c.flow.wrap {
                    style.push("flex-wrap:wrap".into());
                }
                ("div", vec!["s-row"], vec![])
            }
            ContainerKind::Grid(spec) => {
                style.push(format!(
                    "grid-template-columns:{}",
                    grid_columns(spec, c.flow.gap)
                ));
                if c.flow.gap > 0.0 {
                    style.push(format!("gap:{}", px(c.flow.gap)));
                }
                style.push(format!("align-items:{}", align_css(c.flow.align)));
                ("div", vec!["s-grid"], vec![])
            }
            ContainerKind::Form(f) => {
                flow_css(&c.flow, &mut style, false);
                let method = if f.method == Method::Post {
                    "post"
                } else {
                    "get"
                };
                (
                    "form",
                    vec!["s-col", "s-v"],
                    vec![
                        ("action", Some(href(&f.to, self.opts))),
                        ("method", Some(method.to_string())),
                    ],
                )
            }
            _ => {
                flow_css(&c.flow, &mut style, false);
                ("div", vec!["s-col", "s-v"], vec![])
            }
        };
        let tag = if let Some(to) = &c.frame.link {
            attrs.push(("href", Some(href(to, self.opts))));
            classes.push("s-linkbox");
            "a"
        } else {
            tag
        };
        attrs.push(("id", c.frame.id.clone()));
        self.open(tag, &classes, &style, &attrs);
        self.children(c);
        self.close(tag);
    }

    fn text_block(&mut self, t: &TextBlock, parent: Parent) {
        let mut style = Vec::new();
        sizing_css(&t.frame, parent, &mut style, None);
        frame_css(&t.frame, &mut style, false);
        text_css(&t.text, &mut style);
        let (tag, mut classes): (String, Vec<&str>) = match t.kind {
            TextKind::Heading(level) => (format!("h{level}"), vec!["s-text"]),
            TextKind::Code => ("pre".into(), vec!["s-text"]),
            TextKind::Button => ("a".into(), vec!["s-text", "s-button"]),
            TextKind::Submit => ("button".into(), vec!["s-text", "s-button"]),
            TextKind::Item => ("div".into(), vec!["s-text"]),
            TextKind::Paragraph => ("p".into(), vec!["s-text"]),
        };
        if t.text.mono == Some(true) {
            classes.push("s-mono");
        }
        let align = match t.align {
            TextAlign::Start => None,
            TextAlign::Center => Some("center"),
            TextAlign::End => Some("end"),
        };
        let is_button = matches!(t.kind, TextKind::Button | TextKind::Submit);
        if let Some(a) = align {
            style.push(format!("text-align:{a}"));
        }
        if is_button {
            style.push(format!(
                "justify-content:{}",
                match t.align {
                    TextAlign::Start => "flex-start",
                    TextAlign::Center => "center",
                    TextAlign::End => "flex-end",
                }
            ));
        }
        let mut attrs: Vec<(&str, Option<String>)> = vec![("id", t.frame.id.clone())];
        if t.kind == TextKind::Button
            && let Some(to) = &t.frame.link
        {
            attrs.push(("href", Some(href(to, self.opts))));
        }
        if t.kind == TextKind::Submit {
            attrs.push(("type", Some("submit".into())));
        }
        self.indent();
        let _ = write!(self.out, "<{tag} class=\"{}\"", classes.join(" "));
        for (name, value) in &attrs {
            if let Some(v) = value {
                let _ = write!(self.out, " {name}=\"{}\"", escape(v));
            }
        }
        if !style.is_empty() {
            let _ = write!(self.out, " style=\"{}\"", escape(&style.join(";")));
        }
        self.out.push('>');
        if is_button {
            self.out.push_str("<span>");
        }
        self.inlines(&t.content);
        if is_button {
            self.out.push_str("</span>");
        }
        let _ = writeln!(self.out, "</{tag}>");
    }

    fn inlines(&mut self, content: &[Inline]) {
        for inline in content {
            match inline {
                Inline::Text(s) => self.out.push_str(&escape(s)),
                Inline::Span(span) => {
                    let mut style = Vec::new();
                    text_css(&span.text, &mut style);
                    let tag = if span.link.is_some() {
                        "a"
                    } else if span.text.mono == Some(true) {
                        "code"
                    } else if span.text.bold == Some(true) && style.len() == 1 {
                        "strong"
                    } else if span.text.italic == Some(true) && style.len() == 1 {
                        "em"
                    } else {
                        "span"
                    };
                    let _ = write!(self.out, "<{tag}");
                    if span.text.mono == Some(true) {
                        self.out.push_str(" class=\"s-mono\"");
                    }
                    if let Some(to) = &span.link {
                        let _ = write!(self.out, " href=\"{}\"", escape(&href(to, self.opts)));
                    }
                    if !style.is_empty() && !matches!(tag, "strong" | "em") {
                        let _ = write!(self.out, " style=\"{}\"", escape(&style.join(";")));
                    }
                    self.out.push('>');
                    self.inlines(&span.content);
                    let _ = write!(self.out, "</{tag}>");
                }
            }
        }
    }

    fn image(&mut self, img: &Image, parent: Parent, parent_align: Align) {
        let f = &img.frame;
        let mut outer = Vec::new();
        sizing_css(f, parent, &mut outer, Some(parent_align));
        let mut style = Vec::new();
        frame_css(f, &mut style, false);
        match (f.width, f.height) {
            (Size::Px(_), Size::Px(h)) => style.push(format!("height:{}", px(h))),
            (_, Size::Px(h)) => {
                style.push(format!("height:{}", px(h)));
                style.push("width:auto".into());
            }
            _ => style.push("height:auto".into()),
        }
        if parent == Parent::Grid || f.width == Size::Fill {
            style.push("width:100%".into());
        }
        let src = asset(&img.src, self.opts);
        let alt = escape(&img.alt);
        match &f.link {
            Some(to) => {
                // The link is the layout item; the image fills it.
                outer.push("display:block".into());
                self.indent();
                let _ = write!(
                    self.out,
                    "<a class=\"s-linkbox\" href=\"{}\"",
                    escape(&href(to, self.opts))
                );
                if let Some(id) = &f.id {
                    let _ = write!(self.out, " id=\"{}\"", escape(id));
                }
                let _ = write!(self.out, " style=\"{}\">", escape(&outer.join(";")));
                style.push("width:100%".into());
                let _ = writeln!(
                    self.out,
                    "<img class=\"s-img\" src=\"{}\" alt=\"{alt}\" style=\"{}\"></a>",
                    escape(&src),
                    escape(&style.join(";"))
                );
            }
            None => {
                outer.extend(style);
                self.indent();
                let id =
                    f.id.as_ref()
                        .map(|i| format!(" id=\"{}\"", escape(i)))
                        .unwrap_or_default();
                let _ = writeln!(
                    self.out,
                    "<img class=\"s-img\"{id} src=\"{}\" alt=\"{alt}\" style=\"{}\">",
                    escape(&src),
                    escape(&outer.join(";"))
                );
            }
        }
    }

    fn list(&mut self, l: &List, parent: Parent) {
        let mut style = Vec::new();
        sizing_css(&l.frame, parent, &mut style, None);
        frame_css(&l.frame, &mut style, false);
        text_css(&l.text, &mut style);
        if l.gap > 0.0 {
            style.push(format!("gap:{}", px(l.gap)));
        }
        let tag = if l.marker == Marker::Number {
            "ol"
        } else {
            "ul"
        };
        self.open(tag, &["s-list"], &style, &[("id", l.frame.id.clone())]);
        let count = l
            .items
            .iter()
            .filter(|e| matches!(e, ListEntry::Item(_)))
            .count();
        let marker_ch = match l.marker {
            Marker::Number => count.to_string().len() + 1,
            Marker::Bullet => 1,
            Marker::None => 0,
        };
        let marker_style = if marker_ch > 0 {
            format!(" style=\"min-width:{marker_ch}ch\"")
        } else {
            " style=\"display:none\"".to_string()
        };
        let mut n = 0;
        for entry in &l.items {
            match entry {
                ListEntry::Item(t) => {
                    n += 1;
                    let marker = match l.marker {
                        Marker::Bullet => "•".to_string(),
                        Marker::Number => format!("{n}."),
                        Marker::None => String::new(),
                    };
                    self.indent();
                    let _ = writeln!(
                        self.out,
                        "<li class=\"s-li\"><span class=\"s-marker\" aria-hidden=\"true\"{marker_style}>{marker}</span>"
                    );
                    self.depth += 1;
                    self.text_block(t, Parent::ListItem);
                    self.depth -= 1;
                    self.indent();
                    self.out.push_str("</li>\n");
                }
                ListEntry::List(sub) => {
                    self.indent();
                    let _ = writeln!(
                        self.out,
                        "<li class=\"s-li\"><span class=\"s-marker\" aria-hidden=\"true\"{marker_style}></span>"
                    );
                    self.depth += 1;
                    self.list(sub, Parent::ListItem);
                    self.depth -= 1;
                    self.indent();
                    self.out.push_str("</li>\n");
                }
            }
        }
        self.close(tag);
    }

    fn field(&mut self, f: &Field, parent: Parent) {
        let mut style = Vec::new();
        let auto_width =
            f.frame.width == Size::Auto && !matches!(f.kind, FieldKind::Checkbox { .. });
        match parent {
            Parent::Row if auto_width => style.push("flex:0 1 320px".into()),
            Parent::Column(false) if auto_width => {
                style.push("width:320px".into());
                style.push("max-width:100%".into());
            }
            _ => {}
        }
        sizing_css(&f.frame, parent, &mut style, None);
        // Fields keep their own height in rows and grids (SPEC.md §5).
        if matches!(parent, Parent::Row | Parent::Grid) {
            style.push("align-self:flex-start".into());
        }
        text_css(&f.text, &mut style);
        let required = if f.required {
            "<span aria-hidden=\"true\"> *</span>"
        } else {
            ""
        };
        let req_attr = if f.required { " required" } else { "" };
        let name = escape(&f.name);
        let label = escape(&f.label);
        let id = f
            .frame
            .id
            .as_ref()
            .map(|i| format!(" id=\"{}\"", escape(i)))
            .unwrap_or_default();
        let mut box_style = Vec::new();
        if let Some(fill) = f.frame.fill {
            box_style.push(format!("background-color:{}", color(fill)));
        }
        if f.frame.border > 0.0 {
            box_style.push(format!("border-width:{}", px(f.frame.border)));
        }
        if let Some(bc) = f.frame.border_color {
            box_style.push(format!("border-color:{}", color(bc)));
        }
        if f.frame.radius > 0.0 {
            box_style.push(format!("border-radius:{}", px(f.frame.radius)));
        }
        let box_style = if box_style.is_empty() {
            String::new()
        } else {
            format!(" style=\"{}\"", escape(&box_style.join(";")))
        };
        let outer = if style.is_empty() {
            String::new()
        } else {
            format!(" style=\"{}\"", escape(&style.join(";")))
        };
        self.indent();
        match &f.kind {
            FieldKind::Checkbox { checked } => {
                let checked = if *checked { " checked" } else { "" };
                let _ = writeln!(
                    self.out,
                    "<label class=\"s-check\"{id}{outer}><input type=\"checkbox\" name=\"{name}\" value=\"on\"{checked}{req_attr}><span>{label}{required}</span></label>"
                );
            }
            FieldKind::Input {
                kind,
                placeholder,
                value,
            } => {
                let ph = placeholder
                    .as_ref()
                    .map(|p| format!(" placeholder=\"{}\"", escape(p)))
                    .unwrap_or_default();
                let val = value
                    .as_ref()
                    .map(|v| format!(" value=\"{}\"", escape(v)))
                    .unwrap_or_default();
                let _ = writeln!(
                    self.out,
                    "<label class=\"s-field\"{id}{outer}><span class=\"s-label\">{label}{required}</span><input class=\"s-box\" type=\"{}\" name=\"{name}\"{ph}{val}{req_attr}{box_style}></label>",
                    kind.name()
                );
            }
            FieldKind::TextArea {
                rows,
                placeholder,
                value,
            } => {
                let ph = placeholder
                    .as_ref()
                    .map(|p| format!(" placeholder=\"{}\"", escape(p)))
                    .unwrap_or_default();
                let val = value.as_deref().map(escape).unwrap_or_default();
                let _ = writeln!(
                    self.out,
                    "<label class=\"s-field\"{id}{outer}><span class=\"s-label\">{label}{required}</span><textarea class=\"s-box\" name=\"{name}\" rows=\"{rows}\"{ph}{req_attr}{box_style}>{val}</textarea></label>"
                );
            }
            FieldKind::Select { options } => {
                let _ = write!(
                    self.out,
                    "<label class=\"s-field\"{id}{outer}><span class=\"s-label\">{label}{required}</span><select class=\"s-box\" name=\"{name}\"{req_attr}{box_style}>"
                );
                for o in options {
                    let value = o
                        .value
                        .as_ref()
                        .map(|v| format!(" value=\"{}\"", escape(v)))
                        .unwrap_or_default();
                    let selected = if o.selected { " selected" } else { "" };
                    let _ = write!(
                        self.out,
                        "<option{value}{selected}>{}</option>",
                        escape(&o.label)
                    );
                }
                self.out.push_str("</select></label>\n");
            }
        }
    }
}

/// Width/height of a child, which depends on its parent (SPEC.md §5).
/// `own_align` is set for items that are never stretched (images, fields):
/// they keep their parent's alignment, or `start` for `stretch`.
fn sizing_css(f: &Frame, parent: Parent, style: &mut Vec<String>, own_align: Option<Align>) {
    match parent {
        Parent::Row => {
            match f.width {
                Size::Px(w) => style.push(format!("flex:0 0 {}", px(w))),
                Size::Fill => {
                    let min = f.min_width.unwrap_or(0.0);
                    style.push(format!("flex:1 1 {}", px(min)));
                }
                Size::Auto => style.push("flex:0 1 auto".into()),
            }
            if f.min_width.is_none() {
                style.push("min-width:0".into());
            }
            if let Size::Px(h) = f.height {
                style.push(format!("height:{}", px(h)));
            }
            if let Some(Align::Stretch) = own_align {
                style.push("align-self:flex-start".into());
            }
        }
        Parent::Column(stretch) => {
            match f.width {
                Size::Px(w) => style.push(format!("width:{}", px(w))),
                Size::Fill => style.push("width:100%".into()),
                Size::Auto => {}
            }
            match f.height {
                Size::Px(h) => style.push(format!("height:{}", px(h))),
                Size::Fill => style.push("flex:1 1 0".into()),
                Size::Auto => {}
            }
            if own_align.is_some() && stretch {
                style.push("align-self:flex-start".into());
            }
        }
        Parent::Grid => {
            if let Size::Px(h) = f.height {
                style.push(format!("height:{}", px(h)));
            }
            if let Some(Align::Stretch) = own_align {
                style.push("align-self:start".into());
            }
        }
        Parent::ListItem => {}
    }
    if let Some(m) = f.max_width {
        style.push(format!("max-width:{}", px(m)));
    } else if !matches!(parent, Parent::Row | Parent::ListItem) && matches!(f.width, Size::Px(_)) {
        style.push("max-width:100%".into());
    }
    if let Some(m) = f.min_width {
        style.push(format!("min-width:{}", px(m)));
    }
}

fn frame_css(f: &Frame, style: &mut Vec<String>, is_page: bool) {
    if f.padding_x > 0.0 || f.padding_y > 0.0 {
        if f.padding_x == f.padding_y {
            style.push(format!("padding:{}", px(f.padding_x)));
        } else {
            style.push(format!("padding:{} {}", px(f.padding_y), px(f.padding_x)));
        }
    }
    if let Some(fill) = f.fill
        && !is_page
    {
        style.push(format!("background:{}", color(fill)));
    }
    if f.radius > 0.0 {
        style.push(format!("border-radius:{}", px(f.radius)));
    }
    if f.border > 0.0 {
        let c = f.border_color.unwrap_or(Color(0xd4d4d8ff));
        style.push(format!("border:{} solid {}", px(f.border), color(c)));
    }
    match f.shadow {
        Shadow::None => {}
        Shadow::Small => style.push("box-shadow:0 2px 6px rgba(0,0,0,.14)".into()),
        Shadow::Large => style.push("box-shadow:0 8px 18px rgba(0,0,0,.18)".into()),
    }
}

fn align_css(a: Align) -> &'static str {
    match a {
        Align::Start => "flex-start",
        Align::Center => "center",
        Align::End => "flex-end",
        Align::Stretch => "stretch",
    }
}

fn flow_css(flow: &Flow, style: &mut Vec<String>, _row: bool) {
    if flow.gap > 0.0 {
        style.push(format!("gap:{}", px(flow.gap)));
    }
    if flow.align != Align::Stretch {
        style.push(format!("align-items:{}", align_css(flow.align)));
    }
    let justify = match flow.justify {
        Justify::Start => None,
        Justify::Center => Some("center"),
        Justify::End => Some("flex-end"),
        Justify::Between => Some("space-between"),
    };
    if let Some(j) = justify {
        style.push(format!("justify-content:{j}"));
    }
}

fn text_css(t: &TextStyle, style: &mut Vec<String>) {
    if let Some(c) = t.color {
        style.push(format!("color:{}", color(c)));
    }
    if let Some(s) = t.size {
        style.push(format!("font-size:{}", px(s)));
    }
    if let Some(b) = t.bold {
        style.push(format!("font-weight:{}", if b { 700 } else { 400 }));
    }
    if let Some(i) = t.italic {
        style.push(format!(
            "font-style:{}",
            if i { "italic" } else { "normal" }
        ));
    }
    if let Some(u) = t.underline {
        style.push(format!(
            "text-decoration:{}",
            if u { "underline" } else { "none" }
        ));
    }
    if let Some(mono) = t.mono {
        let (family, generic) = if mono {
            ("SLML Mono", "monospace")
        } else {
            ("SLML Sans", "sans-serif")
        };
        style.push(format!(
            "font-family:\"{family}\",{FALLBACK_FAMILIES},{generic}"
        ));
    }
}

fn grid_columns(spec: &GridSpec, gap: f32) -> String {
    match (spec.columns, spec.cell_width) {
        (Some(n), None) => format!("repeat({n},minmax(0,1fr))"),
        (None, Some(cw)) => format!("repeat(auto-fill,minmax(min({},100%),1fr))", px(cw)),
        (Some(n), Some(cw)) => format!(
            "repeat(auto-fill,minmax(min(max({},calc((100% - {}) / {n})),100%),1fr))",
            px(cw),
            px(gap * (n.saturating_sub(1)) as f32)
        ),
        (None, None) => "minmax(0,1fr)".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn html(src: &str) -> String {
        let doc = slml_syntax::compile(src).unwrap();
        page(&doc, &Options::default())
    }

    #[test]
    fn document_shell() {
        let out =
            html(r#"slml 0.1 meta(title: "A & B", lang: "pl", icon: "/i.png") page { text "x" }"#);
        assert!(out.starts_with("<!doctype html>"));
        assert!(out.contains("<html lang=\"pl\">"));
        assert!(out.contains("<title>A &amp; B</title>"));
        assert!(out.contains("<link rel=\"icon\" href=\"i.png\">"));
        assert!(out.contains("_slml/slml.css"));
        assert!(
            !out.contains("<script"),
            "built pages contain no JavaScript"
        );
    }

    #[test]
    fn escapes_text_and_keeps_it_literal() {
        let out = html(r#"slml 0.1 page { text "<b>  x\ny</b>" }"#);
        assert!(out.contains("&lt;b&gt;  x\ny&lt;/b&gt;"));
    }

    #[test]
    fn links_point_to_html_and_respect_root() {
        let doc = slml_syntax::compile(
            r##"slml 0.1 page { heading(id: "t") "T" text { link(to: "/docs/a.slml#x") "a" link(to: "b.slml") "b" link(to: "#t") "c" link(to: "https://e.com/x.slml") "d" } }"##,
        )
        .unwrap();
        let out = page(
            &doc,
            &Options {
                root_prefix: "../".into(),
                ..Options::default()
            },
        );
        assert!(out.contains("href=\"../docs/a.html#x\""), "{out}");
        assert!(out.contains("href=\"b.html\""));
        assert!(out.contains("href=\"#t\""));
        assert!(out.contains("href=\"https://e.com/x.slml\""));
        assert!(out.contains("../_slml/slml.css"));
    }

    #[test]
    fn local_file_names_are_encoded_as_url_paths() {
        let opts = Options {
            root_prefix: "../".into(),
            ..Options::default()
        };
        assert_eq!(
            href("/docs/żółć 100%.slml#top", &opts),
            "../docs/%C5%BC%C3%B3%C5%82%C4%87%20100%25.html#top"
        );
        assert_eq!(href("literal%2e%2e.txt", &opts), "literal%252e%252e.txt");
        assert_eq!(asset("img/a#b?.png", &opts), "img/a%23b%3F.png");
        assert_eq!(href("#top", &opts), "#top");
        let external = "https://example.com/a%20b?q=one&x=two#top";
        assert_eq!(asset(external, &opts), external);
        assert_eq!(href(external, &opts), external);
    }

    #[test]
    fn layout_maps_to_flexbox() {
        let out = html(
            "slml 0.1 page { row(gap: 8, wrap: true) { column(width: fill, min-width: 200) column(width: 120) } grid(cell-width: 240, gap: 16) { text \"a\" } }",
        );
        assert!(out.contains("flex:1 1 200px"), "{out}");
        assert!(out.contains("flex:0 0 120px"));
        assert!(out.contains("flex-wrap:wrap"));
        assert!(out.contains("repeat(auto-fill,minmax(min(240px,100%),1fr))"));
    }

    #[test]
    fn semantic_elements() {
        let out = html(
            r#"slml 0.1 page {
              heading(level: 2) "H"
              list(marker: number) { item "a" item "b" }
              codeblock "x"
              button(to: "a.slml") "Go"
              image(src: "a.png", alt: "A", to: "b.slml")
              form(to: "https://f.io/x", method: post) {
                input(name: "e", label: "E-mail", kind: email, required: true)
                select(name: "s", label: "S") { option(value: "1") "One" }
                checkbox(name: "c", label: "C", checked: true)
                submit "Send"
              }
            }"#,
        );
        for needle in [
            "<h2 class=\"s-text\"",
            "<ol class=\"s-list\"",
            ">2.</span>",
            "<pre class=\"s-text s-mono\"",
            "<a class=\"s-text s-button\" href=\"a.html\"",
            "<a class=\"s-linkbox\" href=\"b.html\"",
            "<form class=\"s-col s-v\" action=\"https://f.io/x\" method=\"post\"",
            "type=\"email\" name=\"e\" required",
            "<option value=\"1\">One</option>",
            "checked",
            "<button class=\"s-text s-button\" type=\"submit\"",
        ] {
            assert!(out.contains(needle), "missing {needle}\n{out}");
        }
    }

    #[test]
    fn monospace_is_not_overridden() {
        let css = stylesheet(&[]);
        assert!(!css.contains("font-family:inherit"));
        assert!(!css.contains("code{font:inherit}"));
    }

    #[test]
    fn font_choices_apply_to_containers_lists_and_fields() {
        let out = html(
            r#"slml 0.1 page(font: mono) {
            column(font: mono) { text "Inherited mono" text(font: sans) "Sans override" }
            list(font: mono) { item "Monospaced list" }
            form(to: "https://example.com") {
                input(name: "code", label: "Code", font: mono)
                checkbox(name: "check", label: "Check", font: mono)
            }
        }"#,
        );
        let mono = "font-family:&quot;SLML Mono&quot;";
        let sans = "font-family:&quot;SLML Sans&quot;";
        assert_eq!(out.matches(mono).count(), 5, "{out}");
        assert_eq!(out.matches(sans).count(), 1, "{out}");
        assert!(
            out.contains("<p class=\"s-text\">Inherited mono</p>"),
            "inherit from column"
        );
    }

    #[test]
    fn stylesheet_lists_fonts() {
        let css = stylesheet(&[("A.ttf", "SLML Sans", 400, false)]);
        assert!(css.contains("url(\"fonts/A.ttf\")"));
        assert!(css.contains(".s-row{"));
    }
}
