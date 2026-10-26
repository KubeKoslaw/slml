//! Loads and caches images referenced by `image(src: ...)`.

use std::collections::HashMap;
use std::path::PathBuf;

use tiny_skia::Pixmap;

/// Resolves `src` relative to the document's directory. Only local PNG files
/// are supported in 0.1; anything else renders as an `alt` placeholder.
pub struct ImageStore {
    base: PathBuf,
    cache: HashMap<String, Option<Pixmap>>,
}

impl ImageStore {
    pub fn new(base: impl Into<PathBuf>) -> Self {
        Self {
            base: base.into(),
            cache: HashMap::new(),
        }
    }

    pub fn get(&mut self, src: &str) -> Option<&Pixmap> {
        let base = &self.base;
        self.cache
            .entry(src.to_string())
            .or_insert_with(|| {
                if src.contains("://") {
                    return None;
                }
                Pixmap::load_png(base.join(src)).ok()
            })
            .as_ref()
    }

    pub fn natural_size(&mut self, src: &str) -> Option<(f32, f32)> {
        self.get(src).map(|p| (p.width() as f32, p.height() as f32))
    }

    /// Forget cached images, e.g. when the document is reloaded.
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}
