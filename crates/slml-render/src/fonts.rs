//! Bundled fonts (Noto, SIL Open Font License) and text measurement.
//!
//! Fonts are compiled into the binary so a document renders identically on
//! every machine, which is one of SLML's goals.

use fontdue::{Font, FontSettings};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Face {
    Regular = 0,
    Bold = 1,
    Italic = 2,
    BoldItalic = 3,
    Mono = 4,
    MonoBold = 5,
}

impl Face {
    pub fn select(bold: bool, italic: bool, mono: bool) -> Face {
        match (mono, bold, italic) {
            (true, true, _) => Face::MonoBold,
            (true, false, _) => Face::Mono,
            (false, true, true) => Face::BoldItalic,
            (false, true, false) => Face::Bold,
            (false, false, true) => Face::Italic,
            (false, false, false) => Face::Regular,
        }
    }
}

pub struct Fonts {
    faces: Vec<Font>,
}

/// Distance between baselines, as a multiple of the font size.
pub const LINE_HEIGHT: f32 = 1.45;

impl Fonts {
    pub fn load() -> Fonts {
        let data: [&[u8]; 6] = [
            include_bytes!("../../../assets/fonts/NotoSans-Regular.ttf"),
            include_bytes!("../../../assets/fonts/NotoSans-Bold.ttf"),
            include_bytes!("../../../assets/fonts/NotoSans-Italic.ttf"),
            include_bytes!("../../../assets/fonts/NotoSans-BoldItalic.ttf"),
            include_bytes!("../../../assets/fonts/NotoSansMono-Regular.ttf"),
            include_bytes!("../../../assets/fonts/NotoSansMono-Bold.ttf"),
        ];
        let faces = data
            .iter()
            .map(|bytes| {
                Font::from_bytes(*bytes, FontSettings::default()).expect("bundled font is valid")
            })
            .collect();
        Fonts { faces }
    }

    pub fn font(&self, face: Face) -> &Font {
        &self.faces[face as usize]
    }

    /// Glyph indices and x offsets (relative to the start) for `text`,
    /// plus the total advance width.
    pub fn shape(&self, face: Face, size: f32, text: &str) -> (Vec<(u16, f32)>, f32) {
        let font = self.font(face);
        let mut x = 0.0;
        let mut prev: Option<u16> = None;
        let mut glyphs = Vec::with_capacity(text.len());
        for ch in text.chars() {
            let gi = font.lookup_glyph_index(ch);
            if let Some(p) = prev {
                x += font.horizontal_kern_indexed(p, gi, size).unwrap_or(0.0);
            }
            glyphs.push((gi, x));
            x += font.metrics_indexed(gi, size).advance_width;
            prev = Some(gi);
        }
        (glyphs, x)
    }

    pub fn width(&self, face: Face, size: f32, text: &str) -> f32 {
        self.shape(face, size, text).1
    }

    /// Distance from the top of a line box to the baseline.
    pub fn baseline(&self, face: Face, size: f32) -> f32 {
        let line = size * LINE_HEIGHT;
        match self.font(face).horizontal_line_metrics(size) {
            Some(m) => (line - (m.ascent - m.descent)) / 2.0 + m.ascent,
            None => size,
        }
    }
}
