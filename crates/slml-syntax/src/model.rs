//! Typed, validated document. Everything a renderer needs, nothing it has to
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
    /// Take all space the parent offers (shared equally between `fill` siblings).
    Fill,
    /// Logical pixels.
    Px(f32),
}

/// Cross-axis placement of children in a `column`/`row`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
    Stretch,
}

/// Main-axis distribution of children in a `column`/`row`.
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

/// The box around any block element. SLML always uses border-box sizing:
/// `width` includes padding and border.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub width: Size,
    pub height: Size,
    pub max_width: Option<f32>,
    pub padding_x: f32,
    pub padding_y: f32,
    pub fill: Option<Color>,
    pub radius: f32,
    pub border: f32,
    pub border_color: Option<Color>,
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            width: Size::Auto,
            height: Size::Auto,
            max_width: None,
            padding_x: 0.0,
            padding_y: 0.0,
            fill: None,
            radius: 0.0,
            border: 0.0,
            border_color: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Flow {
    pub gap: f32,
    pub align: Align,
    pub justify: Justify,
}

impl Default for Flow {
    fn default() -> Self {
        Self {
            gap: 0.0,
            align: Align::Stretch,
            justify: Justify::Start,
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
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Container(Container),
    Text(TextBlock),
    Image(Image),
    Divider(Divider),
    Spacer(Spacer),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerKind {
    /// The root. Lays children out vertically, like `column`.
    Page,
    Column,
    Row,
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
    /// Level 1–3.
    Heading(u8),
    Button,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextBlock {
    pub kind: TextKind,
    pub frame: Frame,
    pub text: TextStyle,
    pub align: TextAlign,
    /// Target of a `button`.
    pub link: Option<String>,
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
