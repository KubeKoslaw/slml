//! The element inspector (F12): highlights the box under the pointer and
//! shows the element's name, resolved attributes, box and source position
//! in a panel beside the page.

use std::path::Path;

use slml_render::ElementBox;
use slml_render::fonts::Face;
use slml_render::layout::{Layout, Rect};
use slml_syntax::model::{Color, Document};

use super::source::Sources;
use super::ui::{self, Ui};

/// Width of the panel in window pixels.
pub const PANEL: f32 = 340.0;

const CONTENT: Color = Color(0x6fa8dc66);
const PADDING: Color = Color(0x93c47d66);
const OUTLINE: Color = Color(0x2457d6ff);

#[derive(Default)]
pub struct Inspector {
    /// The element under the pointer (a layout path).
    pub hover: Option<Vec<usize>>,
    /// The element clicked last; shown until Esc or another click.
    pub pinned: Option<Vec<usize>>,
    /// Source positions of the page, read when first needed.
    pub sources: Option<Sources>,
}

impl Inspector {
    /// The element the panel describes.
    pub fn shown(&self) -> Option<&[usize]> {
        self.pinned.as_deref().or(self.hover.as_deref())
    }

    /// Click on the page: pin the element, or unpin it when clicked again.
    pub fn click(&mut self) {
        self.pinned = match (&self.pinned, &self.hover) {
            (Some(p), Some(h)) if p == h => None,
            (_, hover) => hover.clone(),
        };
    }
}

fn padding_bands(e: &ElementBox) -> [Rect; 4] {
    let (r, c) = (e.rect, e.content);
    [
        Rect::new(r.x, r.y, r.w, c.y - r.y),
        Rect::new(r.x, c.y + c.h, r.w, r.y + r.h - c.y - c.h),
        Rect::new(r.x, c.y, c.x - r.x, c.h),
        Rect::new(c.x + c.w, c.y, r.x + r.w - c.x - c.w, c.h),
    ]
}

/// Highlights in page coordinates: content in blue, padding (and border)
/// in green, an outline around the box.
pub fn highlight(ui: &mut Ui, layout: &Layout, inspector: &Inspector) {
    if let Some(e) = inspector.shown().and_then(|p| layout.element(p)) {
        ui.fill(e.content, CONTENT, 0.0);
        for band in padding_bands(e) {
            if band.w > 0.0 && band.h > 0.0 {
                ui.fill(band, PADDING, 0.0);
            }
        }
        ui.border(e.rect, 1.0, OUTLINE, 0.0);
    }
    // While an element is pinned, the one under the pointer is outlined.
    if let (Some(_), Some(hover)) = (&inspector.pinned, &inspector.hover)
        && inspector.pinned.as_ref() != Some(hover)
        && let Some(e) = layout.element(hover)
    {
        ui.border(e.rect, 1.0, Color(0x2457d699), 0.0);
    }
}

fn size(r: Rect) -> String {
    format!("{} × {}", round(r.w), round(r.h))
}

fn round(v: f32) -> String {
    let r = (v * 10.0).round() / 10.0;
    if r.fract() == 0.0 {
        format!("{r:.0}")
    } else {
        format!("{r:.1}")
    }
}

/// The small name tag next to the highlighted box. `rect` is the box in
/// window coordinates; `view` is the page viewport.
pub fn tag(ui: &mut Ui, e: &ElementBox, rect: Rect, view: (f32, f32)) {
    let text = format!("{}  {}", e.name, size(e.rect));
    let size = 12.0;
    let w = ui.width(&text, size, Face::Mono) + 12.0;
    let h = Ui::line(size) + 4.0;
    let x = rect.x.clamp(2.0, (view.0 - w - 2.0).max(2.0));
    let y = if rect.y - h - 4.0 >= 0.0 {
        rect.y - h - 4.0
    } else {
        (rect.y + rect.h + 4.0).min(view.1 - h - 2.0)
    };
    let r = Rect::new(x, y, w, h);
    ui.fill(r, Color(0x1b1b1fe6), 3.0);
    ui.text(x + 6.0, y + 2.0, &text, size, Color::WHITE, Face::Mono);
}

/// The panel at `x` (window coordinates), `h` tall.
#[allow(clippy::too_many_arguments)]
pub fn panel(
    ui: &mut Ui,
    x: f32,
    width: f32,
    h: f32,
    layout: &Layout,
    doc: &Document,
    inspector: &Inspector,
    root: &Path,
) {
    ui.fill(Rect::new(x, 0.0, width, h), ui::PANEL, 0.0);
    ui.fill(Rect::new(x, 0.0, 1.0, h), ui::LINE, 0.0);
    let pad = 16.0;
    let (left, right) = (x + pad, x + width - pad);
    let room = right - left;
    let mut y = 12.0;
    ui.text(left, y, "Inspector", 13.0, ui::MUTED, Face::Bold);
    let hint = "F12 closes";
    let hint_w = ui.width(hint, 12.0, Face::Regular);
    ui.text(
        right - hint_w,
        y + 1.0,
        hint,
        12.0,
        ui::MUTED,
        Face::Regular,
    );
    y += 30.0;

    let Some(path) = inspector.shown() else {
        for line in [
            "Point at an element to inspect it.",
            "Click to pin it, Esc to unpin.",
        ] {
            ui.text(left, y, line, 13.0, ui::INK, Face::Regular);
            y += Ui::line(13.0) + 2.0;
        }
        return;
    };
    let (Some(e), Some(element)) = (
        layout.element(path),
        slml_render::inspect::element(doc, path),
    ) else {
        return;
    };

    ui.text_fit(left, y, room, e.name, 18.0, ui::INK, Face::MonoBold);
    if inspector.pinned.as_deref() == Some(path) {
        let pin = "pinned";
        let w = ui.width(pin, 11.0, Face::Bold) + 10.0;
        let r = Rect::new(right - w, y + 4.0, w, Ui::line(11.0) + 2.0);
        ui.fill(r, Color(0xdbe5fbff), 99.0);
        ui.text(r.x + 5.0, r.y + 1.0, pin, 11.0, ui::ACCENT, Face::Bold);
    }
    y += Ui::line(18.0) + 2.0;

    // page › column › row
    let crumbs: Vec<&str> = (0..=path.len())
        .filter_map(|n| layout.element(&path[..n]).map(|e| e.name))
        .collect();
    ui.text_fit(
        left,
        y,
        room,
        &crumbs.join(" › "),
        12.0,
        ui::MUTED,
        Face::Regular,
    );
    y += Ui::line(12.0) + 4.0;

    let location = inspector.sources.as_ref().and_then(|s| s.locate(path));
    let source = match location {
        Some(l) => {
            let file = crate::build::normalize(&l.file);
            let root = crate::build::normalize(root);
            let shown = file.strip_prefix(&root).unwrap_or(&file).display();
            format!("{shown}:{}:{}", l.line, l.col)
        }
        None => "source position not available".into(),
    };
    ui.text_fit(left, y, room, &source, 12.5, ui::ACCENT, Face::Mono);
    y += Ui::line(12.5) + 12.0;

    let row = |ui: &mut Ui, y: &mut f32, name: &str, value: &str| {
        let key_w = 104.0;
        ui.text_fit(left, *y, key_w - 8.0, name, 12.5, ui::MUTED, Face::Mono);
        ui.text_fit(
            left + key_w,
            *y,
            room - key_w,
            value,
            12.5,
            ui::INK,
            Face::Mono,
        );
        *y += Ui::line(12.5) + 1.0;
    };
    let section = |ui: &mut Ui, y: &mut f32, title: &str| {
        ui.fill(Rect::new(left, *y, room, 1.0), ui::LINE, 0.0);
        *y += 8.0;
        ui.text(left, *y, title, 12.0, ui::MUTED, Face::Bold);
        *y += Ui::line(12.0) + 4.0;
    };

    section(ui, &mut y, "Box");
    let (r, c) = (e.rect, e.content);
    row(ui, &mut y, "border box", &size(r));
    row(
        ui,
        &mut y,
        "position",
        &format!("{}, {}", round(r.x), round(r.y)),
    );
    row(ui, &mut y, "content", &size(c));
    let insets = |a: f32, b: f32| round((b - a).max(0.0));
    row(
        ui,
        &mut y,
        "insets",
        &format!(
            "{} {} {} {}",
            insets(r.y, c.y),
            insets(c.x + c.w, r.x + r.w),
            insets(c.y + c.h, r.y + r.h),
            insets(r.x, c.x),
        ),
    );
    y += 8.0;

    section(ui, &mut y, "Attributes");
    let attrs = element.attributes();
    let line = Ui::line(12.5) + 1.0;
    let fits = ((h - y - 12.0) / line).floor().max(0.0) as usize;
    for (i, (name, value)) in attrs.iter().enumerate() {
        if i + 1 == fits && attrs.len() > fits {
            row(ui, &mut y, "…", &format!("{} more", attrs.len() - i));
            break;
        }
        row(ui, &mut y, name, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clicking_pins_and_unpins() {
        let mut i = Inspector {
            hover: Some(vec![0, 1]),
            ..Inspector::default()
        };
        assert_eq!(i.shown(), Some(&[0, 1][..]));
        i.click();
        assert_eq!(i.pinned, Some(vec![0, 1]));
        i.hover = Some(vec![2]);
        assert_eq!(i.shown(), Some(&[0, 1][..]), "pinned wins over hover");
        i.click();
        assert_eq!(i.pinned, Some(vec![2]), "another element moves the pin");
        i.click();
        assert_eq!(i.pinned, None, "the same element unpins");
    }

    #[test]
    fn padding_bands_surround_the_content() {
        let e = ElementBox {
            name: "column",
            path: vec![],
            rect: Rect::new(10.0, 10.0, 100.0, 50.0),
            content: Rect::new(20.0, 15.0, 80.0, 40.0),
            radius: 0.0,
        };
        let area: f32 = padding_bands(&e).iter().map(|b| b.w * b.h).sum();
        assert_eq!(area, 100.0 * 50.0 - 80.0 * 40.0);
    }
}
