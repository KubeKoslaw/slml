//! Loads and caches images referenced by `image(src: ...)`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tiny_skia::{Pixmap, PremultipliedColorU8};
use zune_jpeg::JpegDecoder;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;

/// Resolves `src` relative to the document's directory, or to the site root
/// for paths starting with `/`. PNG and JPEG are supported; anything else
/// (including remote URLs, for now) renders as an `alt` placeholder.
pub struct ImageStore {
    base: PathBuf,
    root: PathBuf,
    cache: HashMap<String, Option<Pixmap>>,
}

impl ImageStore {
    /// Images resolve against `base`; `/` paths resolve against `base` too.
    pub fn new(base: impl Into<PathBuf>) -> Self {
        let base = base.into();
        Self::with_root(base.clone(), base)
    }

    pub fn with_root(base: impl Into<PathBuf>, root: impl Into<PathBuf>) -> Self {
        Self {
            base: base.into(),
            root: root.into(),
            cache: HashMap::new(),
        }
    }

    fn resolve(&self, src: &str) -> PathBuf {
        match src.strip_prefix('/') {
            Some(rest) => self.root.join(rest),
            None => self.base.join(src),
        }
    }

    pub fn get(&mut self, src: &str) -> Option<&Pixmap> {
        if !self.cache.contains_key(src) {
            let loaded = if src.contains("://") {
                None
            } else {
                load(&self.resolve(src))
            };
            self.cache.insert(src.to_string(), loaded);
        }
        self.cache.get(src).and_then(Option::as_ref)
    }

    pub fn natural_size(&mut self, src: &str) -> Option<(f32, f32)> {
        self.get(src).map(|p| (p.width() as f32, p.height() as f32))
    }

    /// Supplies image bytes from a host (for example a browser's fetch API).
    pub fn insert_encoded(&mut self, src: &str, data: &[u8]) -> bool {
        let image = if data.starts_with(b"\x89PNG\r\n\x1a\n") {
            Pixmap::decode_png(data).ok()
        } else {
            load_jpeg(data)
        };
        let loaded = image.is_some();
        self.cache.insert(src.to_string(), image);
        loaded
    }

    /// Forget cached images, e.g. when the document is reloaded.
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

fn load(path: &Path) -> Option<Pixmap> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "png" => Pixmap::load_png(path).ok(),
        "jpg" | "jpeg" => load_jpeg(&std::fs::read(path).ok()?),
        _ => None,
    }
}

fn load_jpeg(data: &[u8]) -> Option<Pixmap> {
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGB);
    let mut decoder = JpegDecoder::new_with_options(data, options);
    let rgb = decoder.decode().ok()?;
    let info = decoder.info()?;
    let (w, h) = (info.width as u32, info.height as u32);
    let mut pixmap = Pixmap::new(w, h)?;
    for (dst, px) in pixmap.pixels_mut().iter_mut().zip(rgb.as_chunks::<3>().0) {
        *dst = PremultipliedColorU8::from_rgba(px[0], px[1], px[2], 255)?;
    }
    Some(pixmap)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_supplied_images_are_used_without_file_access() {
        let bytes = include_bytes!("../../../examples/site/logo.png");
        let mut images = ImageStore::new("/not-a-real-directory");
        assert!(images.insert_encoded("logo.png", bytes));
        assert!(images.natural_size("logo.png").is_some());
        assert!(!images.insert_encoded("logo.png", b"not an image"));
        assert!(images.get("logo.png").is_none());
    }
}
