//! # slml-render
//!
//! Lays out and paints a validated SLML document. Pure CPU rendering, no
//! platform dependencies: the same code renders to a PNG, a window or (later)
//! a WebAssembly canvas.

pub mod fonts;
pub mod images;
pub mod inspect;
pub mod layout;
pub mod paint;

use std::path::Path;

use slml_syntax::model::Document;
pub use tiny_skia::Pixmap;

use fonts::Fonts;
use images::ImageStore;
use layout::Item;
pub use layout::{ElementBox, Layout, LinkArea};
use paint::Painter;

/// Owns everything that is expensive to create: fonts, glyph and image caches.
pub struct Renderer {
    fonts: Fonts,
    painter: Painter,
    pub images: ImageStore,
}

impl Renderer {
    /// `base_dir` is where relative image paths are resolved.
    pub fn new(base_dir: impl AsRef<Path>) -> Self {
        Self {
            fonts: Fonts::load(),
            painter: Painter::default(),
            images: ImageStore::new(base_dir.as_ref()),
        }
    }

    /// Lays the page out for a viewport `width` logical pixels wide. The page
    /// is at least `min_height` tall.
    pub fn layout(&mut self, doc: &Document, width: f32, min_height: f32) -> Layout {
        layout::layout(doc, &self.fonts, &mut self.images, width, min_height)
    }

    pub fn paint(&mut self, layout: &Layout, target: &mut Pixmap, scroll_y: f32, scale: f32) {
        self.painter.paint(
            layout,
            &self.fonts,
            &mut self.images,
            target,
            scroll_y,
            scale,
        );
    }

    /// Draws extra items over what is already in `target`, e.g. a host's
    /// highlights or its own interface. Coordinates work like in
    /// [`Renderer::paint`].
    pub fn paint_items(&mut self, items: &[Item], target: &mut Pixmap, scroll_y: f32, scale: f32) {
        self.painter.draw(
            items,
            &self.fonts,
            &mut self.images,
            target,
            scroll_y,
            scale,
        );
    }

    /// The bundled fonts, for hosts that measure or shape their own text.
    pub fn fonts(&self) -> &Fonts {
        &self.fonts
    }

    /// Renders the whole page into one image.
    pub fn render_page(&mut self, doc: &Document, width: u32, scale: f32) -> Pixmap {
        let layout = self.layout(doc, width as f32, 0.0);
        let w = ((width as f32) * scale).ceil().max(1.0) as u32;
        let h = (layout.height * scale).ceil().max(1.0) as u32;
        let mut pixmap = Pixmap::new(w, h).expect("page size is positive");
        self.paint(&layout, &mut pixmap, 0.0, scale);
        pixmap
    }
}
