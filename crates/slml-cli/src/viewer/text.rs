//! Text selection and find-in-page on top of the layout's text areas.
//! Positions are UTF-16 offsets into [`TextArea::text`], like the renderer's
//! [`TextRun::range`], so the native and web viewers agree on them.

use slml_render::fonts::{Face, Fonts};
use slml_render::layout::{Layout, Rect, TextArea, TextRun};

/// A position between two characters of the document's text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Caret {
    /// Index into [`Layout::text`].
    pub area: usize,
    /// UTF-16 offset into that area's text.
    pub offset: usize,
}

/// A stretch of text between two carets, in either order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub anchor: Caret,
    pub head: Caret,
}

impl Span {
    pub fn ordered(self) -> (Caret, Caret) {
        if self.anchor <= self.head {
            (self.anchor, self.head)
        } else {
            (self.head, self.anchor)
        }
    }

    pub fn is_empty(self) -> bool {
        self.anchor == self.head
    }
}

fn byte_index(text: &str, utf16: usize) -> usize {
    let mut units = 0;
    for (i, c) in text.char_indices() {
        if units >= utf16 {
            return i;
        }
        units += c.len_utf16();
    }
    text.len()
}

fn run_text<'a>(area: &'a TextArea, run: &TextRun) -> &'a str {
    let start = byte_index(&area.text, run.range.start);
    let end = byte_index(&area.text, run.range.end);
    &area.text[start..end]
}

fn face(run: &TextRun) -> Face {
    Face::select(run.bold, run.italic, run.mono)
}

/// Every caret position inside a run, with its x coordinate.
pub(super) fn stops(fonts: &Fonts, area: &TextArea, run: &TextRun) -> Vec<(usize, f32)> {
    let text = run_text(area, run);
    let mut out = vec![(run.range.start, run.rect.x)];
    let mut units = run.range.start;
    for (i, c) in text.char_indices() {
        units += c.len_utf16();
        let end = i + c.len_utf8();
        let x = run.rect.x + fonts.width(face(run), run.size, &text[..end]);
        out.push((units, x.min(run.rect.x + run.rect.w)));
    }
    out
}

pub(super) fn x_of(fonts: &Fonts, area: &TextArea, run: &TextRun, offset: usize) -> f32 {
    stops(fonts, area, run)
        .into_iter()
        .find(|(o, _)| *o >= offset)
        .map_or(run.rect.x + run.rect.w, |(_, x)| x)
}

/// Is there text under the point?
pub fn over_text(layout: &Layout, x: f32, y: f32) -> bool {
    layout
        .text
        .iter()
        .flat_map(|a| &a.runs)
        .any(|r| r.rect.contains(x, y))
}

/// The caret nearest to a point: on the closest line, then the closest
/// character boundary. `None` when the page has no text.
pub fn caret_at(layout: &Layout, fonts: &Fonts, x: f32, y: f32) -> Option<Caret> {
    let distance = |r: &Rect| {
        let dx = (r.x - x).max(x - (r.x + r.w)).max(0.0);
        let dy = (r.y - y).max(y - (r.y + r.h)).max(0.0);
        (dy, dx)
    };
    let (area_index, run) = layout
        .text
        .iter()
        .enumerate()
        .flat_map(|(i, a)| a.runs.iter().map(move |r| (i, r)))
        .min_by(|(_, a), (_, b)| distance(&a.rect).partial_cmp(&distance(&b.rect)).unwrap())?;
    let area = &layout.text[area_index];
    let offset = stops(fonts, area, run)
        .into_iter()
        .min_by(|(_, a), (_, b)| (a - x).abs().partial_cmp(&(b - x).abs()).unwrap())
        .map_or(run.range.start, |(o, _)| o);
    Some(Caret {
        area: area_index,
        offset,
    })
}

/// The word (or run of spaces) around a caret, for double-click selection.
pub fn word_at(layout: &Layout, caret: Caret) -> Span {
    let text = &layout.text[caret.area].text;
    let units: Vec<(usize, char)> = text
        .chars()
        .scan(0, |at, c| {
            let start = *at;
            *at += c.len_utf16();
            Some((start, c))
        })
        .collect();
    let kind = |c: char| c.is_alphanumeric() || c == '_';
    let index = units
        .iter()
        .position(|(o, _)| *o >= caret.offset)
        .unwrap_or(units.len());
    let pick = if index < units.len() {
        index
    } else {
        index.saturating_sub(1)
    };
    let Some(&(_, c)) = units.get(pick) else {
        return Span {
            anchor: caret,
            head: caret,
        };
    };
    let same = |d: char| kind(d) == kind(c) && d.is_whitespace() == c.is_whitespace();
    let mut start = pick;
    while start > 0 && same(units[start - 1].1) {
        start -= 1;
    }
    let mut end = pick + 1;
    while end < units.len() && same(units[end].1) {
        end += 1;
    }
    let offset = |i: usize| units.get(i).map_or(text.encode_utf16().count(), |u| u.0);
    Span {
        anchor: Caret {
            area: caret.area,
            offset: offset(start),
        },
        head: Caret {
            area: caret.area,
            offset: offset(end),
        },
    }
}

/// The whole paragraph around a caret, for triple-click selection.
pub fn paragraph(layout: &Layout, caret: Caret) -> Span {
    let len = layout.text[caret.area].text.encode_utf16().count();
    Span {
        anchor: Caret {
            area: caret.area,
            offset: 0,
        },
        head: Caret {
            area: caret.area,
            offset: len,
        },
    }
}

/// All text on the page, as one selection.
pub fn everything(layout: &Layout) -> Option<Span> {
    let last = layout.text.len().checked_sub(1)?;
    Some(Span {
        anchor: Caret { area: 0, offset: 0 },
        head: Caret {
            area: last,
            offset: layout.text[last].text.encode_utf16().count(),
        },
    })
}

/// Highlight rectangles for a selection or a match, merged per line.
pub fn rects(layout: &Layout, fonts: &Fonts, span: Span) -> Vec<Rect> {
    let (from, to) = span.ordered();
    let mut out: Vec<Rect> = Vec::new();
    for index in from.area..=to.area.min(layout.text.len().saturating_sub(1)) {
        let area = &layout.text[index];
        let start = if index == from.area { from.offset } else { 0 };
        let end = if index == to.area {
            to.offset
        } else {
            usize::MAX
        };
        for run in &area.runs {
            let (a, b) = (start.max(run.range.start), end.min(run.range.end));
            if a >= b {
                continue;
            }
            let x0 = x_of(fonts, area, run, a);
            let x1 = x_of(fonts, area, run, b);
            let rect = Rect::new(x0, run.rect.y, x1 - x0, run.rect.h);
            match out.last_mut() {
                Some(last)
                    if last.y == rect.y
                        && last.h == rect.h
                        && (last.x + last.w - x0).abs() < 1.0 =>
                {
                    last.w = x1 - last.x;
                }
                _ => out.push(rect),
            }
        }
    }
    out
}

/// The selected text, paragraphs separated by line breaks.
pub fn copy(layout: &Layout, span: Span) -> String {
    let (from, to) = span.ordered();
    let mut parts = Vec::new();
    for index in from.area..=to.area.min(layout.text.len().saturating_sub(1)) {
        let text = &layout.text[index].text;
        let start = if index == from.area {
            byte_index(text, from.offset)
        } else {
            0
        };
        let end = if index == to.area {
            byte_index(text, to.offset)
        } else {
            text.len()
        };
        if start < end {
            parts.push(&text[start..end]);
        }
    }
    parts.join("\n")
}

/// Every case-insensitive occurrence of `query`, in reading order.
pub fn find(layout: &Layout, query: &str) -> Vec<Span> {
    let needle: Vec<char> = query.chars().flat_map(char::to_lowercase).collect();
    if needle.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for (index, area) in layout.text.iter().enumerate() {
        // Lowercase per character, remembering where each one came from.
        let mut folded: Vec<(char, usize, usize)> = Vec::new();
        let mut at = 0;
        for c in area.text.chars() {
            let next = at + c.len_utf16();
            folded.extend(c.to_lowercase().map(|l| (l, at, next)));
            at = next;
        }
        let mut i = 0;
        while i + needle.len() <= folded.len() {
            let window = &folded[i..i + needle.len()];
            if window.iter().zip(&needle).all(|((c, ..), n)| c == n) {
                let caret = |offset| Caret {
                    area: index,
                    offset,
                };
                out.push(Span {
                    anchor: caret(window[0].1),
                    head: caret(window[needle.len() - 1].2),
                });
                i += needle.len();
            } else {
                i += 1;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use slml_render::Renderer;

    fn lay(src: &str) -> (Layout, Renderer) {
        let doc = slml_syntax::compile(src).unwrap();
        let mut renderer = Renderer::new(".");
        let layout = renderer.layout(&doc, 400.0, 0.0);
        (layout, renderer)
    }

    #[test]
    fn selection_copies_across_paragraphs_and_keeps_utf16_offsets() {
        let (l, r) =
            lay(r#"slml 0.1 page(padding: 10) { text "Zażółć gęślą" text "jaźń 😀 end" }"#);
        let start = caret_at(&l, r.fonts(), 0.0, 0.0).unwrap();
        assert_eq!(start, Caret { area: 0, offset: 0 });
        let span = Span {
            anchor: Caret { area: 0, offset: 7 },
            head: Caret { area: 1, offset: 7 },
        };
        assert_eq!(copy(&l, span), "gęślą\njaźń 😀");
        let reversed = Span {
            anchor: span.head,
            head: span.anchor,
        };
        assert_eq!(copy(&l, reversed), copy(&l, span));
        let all = everything(&l).unwrap();
        assert_eq!(copy(&l, all), "Zażółć gęślą\njaźń 😀 end");
        // One highlight per line, starting at the selected character.
        let boxes = rects(&l, r.fonts(), span);
        assert_eq!(boxes.len(), 2);
        assert!(boxes[0].x > l.text[0].runs[0].rect.x);
        assert_eq!(boxes[1].x, l.text[1].runs[0].rect.x);
    }

    #[test]
    fn caret_snaps_to_the_nearest_character_boundary() {
        let (l, r) = lay(r#"slml 0.1 page(padding: 0) { text "abc def" }"#);
        let run = &l.text[0].runs[0];
        let mid = run.rect.y + run.rect.h / 2.0;
        let end_of_abc = caret_at(&l, r.fonts(), run.rect.x + run.rect.w - 1.0, mid).unwrap();
        assert_eq!(end_of_abc.offset, 3);
        // Far right of the line: the end of the text.
        assert_eq!(caret_at(&l, r.fonts(), 390.0, mid).unwrap().offset, 7);
        // Below the text: still on the last line.
        assert_eq!(caret_at(&l, r.fonts(), 0.0, 500.0).unwrap().offset, 0);
        assert!(over_text(&l, run.rect.x + 1.0, mid));
        assert!(!over_text(&l, 390.0, mid));
    }

    #[test]
    fn words_and_matches() {
        let (l, _) = lay(r#"slml 0.1 page { text "Hello, wide World" text "world again" }"#);
        let word = word_at(&l, Caret { area: 0, offset: 9 });
        assert_eq!(copy(&l, word), "wide");
        let whole = paragraph(&l, Caret { area: 1, offset: 3 });
        assert_eq!(copy(&l, whole), "world again");
        let found = find(&l, "WORLD");
        assert_eq!(found.len(), 2);
        assert_eq!(copy(&l, found[0]), "World");
        assert_eq!(found[1].anchor, Caret { area: 1, offset: 0 });
        assert!(find(&l, "").is_empty());
        assert!(find(&l, "missing").is_empty());
    }
}
