//! The browser's own interface (status bubble, scrollbar, find bar,
//! inspector panel), drawn as paint items with the document's fonts.

use slml_render::fonts::{Face, Fonts, LINE_HEIGHT};
use slml_render::layout::{Item, Rect};
use slml_syntax::model::Color;

pub const ACCENT: Color = Color(0x2457d6ff);
pub const INK: Color = Color(0x1b1b1fff);
pub const PANEL: Color = Color(0xf6f6f8ff);
pub const LINE: Color = Color(0xd4d4d8ff);

/// Collects paint items for one frame of interface.
pub struct Ui<'a> {
    fonts: &'a Fonts,
    pub items: Vec<Item>,
}

impl<'a> Ui<'a> {
    pub fn new(fonts: &'a Fonts) -> Self {
        Self {
            fonts,
            items: Vec::new(),
        }
    }

    pub fn fill(&mut self, rect: Rect, color: Color, radius: f32) {
        self.items.push(Item::Fill {
            rect,
            color,
            radius,
        });
    }

    pub fn border(&mut self, rect: Rect, width: f32, color: Color, radius: f32) {
        self.items.push(Item::Border {
            rect,
            width,
            color,
            radius,
        });
    }

    pub fn shadow(&mut self, rect: Rect, radius: f32) {
        self.items.push(Item::Shadow {
            rect,
            radius,
            blur: 6.0,
            offset_y: 2.0,
            strength: 0.16,
        });
    }

    pub fn width(&self, text: &str, size: f32, face: Face) -> f32 {
        self.fonts.width(face, size, text)
    }

    /// Height of one line of text.
    pub fn line(size: f32) -> f32 {
        size * LINE_HEIGHT
    }

    /// One line of text with its line box's top-left corner at (x, y).
    /// Returns the width drawn.
    pub fn text(&mut self, x: f32, y: f32, text: &str, size: f32, color: Color, face: Face) -> f32 {
        let (glyphs, width) = self.fonts.shape(face, size, text);
        self.items.push(Item::Glyphs {
            face,
            size,
            color,
            x,
            baseline: y + self.fonts.baseline(face, size),
            glyphs,
        });
        width
    }

    pub fn fit(&self, text: &str, max_w: f32, size: f32, face: Face) -> String {
        // Multi-line values show their first line only.
        let first = text.lines().next().unwrap_or("");
        let cut = first.len() < text.len();
        if !cut && self.width(first, size, face) <= max_w {
            return first.to_string();
        }
        let mut chars: Vec<char> = first.chars().collect();
        loop {
            let candidate: String = chars.iter().collect::<String>() + "…";
            if chars.is_empty() || self.width(&candidate, size, face) <= max_w {
                return candidate;
            }
            chars.pop();
        }
    }

    /// A rounded bubble with one line of text, e.g. a link target.
    pub fn bubble(&mut self, x: f32, y: f32, max_w: f32, text: &str, size: f32) -> Rect {
        let (pad_x, pad_y) = (8.0, 3.0);
        let fitted = self.fit(text, max_w - 2.0 * pad_x, size, Face::Regular);
        let w = self.width(&fitted, size, Face::Regular) + 2.0 * pad_x;
        let h = Self::line(size) + 2.0 * pad_y;
        let rect = Rect::new(x, y - h, w, h);
        self.shadow(rect, 4.0);
        self.fill(rect, PANEL, 4.0);
        self.border(rect, 1.0, LINE, 4.0);
        self.text(x + pad_x, y - h + pad_y, &fitted, size, INK, Face::Regular);
        rect
    }
}

/// `rect` grown by `by` on every side.
pub fn outset(rect: Rect, by: f32) -> Rect {
    Rect::new(
        rect.x - by,
        rect.y - by,
        rect.w + 2.0 * by,
        rect.h + 2.0 * by,
    )
}
