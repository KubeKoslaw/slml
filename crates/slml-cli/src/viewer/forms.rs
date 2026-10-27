//! Native form fields: typing with a caret, checkboxes, drop-down selects,
//! validation and sending.
//!
//! The values live in the page's document model, so the renderer draws
//! them exactly like values written in the source (bullets for passwords,
//! wrapping in text areas). Controls are found by the layout's child-index
//! paths (see [`slml_render::layout::ControlArea`]).

use slml_render::fonts::{Fonts, LINE_HEIGHT};
use slml_render::layout::{ControlArea, Rect};
use slml_syntax::model::{
    ContainerKind, Document, Field, FieldKind, FormSpec, InputKind, Method, Node, TextKind,
};

use super::text::{stops, x_of};

/// What a control path points to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `in` or `area`: takes typed text.
    Text {
        multiline: bool,
    },
    Check,
    Select,
    Submit,
}

pub fn node_at<'a>(doc: &'a Document, path: &[usize]) -> Option<&'a Node> {
    let (last, parents) = path.split_last()?;
    let mut container = &doc.page;
    for &i in parents {
        match container.children.get(i)? {
            Node::Container(c) => container = c,
            _ => return None,
        }
    }
    container.children.get(*last)
}

fn field_mut<'a>(doc: &'a mut Document, path: &[usize]) -> Option<&'a mut Field> {
    let (last, parents) = path.split_last()?;
    let mut container = &mut doc.page;
    for &i in parents {
        match container.children.get_mut(i)? {
            Node::Container(c) => container = c,
            _ => return None,
        }
    }
    match container.children.get_mut(*last)? {
        Node::Field(f) => Some(f),
        _ => None,
    }
}

pub fn kind(doc: &Document, path: &[usize]) -> Option<Kind> {
    Some(match node_at(doc, path)? {
        Node::Field(f) => match f.kind {
            FieldKind::Input { .. } => Kind::Text { multiline: false },
            FieldKind::TextArea { .. } => Kind::Text { multiline: true },
            FieldKind::Checkbox { .. } => Kind::Check,
            FieldKind::Select { .. } => Kind::Select,
        },
        Node::Text(t) if t.kind == TextKind::Submit => Kind::Submit,
        _ => return None,
    })
}

pub fn field<'a>(doc: &'a Document, path: &[usize]) -> Option<&'a Field> {
    match node_at(doc, path)? {
        Node::Field(f) => Some(f),
        _ => None,
    }
}

/// The text typed into an `in` or `area` (empty when none).
pub fn text(doc: &Document, path: &[usize]) -> String {
    match field(doc, path).map(|f| &f.kind) {
        Some(FieldKind::Input { value, .. } | FieldKind::TextArea { value, .. }) => {
            value.clone().unwrap_or_default()
        }
        _ => String::new(),
    }
}

fn set_text(doc: &mut Document, path: &[usize], text: String) {
    if let Some(f) = field_mut(doc, path) {
        match &mut f.kind {
            FieldKind::Input { value, .. } | FieldKind::TextArea { value, .. } => {
                *value = (!text.is_empty()).then_some(text)
            }
            _ => {}
        }
    }
}

/// A text field being edited, with the caret as a character index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edit {
    pub path: Vec<usize>,
    pub caret: usize,
}

impl Edit {
    /// Starts editing with the caret at the end of the text.
    pub fn at_end(doc: &Document, path: Vec<usize>) -> Edit {
        let caret = text(doc, &path).chars().count();
        Edit { path, caret }
    }

    pub fn insert(&mut self, doc: &mut Document, s: &str) {
        let multiline = matches!(kind(doc, &self.path), Some(Kind::Text { multiline: true }));
        let s: String = s
            .chars()
            .filter(|c| multiline || (*c != '\n' && *c != '\r'))
            .filter(|c| *c != '\r')
            .collect();
        let mut chars: Vec<char> = text(doc, &self.path).chars().collect();
        let at = self.caret.min(chars.len());
        let count = s.chars().count();
        chars.splice(at..at, s.chars());
        self.caret = at + count;
        set_text(doc, &self.path, chars.into_iter().collect());
    }

    pub fn backspace(&mut self, doc: &mut Document) {
        let mut chars: Vec<char> = text(doc, &self.path).chars().collect();
        if self.caret == 0 || chars.is_empty() {
            return;
        }
        self.caret = self.caret.min(chars.len());
        chars.remove(self.caret - 1);
        self.caret -= 1;
        set_text(doc, &self.path, chars.into_iter().collect());
    }

    pub fn delete(&mut self, doc: &mut Document) {
        let mut chars: Vec<char> = text(doc, &self.path).chars().collect();
        if self.caret < chars.len() {
            chars.remove(self.caret);
            set_text(doc, &self.path, chars.into_iter().collect());
        }
    }

    /// Moves the caret by `by` characters, staying inside the text.
    pub fn step(&mut self, doc: &Document, by: i32) {
        let len = text(doc, &self.path).chars().count() as i32;
        self.caret = (self.caret as i32 + by).clamp(0, len) as usize;
    }

    /// Home and End: the start or end of the caret's line.
    pub fn line_edge(&mut self, doc: &Document, end: bool) {
        let chars: Vec<char> = text(doc, &self.path).chars().collect();
        let mut i = self.caret.min(chars.len());
        if end {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else {
            while i > 0 && chars[i - 1] != '\n' {
                i -= 1;
            }
        }
        self.caret = i;
    }
}

/// Character index → UTF-16 offset in the same text.
fn utf16_of(text: &str, chars: usize) -> usize {
    text.chars().take(chars).map(char::len_utf16).sum()
}

/// UTF-16 offset → character index.
fn chars_of(text: &str, utf16: usize) -> usize {
    let mut units = 0;
    for (i, c) in text.chars().enumerate() {
        if units >= utf16 {
            return i;
        }
        units += c.len_utf16();
    }
    text.chars().count()
}

/// Where the caret is drawn (page coordinates), for a caret given as a
/// character index into the field's text.
pub fn caret_rect(fonts: &Fonts, area: &ControlArea, caret: usize) -> Rect {
    let line = area.size * LINE_HEIGHT;
    let start = Rect::new(area.text_rect.x, area.text_rect.y, 1.5, line);
    let Some(value) = &area.value else {
        return start;
    };
    let shown = &value.text;
    let total = shown.chars().count();
    // After a final line break the caret is at the start of an empty line.
    if caret >= total && shown.ends_with('\n') {
        let below = value.runs.last().map_or(start.y, |r| r.rect.y + line);
        return Rect::new(area.text_rect.x, below, 1.5, line);
    }
    let offset = utf16_of(shown, caret);
    let run = value
        .runs
        .iter()
        .filter(|r| r.range.start <= offset)
        .find(|r| offset <= r.range.end)
        .or(value.runs.iter().rfind(|r| r.range.start <= offset));
    match run {
        Some(run) => Rect::new(
            x_of(fonts, value, run, offset),
            run.rect.y,
            1.5,
            run.rect.h.max(line),
        ),
        None => start,
    }
}

/// The character index nearest to a point inside a text field.
pub fn caret_at(fonts: &Fonts, area: &ControlArea, x: f32, y: f32) -> usize {
    let Some(value) = &area.value else {
        return 0;
    };
    let distance = |r: &Rect| {
        let dy = (r.y - y).max(y - (r.y + r.h)).max(0.0);
        let dx = (r.x - x).max(x - (r.x + r.w)).max(0.0);
        (dy, dx)
    };
    let Some(run) = value
        .runs
        .iter()
        .min_by(|a, b| distance(&a.rect).partial_cmp(&distance(&b.rect)).unwrap())
    else {
        return 0;
    };
    let offset = stops(fonts, value, run)
        .into_iter()
        .min_by(|(_, a), (_, b)| (a - x).abs().partial_cmp(&(b - x).abs()).unwrap())
        .map_or(run.range.start, |(o, _)| o);
    chars_of(&value.text, offset)
}

pub fn toggle(doc: &mut Document, path: &[usize]) {
    if let Some(f) = field_mut(doc, path)
        && let FieldKind::Checkbox { checked } = &mut f.kind
    {
        *checked = !*checked;
    }
}

/// The labels of a select's options and the index of the chosen one.
pub fn options(doc: &Document, path: &[usize]) -> (Vec<String>, usize) {
    match field(doc, path).map(|f| &f.kind) {
        Some(FieldKind::Select { options }) => (
            options.iter().map(|o| o.label.clone()).collect(),
            options.iter().position(|o| o.selected).unwrap_or(0),
        ),
        _ => (Vec::new(), 0),
    }
}

pub fn choose(doc: &mut Document, path: &[usize], index: usize) {
    if let Some(f) = field_mut(doc, path)
        && let FieldKind::Select { options } = &mut f.kind
        && index < options.len()
    {
        for (i, o) in options.iter_mut().enumerate() {
            o.selected = i == index;
        }
    }
}

/// What pressing a submit button does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Submission {
    /// A field is missing or wrong: focus it and say why.
    Invalid { path: Vec<usize>, message: String },
    /// Follow `to` (a page of the site) with the encoded values.
    Local { to: String, query: String },
    /// Open `url` (with the values in its query) in the system browser.
    External { url: String },
    /// Send the values with POST to `url` through the system browser.
    Post {
        url: String,
        pairs: Vec<(String, String)>,
    },
    /// The form cannot be sent from here.
    Unsupported(String),
}

/// The form containing `path`: its spec and its own path.
fn form_of<'a>(doc: &'a Document, path: &[usize]) -> Option<(&'a FormSpec, Vec<usize>)> {
    let mut container = &doc.page;
    let mut found = None;
    for (depth, &i) in path.iter().enumerate() {
        match container.children.get(i)? {
            Node::Container(c) => {
                if let ContainerKind::Form(spec) = &c.kind {
                    found = Some((spec, path[..=depth].to_vec()));
                }
                container = c;
            }
            _ => break,
        }
    }
    found
}

/// Every field of a form with its path, in document order.
fn fields(doc: &Document, form: &[usize]) -> Vec<(Vec<usize>, Field)> {
    fn walk(nodes: &[Node], prefix: &mut Vec<usize>, out: &mut Vec<(Vec<usize>, Field)>) {
        for (i, n) in nodes.iter().enumerate() {
            prefix.push(i);
            match n {
                Node::Field(f) => out.push((prefix.clone(), f.clone())),
                Node::Container(c) => walk(&c.children, prefix, out),
                _ => {}
            }
            prefix.pop();
        }
    }
    let mut out = Vec::new();
    if let Some(Node::Container(c)) = node_at(doc, form) {
        let mut prefix = form.to_vec();
        walk(&c.children, &mut prefix, &mut out);
    }
    out
}

fn problem(field: &Field) -> Option<String> {
    let label = &field.label;
    match &field.kind {
        FieldKind::Input { kind, value, .. } => {
            let v = value.as_deref().unwrap_or("").trim();
            if v.is_empty() {
                return field.required.then(|| format!("Please fill in “{label}”"));
            }
            match kind {
                InputKind::Email
                    if !(v.contains('@') && !v.starts_with('@') && !v.ends_with('@')) =>
                {
                    Some(format!("“{label}” needs an e-mail address"))
                }
                InputKind::Number if v.parse::<f64>().is_err() => {
                    Some(format!("“{label}” needs a number"))
                }
                InputKind::Url if !(v.starts_with("https://") || v.starts_with("http://")) => {
                    Some(format!("“{label}” needs an address starting with https://"))
                }
                _ => None,
            }
        }
        FieldKind::TextArea { value, .. } => (field.required
            && value.as_deref().unwrap_or("").trim().is_empty())
        .then(|| format!("Please fill in “{label}”")),
        FieldKind::Checkbox { checked } => {
            (field.required && !checked).then(|| format!("Please tick “{label}”"))
        }
        FieldKind::Select { .. } => None,
    }
}

fn value_of(field: &Field) -> Option<String> {
    match &field.kind {
        FieldKind::Input { value, .. } | FieldKind::TextArea { value, .. } => {
            Some(value.clone().unwrap_or_default())
        }
        FieldKind::Checkbox { checked } => checked.then(|| "on".to_string()),
        FieldKind::Select { options } => options
            .iter()
            .find(|o| o.selected)
            .or(options.first())
            .map(|o| o.value.clone().unwrap_or_else(|| o.label.clone())),
    }
}

/// `application/x-www-form-urlencoded`, as browsers send forms.
pub fn urlencode(pairs: &[(String, String)]) -> String {
    let enc = |s: &str| {
        let mut out = String::new();
        for b in s.bytes() {
            match b {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'*' => {
                    out.push(b as char)
                }
                b' ' => out.push('+'),
                b => out.push_str(&format!("%{b:02X}")),
            }
        }
        out
    };
    pairs
        .iter()
        .map(|(k, v)| format!("{}={}", enc(k), enc(v)))
        .collect::<Vec<_>>()
        .join("&")
}

/// Validates the form of the submit button at `path` and says how to send it.
pub fn submit(doc: &Document, path: &[usize]) -> Submission {
    let Some((spec, form)) = form_of(doc, path) else {
        return Submission::Unsupported("This button is not inside a form".into());
    };
    let mut pairs = Vec::new();
    for (field_path, field) in fields(doc, &form) {
        if let Some(message) = problem(&field) {
            return Submission::Invalid {
                path: field_path,
                message,
            };
        }
        if let Some(v) = value_of(&field) {
            pairs.push((field.name.clone(), v));
        }
    }
    let external = slml_syntax::analyze::is_external(&spec.to);
    match (spec.method, external) {
        (Method::Get, false) => Submission::Local {
            to: spec.to.clone(),
            query: urlencode(&pairs),
        },
        (Method::Get, true) => {
            let sep = if spec.to.contains('?') { '&' } else { '?' };
            Submission::External {
                url: format!("{}{sep}{}", spec.to, urlencode(&pairs)),
            }
        }
        (Method::Post, true) => Submission::Post {
            url: spec.to.clone(),
            pairs,
        },
        (Method::Post, false) => Submission::Unsupported(format!(
            "`{}` is a page of this site; it cannot receive a post form. Use `get`, or send to an https address",
            spec.to
        )),
    }
}

/// A one-off HTML page that posts the values to `url` in the system browser.
pub fn post_page(url: &str, pairs: &[(String, String)]) -> String {
    let esc = slml_html::escape;
    let inputs: String = pairs
        .iter()
        .map(|(k, v)| {
            format!(
                "<input type=\"hidden\" name=\"{}\" value=\"{}\">",
                esc(k),
                esc(v)
            )
        })
        .collect();
    format!(
        "<!doctype html><meta charset=\"utf-8\"><title>Sending…</title>\
         <form method=\"post\" action=\"{}\">{inputs}<noscript><button>Send</button></noscript></form>\
         <script>document.forms[0].submit()</script>",
        esc(url)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(src: &str) -> Document {
        slml_syntax::compile(src).unwrap()
    }

    const FORM: &str = r#"slml 0.2 page{form{>preview.slml
        in{n:name "Name" req}
        in{n:email "E-mail" k:email}
        area{n:msg "Message"}
        check{n:ok "OK"}
        sel{n:size "Size" opt"S" opt{v"m" "M"}}
        send"Send"}}"#;

    #[test]
    fn typing_moves_the_caret_and_edits_the_model() {
        let mut d = doc(FORM);
        let mut e = Edit::at_end(&d, vec![0, 0]);
        e.insert(&mut d, "Ann");
        e.step(&d, -1);
        e.insert(&mut d, "x");
        assert_eq!(text(&d, &[0, 0]), "Anxn");
        e.backspace(&mut d);
        e.delete(&mut d);
        assert_eq!(text(&d, &[0, 0]), "An");
        // A one-line input drops line breaks; a text area keeps them.
        e.insert(&mut d, "a\nb");
        assert_eq!(text(&d, &[0, 0]), "Anab");
        let mut m = Edit::at_end(&d, vec![0, 2]);
        m.insert(&mut d, "one\ntwo");
        m.line_edge(&d, false);
        assert_eq!(m.caret, 4);
    }

    #[test]
    fn checkbox_and_select() {
        let mut d = doc(FORM);
        assert_eq!(kind(&d, &[0, 3]), Some(Kind::Check));
        toggle(&mut d, &[0, 3]);
        choose(&mut d, &[0, 4], 1);
        assert_eq!(options(&d, &[0, 4]), (vec!["S".into(), "M".into()], 1));
        assert_eq!(kind(&d, &[0, 5]), Some(Kind::Submit));
    }

    #[test]
    fn submitting_validates_then_encodes() {
        let mut d = doc(FORM);
        match submit(&d, &[0, 5]) {
            Submission::Invalid { path, message } => {
                assert_eq!(path, vec![0, 0]);
                assert!(message.contains("Name"), "{message}");
            }
            other => panic!("{other:?}"),
        }
        Edit::at_end(&d, vec![0, 0]).insert(&mut d, "Ann Smith");
        Edit::at_end(&d, vec![0, 1]).insert(&mut d, "nope");
        assert!(matches!(submit(&d, &[0, 5]), Submission::Invalid { .. }));
        let mut e = Edit::at_end(&d, vec![0, 1]);
        e.insert(&mut d, "@x.pl");
        toggle(&mut d, &[0, 3]);
        choose(&mut d, &[0, 4], 1);
        assert_eq!(
            submit(&d, &[0, 5]),
            Submission::Local {
                to: "preview.slml".into(),
                query: "name=Ann+Smith&email=nope%40x.pl&msg=&ok=on&size=m".into()
            }
        );
    }

    #[test]
    fn external_forms_go_through_the_system_browser() {
        let d = doc(r#"slml 0.2 page{form{>https://e.com/f post in{n:q "Q"} send"Go"}}"#);
        assert!(matches!(submit(&d, &[0, 1]), Submission::Post { .. }));
        let d = doc(r#"slml 0.2 page{form{>https://e.com/s in{n:q "Q" v"a b"} send"Go"}}"#);
        assert_eq!(
            submit(&d, &[0, 1]),
            Submission::External {
                url: "https://e.com/s?q=a+b".into()
            }
        );
        let page = post_page("https://e.com/f", &[("q".into(), "<x>".into())]);
        assert!(page.contains("value=\"&lt;x&gt;\""));
    }

    #[test]
    fn caret_geometry_follows_the_text() {
        let mut d = doc(FORM);
        Edit::at_end(&d, vec![0, 0]).insert(&mut d, "Hello");
        let fonts = Fonts::load();
        let mut images = slml_render::images::ImageStore::new(".");
        let layout = slml_render::layout::layout(&d, &fonts, &mut images, 600.0, 0.0);
        let area = layout.controls.iter().find(|c| c.path == [0, 0]).unwrap();
        let start = caret_rect(&fonts, area, 0);
        let end = caret_rect(&fonts, area, 5);
        assert!(end.x > start.x + 20.0, "{start:?} {end:?}");
        assert_eq!(caret_at(&fonts, area, end.x + 50.0, end.y + 2.0), 5);
        assert_eq!(caret_at(&fonts, area, start.x, start.y + 2.0), 0);
    }
}
