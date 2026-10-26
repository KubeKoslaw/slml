//! Typed, validated document. Everything a backend needs, nothing it has to
//! guess: element defaults are already filled in by the analyzer.

/// sRGB color with alpha, stored as 0xRRGGBBAA.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color(pub u32);

impl Color {
    pub const WHITE: Color = Color(0xffffffff);

    pub fn r(self) -> u8 {
        (self.0 >> 24) as u8
    }
    pub fn g(self) -> u8 {
        (self.0 >> 16) as u8
    }
    pub fn b(self) -> u8 {
        (self.0 >> 8) as u8
    }
    pub fn a(self) -> u8 {
        self.0 as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Size {
    /// Size to content.
    Auto,
    /// Take all space the parent offers (shared between `fill` siblings).
    Fill,
    /// Logical pixels.
    Px(f32),
}

/// Cross-axis placement of children in a container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
}

/// Main-axis distribution of children in a container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Justify {
    Start,
    Center,
    End,
    Between,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    Start,
    Center,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shadow {
    None,
    Small,
    Large,
}

/// The box around any block element. SLML always uses border-box sizing:
/// `width` includes padding and border.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub width: Size,
    pub height: Size,
    pub min_width: Option<f32>,
    pub max_width: Option<f32>,
    pub padding_x: f32,
    pub padding_y: f32,
    pub fill: Option<Color>,
    pub radius: f32,
    pub border: f32,
    pub border_color: Option<Color>,
    pub shadow: Shadow,
    /// Anchor name for `#id` links.
    pub id: Option<String>,
    /// Makes the whole box a link (`button`, `column(to: …)`, `image(to: …)`).
    pub link: Option<String>,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            width: Size::Auto,
            height: Size::Auto,
            min_width: None,
            max_width: None,
            padding_x: 0.0,
            padding_y: 0.0,
            fill: None,
            radius: 0.0,
            border: 0.0,
            border_color: None,
            shadow: Shadow::None,
            id: None,
            link: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Flow {
    pub gap: f32,
    pub align: Align,
    pub justify: Justify,
    /// `row(wrap: true)`: children that don't fit move to the next line.
    pub wrap: bool,
}

impl Default for Flow {
    fn default() -> Self {
        Self {
            gap: 0.0,
            align: Align::Stretch,
            justify: Justify::Start,
            wrap: false,
        }
    }
}

/// Inherited text properties. `None` means "inherit from the parent".
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextStyle {
    pub color: Option<Color>,
    pub size: Option<f32>,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub mono: Option<bool>,
    pub underline: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub version: String,
    pub meta: Meta,
    pub page: Container,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Meta {
    pub title: Option<String>,
    pub lang: Option<String>,
    pub description: Option<String>,
    /// Path of the page icon (favicon).
    pub icon: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Container(Container),
    Text(TextBlock),
    Image(Image),
    Divider(Divider),
    Spacer(Spacer),
    List(List),
    Field(Field),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContainerKind {
    /// The root. Lays children out vertically, like `column`.
    Page,
    Column,
    Row,
    Grid(GridSpec),
    /// A column that submits the fields inside it.
    Form(FormSpec),
}

#[derive(Debug, Clone, PartialEq)]
pub struct GridSpec {
    /// Fixed number of columns, or the maximum when `cell_width` is set.
    pub columns: Option<u32>,
    /// Minimum cell width: as many columns as fit (responsive grid).
    pub cell_width: Option<f32>,
}

impl GridSpec {
    /// Number of columns for a given inner width.
    pub fn count(&self, inner_width: f32, gap: f32) -> usize {
        match (self.columns, self.cell_width) {
            (Some(n), None) => n.max(1) as usize,
            (max, Some(cw)) => {
                let fit = ((inner_width + gap) / (cw + gap)).floor().max(1.0) as usize;
                max.map_or(fit, |m| fit.min(m.max(1) as usize))
            }
            (None, None) => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormSpec {
    pub to: String,
    pub method: Method,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Container {
    pub kind: ContainerKind,
    pub frame: Frame,
    pub flow: Flow,
    pub text: TextStyle,
    pub children: Vec<Node>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextKind {
    Paragraph,
    /// Level 1–4.
    Heading(u8),
    Button,
    Submit,
    /// `codeblock`: preformatted, monospaced.
    Code,
    /// A list `item`.
    Item,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextBlock {
    pub kind: TextKind,
    pub frame: Frame,
    pub text: TextStyle,
    pub align: TextAlign,
    pub content: Vec<Inline>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Inline {
    Text(String),
    Span(InlineSpan),
}

/// `bold`, `italic`, `code`, `link` or `span`.
#[derive(Debug, Clone, PartialEq)]
pub struct InlineSpan {
    pub text: TextStyle,
    pub link: Option<String>,
    pub content: Vec<Inline>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    pub frame: Frame,
    pub src: String,
    pub alt: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Divider {
    pub id: Option<String>,
    pub width: Size,
    pub thickness: f32,
    pub color: Color,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spacer {
    /// `Auto` on the parent's main axis means "absorb free space".
    pub width: Size,
    pub height: Size,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Marker {
    Bullet,
    Number,
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub struct List {
    pub frame: Frame,
    pub text: TextStyle,
    pub gap: f32,
    pub marker: Marker,
    pub items: Vec<ListEntry>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ListEntry {
    Item(TextBlock),
    /// A nested list, indented under the previous item.
    List(List),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub frame: Frame,
    pub text: TextStyle,
    pub name: String,
    /// Visible label; required for accessibility.
    pub label: String,
    pub required: bool,
    pub kind: FieldKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldKind {
    Input {
        kind: InputKind,
        placeholder: Option<String>,
        value: Option<String>,
    },
    TextArea {
        rows: u32,
        placeholder: Option<String>,
        value: Option<String>,
    },
    Checkbox {
        checked: bool,
    },
    Select {
        options: Vec<SelectOption>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    Text,
    Email,
    Password,
    Number,
    Tel,
    Url,
    Date,
    Search,
}

impl InputKind {
    pub const NAMES: [&'static str; 8] = [
        "text", "email", "password", "number", "tel", "url", "date", "search",
    ];

    pub fn name(self) -> &'static str {
        Self::NAMES[self as usize]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectOption {
    pub label: String,
    /// Submitted value; the label when `None`.
    pub value: Option<String>,
    pub selected: bool,
}
