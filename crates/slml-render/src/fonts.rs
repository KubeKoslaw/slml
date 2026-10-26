//! Bundled fonts (Noto, SIL Open Font License) and text measurement.
//!
//! Fonts are compiled into the binary so a document renders identically on
//! every machine, which is one of SLML's goals. Characters missing from the
//! main face are taken from a fixed chain of fallback faces (symbols, math,
//! monochrome emoji).

use fontdue::{Font, FontSettings};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Face {
    Regular = 0,
    Bold = 1,
    Italic = 2,
    BoldItalic = 3,
    Mono = 4,
    MonoBold = 5,
    Symbols2 = 6,
    Symbols = 7,
    Math = 8,
    Emoji = 9,
}

/// Fallback order for characters the main face lacks.
const FALLBACKS: [Face; 4] = [Face::Symbols2, Face::Symbols, Face::Math, Face::Emoji];

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

/// Every bundled font file: (file name, CSS family, weight, italic, bytes).
/// Backends that ship fonts (such as the HTML backend) use this list.
pub const FONT_FILES: [(&str, &str, u16, bool, &[u8]); 10] = [
    (
        "NotoSans-Regular.ttf",
        "SLML Sans",
        400,
        false,
        include_bytes!("../../../assets/fonts/NotoSans-Regular.ttf"),
    ),
    (
        "NotoSans-Bold.ttf",
        "SLML Sans",
        700,
        false,
        include_bytes!("../../../assets/fonts/NotoSans-Bold.ttf"),
    ),
    (
        "NotoSans-Italic.ttf",
        "SLML Sans",
        400,
        true,
        include_bytes!("../../../assets/fonts/NotoSans-Italic.ttf"),
    ),
    (
        "NotoSans-BoldItalic.ttf",
        "SLML Sans",
        700,
        true,
        include_bytes!("../../../assets/fonts/NotoSans-BoldItalic.ttf"),
    ),
    (
        "NotoSansMono-Regular.ttf",
        "SLML Mono",
        400,
        false,
        include_bytes!("../../../assets/fonts/NotoSansMono-Regular.ttf"),
    ),
    (
        "NotoSansMono-Bold.ttf",
        "SLML Mono",
        700,
        false,
        include_bytes!("../../../assets/fonts/NotoSansMono-Bold.ttf"),
    ),
    (
        "NotoSansSymbols2-Regular.ttf",
        "SLML Symbols 2",
        400,
        false,
        include_bytes!("../../../assets/fonts/NotoSansSymbols2-Regular.ttf"),
    ),
    (
        "NotoSansSymbols-Regular.ttf",
        "SLML Symbols",
        400,
        false,
        include_bytes!("../../../assets/fonts/NotoSansSymbols-Regular.ttf"),
    ),
    (
        "NotoSansMath-Regular.ttf",
        "SLML Math",
        400,
        false,
        include_bytes!("../../../assets/fonts/NotoSansMath-Regular.ttf"),
    ),
    (
        "NotoEmoji-Regular.ttf",
        "SLML Emoji",
        400,
        false,
        include_bytes!("../../../assets/fonts/NotoEmoji-Regular.ttf"),
    ),
];

/// One positioned glyph. `x` is relative to the start of its run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Glyph {
    pub face: Face,
    pub id: u16,
    pub x: f32,
}

pub struct Fonts {
    faces: Vec<Font>,
}

/// Distance between baselines, as a multiple of the font size.
pub const LINE_HEIGHT: f32 = 1.45;

/// Characters that only modify their neighbour (variation selectors,
/// zero-width joiner) and take no space.
fn is_invisible(c: char) -> bool {
    matches!(c, '\u{FE00}'..='\u{FE0F}' | '\u{200D}' | '\u{200B}')
}

impl Fonts {
    pub fn load() -> Fonts {
        let faces = FONT_FILES
            .iter()
            .map(|(_, _, _, _, bytes)| {
                Font::from_bytes(*bytes, FontSettings::default()).expect("bundled font is valid")
            })
            .collect();
        Fonts { faces }
    }

    pub fn font(&self, face: Face) -> &Font {
        &self.faces[face as usize]
    }

    /// The face and glyph used for `ch`: the main face if it has the
    /// character, otherwise the first fallback that does.
    fn glyph_for(&self, face: Face, ch: char) -> (Face, u16) {
        let gi = self.font(face).lookup_glyph_index(ch);
        if gi != 0 || ch.is_whitespace() {
            return (face, gi);
        }
        FALLBACKS
            .iter()
            .map(|f| (*f, self.font(*f).lookup_glyph_index(ch)))
            .find(|(_, gi)| *gi != 0)
            .unwrap_or((face, 0))
    }

    /// Positioned glyphs for `text` plus the total advance width.
    pub fn shape(&self, face: Face, size: f32, text: &str) -> (Vec<Glyph>, f32) {
        let mut x = 0.0;
        let mut prev: Option<(Face, u16)> = None;
        let mut glyphs = Vec::with_capacity(text.len());
        for ch in text.chars() {
            if is_invisible(ch) {
                continue;
            }
            let (f, gi) = self.glyph_for(face, ch);
            let font = self.font(f);
            if let Some((pf, pg)) = prev
                && pf == f
            {
                x += font.horizontal_kern_indexed(pg, gi, size).unwrap_or(0.0);
            }
            glyphs.push(Glyph { face: f, id: gi, x });
            x += font.metrics_indexed(gi, size).advance_width;
            prev = Some((f, gi));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_characters_come_from_fallbacks() {
        let fonts = Fonts::load();
        let (glyphs, _) = fonts.shape(Face::Regular, 16.0, "a★→✓😀\u{FE0F}");
        let faces: Vec<Face> = glyphs.iter().map(|g| g.face).collect();
        assert_eq!(faces[0], Face::Regular);
        assert!(glyphs.iter().all(|g| g.id != 0), "{glyphs:?}");
        assert!(faces[1..].iter().all(|f| *f != Face::Regular), "{faces:?}");
        assert_eq!(glyphs.len(), 5, "variation selector takes no glyph");
    }
}
