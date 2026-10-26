//! Looks up document elements by their child-index path and describes them
//! for element inspectors. The layout records a [`crate::layout::ElementBox`]
//! per element; hosts use its `path` to get back here.

use slml_syntax::model::*;

/// A borrowed block element of the document.
#[derive(Debug, Clone, Copy)]
pub enum Element<'a> {
    Container(&'a Container),
    Text(&'a TextBlock),
    Image(&'a Image),
    Divider(&'a Divider),
    Spacer(&'a Spacer),
    List(&'a List),
    Field(&'a Field),
}

impl<'a> Element<'a> {
    pub fn of(node: &'a Node) -> Self {
        match node {
            Node::Container(c) => Element::Container(c),
            Node::Text(t) => Element::Text(t),
            Node::Image(i) => Element::Image(i),
            Node::Divider(d) => Element::Divider(d),
            Node::Spacer(s) => Element::Spacer(s),
            Node::List(l) => Element::List(l),
            Node::Field(f) => Element::Field(f),
        }
    }

    pub fn of_entry(entry: &'a ListEntry) -> Self {
        match entry {
            ListEntry::Item(t) => Element::Text(t),
            ListEntry::List(l) => Element::List(l),
        }
    }

    /// The element's name as written in SLML.
    pub fn name(self) -> &'static str {
        match self {
            Element::Container(c) => match c.kind {
                ContainerKind::Page => "page",
                ContainerKind::Column => "column",
                ContainerKind::Row => "row",
                ContainerKind::Grid(_) => "grid",
                ContainerKind::Form(_) => "form",
            },
            Element::Text(t) => match t.kind {
                TextKind::Paragraph => "text",
                TextKind::Heading(_) => "heading",
                TextKind::Button => "button",
                TextKind::Submit => "submit",
                TextKind::Code => "codeblock",
                TextKind::Item => "item",
            },
            Element::Image(_) => "image",
            Element::Divider(_) => "divider",
            Element::Spacer(_) => "spacer",
            Element::List(_) => "list",
            Element::Field(f) => match f.kind {
                FieldKind::Input { .. } => "input",
                FieldKind::TextArea { .. } => "textarea",
                FieldKind::Checkbox { .. } => "checkbox",
                FieldKind::Select { .. } => "select",
            },
        }
    }

    pub fn frame(self) -> Option<&'a Frame> {
        match self {
            Element::Container(c) => Some(&c.frame),
            Element::Text(t) => Some(&t.frame),
            Element::Image(i) => Some(&i.frame),
            Element::List(l) => Some(&l.frame),
            Element::Field(f) => Some(&f.frame),
            Element::Divider(_) | Element::Spacer(_) => None,
        }
    }

    /// Children in the order their indices appear in a path.
    fn child(self, index: usize) -> Option<Element<'a>> {
        match self {
            Element::Container(c) => c.children.get(index).map(Element::of),
            Element::List(l) => l.items.get(index).map(Element::of_entry),
            _ => None,
        }
    }

    /// The element's resolved attributes, using SLML attribute names. Only
    /// values that differ from "nothing set" are listed, except `width` and
    /// `height`, which every boxed element reports.
    pub fn attributes(self) -> Vec<(&'static str, String)> {
        let mut out = Vec::new();
        if let Some(f) = self.frame() {
            frame(f, &mut out);
        }
        match self {
            Element::Container(c) => {
                flow(&c.flow, &mut out);
                match &c.kind {
                    ContainerKind::Grid(g) => {
                        if let Some(n) = g.columns {
                            out.push(("columns", n.to_string()));
                        }
                        if let Some(w) = g.cell_width {
                            out.push(("cell-width", num(w)));
                        }
                    }
                    ContainerKind::Form(spec) => {
                        out.push(("to", spec.to.clone()));
                        let method = match spec.method {
                            Method::Get => "get",
                            Method::Post => "post",
                        };
                        out.push(("method", method.into()));
                    }
                    ContainerKind::Row if c.flow.wrap => out.push(("wrap", "true".into())),
                    _ => {}
                }
                text(&c.text, &mut out);
            }
            Element::Text(t) => {
                if let TextKind::Heading(level) = t.kind {
                    out.push(("level", level.to_string()));
                }
                match t.align {
                    TextAlign::Start => {}
                    TextAlign::Center => out.push(("align", "center".into())),
                    TextAlign::End => out.push(("align", "end".into())),
                }
                text(&t.text, &mut out);
            }
            Element::Image(i) => {
                out.push(("src", i.src.clone()));
                out.push(("alt", i.alt.clone()));
            }
            Element::Divider(d) => {
                out.push(("width", size(d.width)));
                out.push(("thickness", num(d.thickness)));
                out.push(("color", color(d.color)));
                if let Some(id) = &d.id {
                    out.push(("id", id.clone()));
                }
            }
            Element::Spacer(s) => {
                out.push(("width", size(s.width)));
                out.push(("height", size(s.height)));
            }
            Element::List(l) => {
                if l.gap > 0.0 {
                    out.push(("gap", num(l.gap)));
                }
                let marker = match l.marker {
                    Marker::Bullet => "bullet",
                    Marker::Number => "number",
                    Marker::None => "none",
                };
                out.push(("marker", marker.into()));
                text(&l.text, &mut out);
            }
            Element::Field(f) => {
                out.push(("name", f.name.clone()));
                out.push(("label", f.label.clone()));
                if f.required {
                    out.push(("required", "true".into()));
                }
                match &f.kind {
                    FieldKind::Input {
                        kind,
                        placeholder,
                        value,
                    } => {
                        out.push(("kind", kind.name().into()));
                        if let Some(p) = placeholder {
                            out.push(("placeholder", p.clone()));
                        }
                        if let Some(v) = value {
                            out.push(("value", v.clone()));
                        }
                    }
                    FieldKind::TextArea {
                        rows,
                        placeholder,
                        value,
                    } => {
                        out.push(("rows", rows.to_string()));
                        if let Some(p) = placeholder {
                            out.push(("placeholder", p.clone()));
                        }
                        if let Some(v) = value {
                            out.push(("value", v.clone()));
                        }
                    }
                    FieldKind::Checkbox { checked } => out.push(("checked", checked.to_string())),
                    FieldKind::Select { options } => {
                        let labels: Vec<&str> = options.iter().map(|o| o.label.as_str()).collect();
                        out.push(("options", labels.join(", ")));
                    }
                }
                text(&f.text, &mut out);
            }
        }
        out
    }
}

/// The element at `path` (child indices from the page; list entries count
/// as children of their list). The empty path is the page itself.
pub fn element<'a>(doc: &'a Document, path: &[usize]) -> Option<Element<'a>> {
    path.iter()
        .try_fold(Element::Container(&doc.page), |e, &i| e.child(i))
}

fn num(v: f32) -> String {
    // Integers print without a fraction, like they are usually written.
    if v.fract() == 0.0 {
        format!("{v:.0}")
    } else {
        format!("{v:.2}")
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn size(s: Size) -> String {
    match s {
        Size::Auto => "auto".into(),
        Size::Fill => "fill".into(),
        Size::Px(p) => num(p),
    }
}

fn color(c: Color) -> String {
    if c.a() == 0xff {
        format!("#{:06x}", c.0 >> 8)
    } else {
        format!("#{:08x}", c.0)
    }
}

fn frame(f: &Frame, out: &mut Vec<(&'static str, String)>) {
    out.push(("width", size(f.width)));
    out.push(("height", size(f.height)));
    if let Some(w) = f.min_width {
        out.push(("min-width", num(w)));
    }
    if let Some(w) = f.max_width {
        out.push(("max-width", num(w)));
    }
    if f.padding_x == f.padding_y {
        if f.padding_x > 0.0 {
            out.push(("padding", num(f.padding_x)));
        }
    } else {
        out.push(("padding-x", num(f.padding_x)));
        out.push(("padding-y", num(f.padding_y)));
    }
    if let Some(c) = f.fill {
        out.push(("fill", color(c)));
    }
    if f.radius > 0.0 {
        out.push(("radius", num(f.radius)));
    }
    if f.border > 0.0 {
        out.push(("border", num(f.border)));
    }
    if let Some(c) = f.border_color {
        out.push(("border-color", color(c)));
    }
    match f.shadow {
        Shadow::None => {}
        Shadow::Small => out.push(("shadow", "small".into())),
        Shadow::Large => out.push(("shadow", "large".into())),
    }
    if let Some(id) = &f.id {
        out.push(("id", id.clone()));
    }
    if let Some(to) = &f.link {
        out.push(("to", to.clone()));
    }
}

fn flow(f: &Flow, out: &mut Vec<(&'static str, String)>) {
    if f.gap > 0.0 {
        out.push(("gap", num(f.gap)));
    }
    let align = match f.align {
        Align::Start => "start",
        Align::Center => "center",
        Align::End => "end",
        Align::Stretch => "stretch",
    };
    out.push(("align", align.into()));
    let justify = match f.justify {
        Justify::Start => "start",
        Justify::Center => "center",
        Justify::End => "end",
        Justify::Between => "between",
    };
    out.push(("justify", justify.into()));
}

fn text(t: &TextStyle, out: &mut Vec<(&'static str, String)>) {
    if let Some(c) = t.color {
        out.push(("color", color(c)));
    }
    if let Some(s) = t.size {
        out.push(("size", num(s)));
    }
    let flags = [
        ("bold", t.bold),
        ("italic", t.italic),
        ("underline", t.underline),
    ];
    for (name, value) in flags {
        if let Some(v) = value {
            out.push((name, v.to_string()));
        }
    }
    if let Some(mono) = t.mono {
        out.push(("font", if mono { "mono" } else { "sans" }.into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_reach_nested_elements_and_list_entries() {
        let doc = slml_syntax::compile(
            r#"slml 0.1 page(padding: 8) {
                row(gap: 12, fill: #eef) { text "a" image(src: "x.png", alt: "X", width: 40) }
                list { item "one" list { item(bold: true) "two" } }
            }"#,
        )
        .unwrap();
        assert_eq!(element(&doc, &[]).unwrap().name(), "page");
        let row = element(&doc, &[0]).unwrap();
        assert_eq!(row.name(), "row");
        let attrs = row.attributes();
        assert!(attrs.contains(&("gap", "12".into())), "{attrs:?}");
        assert!(attrs.contains(&("fill", "#eeeeff".into())), "{attrs:?}");
        assert!(attrs.contains(&("width", "auto".into())), "{attrs:?}");
        let image = element(&doc, &[0, 1]).unwrap();
        assert_eq!(image.name(), "image");
        assert!(image.attributes().contains(&("width", "40".into())));
        let nested = element(&doc, &[1, 1, 0]).unwrap();
        assert_eq!(nested.name(), "item");
        assert!(nested.attributes().contains(&("bold", "true".into())));
        assert!(element(&doc, &[0, 5]).is_none());
        assert!(element(&doc, &[0, 0, 0]).is_none(), "text has no children");
    }

    #[test]
    fn numbers_print_like_source() {
        assert_eq!(num(16.0), "16");
        assert_eq!(num(1.5), "1.5");
        assert_eq!(num(0.25), "0.25");
        assert_eq!(color(Color(0x11223344)), "#11223344");
    }
}
