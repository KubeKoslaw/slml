//! Small, dependency-free host interface over the actual SLML engine.
//!
//! The host writes to `slml_input(len)` and then calls `slml_file`,
//! `slml_image` (both `name\0bytes`) or `slml_open` (a UTF-8 file name).
//! Output pointers remain valid until the next mutating call. All memory
//! belongs to this module; there are no host-supplied pointers or unsafe reads.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write;

use slml_render::images::ImageStore;
use slml_render::{Layout, Pixmap, Renderer};
use slml_syntax::model::{Container, ContainerKind, Document, FieldKind, Inline, Method, Node};

#[derive(Default)]
struct Files {
    sources: HashMap<String, String>,
    requested: Vec<String>,
}

impl slml_syntax::Loader for Files {
    fn load(&mut self, from: &str, path: &str) -> Result<(String, String), String> {
        let full = if path.starts_with('/') {
            path.to_string()
        } else {
            format!("{}/{}", from.rsplit_once('/').map_or("", |(p, _)| p), path)
        };
        let name = format!(
            "/{}",
            full.split('/')
                .filter(|s| !s.is_empty() && *s != ".")
                .collect::<Vec<_>>()
                .join("/")
        );
        match self.sources.get(&name) {
            Some(source) => Ok((name, source.clone())),
            None => {
                if !self.requested.contains(&name) {
                    self.requested.push(name.clone());
                }
                Err("file has not been supplied by the browser".into())
            }
        }
    }
}

struct Engine {
    input: Vec<u8>,
    files: Files,
    renderer: Renderer,
    doc: Option<Document>,
    layout: Option<Layout>,
    size: (u32, u32),
    pixels: Vec<u8>,
    info: String,
}

impl Engine {
    fn new() -> Self {
        let mut renderer = Renderer::new("/");
        renderer.images = ImageStore::new("/");
        Self {
            input: Vec::new(),
            files: Files::default(),
            renderer,
            doc: None,
            layout: None,
            size: (0, 0),
            pixels: Vec::new(),
            info: "{}".into(),
        }
    }

    fn error(&mut self, message: &str) {
        self.doc = None;
        self.layout = None;
        self.pixels.clear();
        self.info = format!("{{\"error\":{}}}", json_string(message));
    }

    fn open(&mut self, name: &str) -> bool {
        self.doc = None;
        self.layout = None;
        self.pixels.clear();
        self.files.requested.clear();
        let Some(source) = self.files.sources.get(name).cloned() else {
            self.error("the main SLML file has not been loaded");
            return false;
        };
        let compiled = slml_syntax::compile_with(name, &source, &mut self.files);
        match compiled.document {
            Ok(doc) => {
                self.doc = Some(doc);
                self.metadata();
                true
            }
            Err(errors) => {
                let error = compiled.sources.render_all(&errors);
                self.info = format!(
                    "{{\"error\":{},\"requests\":[{}]}}",
                    json_string(&error),
                    self.files
                        .requested
                        .iter()
                        .map(|s| json_string(s))
                        .collect::<Vec<_>>()
                        .join(",")
                );
                false
            }
        }
    }

    fn metadata(&mut self) {
        let Some(doc) = &self.doc else { return };
        let mut images = Vec::new();
        collect_images(&doc.page, &mut images);
        self.info = format!(
            "{{\"title\":{},\"images\":[{}]",
            json_string(doc.meta.title.as_deref().unwrap_or("SLML")),
            images
                .iter()
                .map(|s| json_string(s))
                .collect::<Vec<_>>()
                .join(",")
        );
        if let Some(layout) = &self.layout {
            let _ = write!(self.info, ",\"height\":{},\"links\":[", layout.height);
            for (i, link) in layout.links.iter().enumerate() {
                if i > 0 {
                    self.info.push(',');
                }
                let r = link.rect;
                let _ = write!(
                    self.info,
                    "{{\"x\":{},\"y\":{},\"w\":{},\"h\":{},\"to\":{}}}",
                    r.x,
                    r.y,
                    r.w,
                    r.h,
                    json_string(&link.target)
                );
            }
            self.info.push_str("],\"anchors\":{");
            for (i, (id, y)) in layout.anchors.iter().enumerate() {
                if i > 0 {
                    self.info.push(',');
                }
                let _ = write!(self.info, "{}:{y}", json_string(id));
            }
            self.info.push('}');
            self.info.push_str(",\"controls\":[");
            for (i, area) in layout.controls.iter().enumerate() {
                if i > 0 {
                    self.info.push(',');
                }
                self.info.push_str(&control_json(doc, area));
            }
            self.info.push(']');
        }
        self.info.push('}');
    }

    fn render(&mut self, width: u32, height: u32, scale: f32, scroll: f32) -> bool {
        if width == 0
            || height == 0
            || width > 8192
            || height > 8192
            || !scale.is_finite()
            || !(0.25..=4.0).contains(&scale)
            || !scroll.is_finite()
        {
            self.error("invalid browser viewport dimensions");
            return false;
        }
        let (w, h) = (
            (width as f32 * scale).ceil() as u32,
            (height as f32 * scale).ceil() as u32,
        );
        if u64::from(w) * u64::from(h) > 32_000_000 {
            self.error("the browser viewport exceeds 32 million device pixels");
            return false;
        }
        let Some(doc) = &self.doc else { return false };
        if self.layout.is_none() || self.size != (width, height) {
            self.layout = Some(self.renderer.layout(doc, width as f32, height as f32));
            self.size = (width, height);
            self.metadata();
        }
        let Some(mut pixmap) = Pixmap::new(w, h) else {
            return false;
        };
        self.renderer.paint(
            self.layout.as_ref().unwrap(),
            &mut pixmap,
            scroll.max(0.0),
            scale,
        );
        self.pixels = pixmap.take();
        // ImageData uses straight alpha; tiny-skia stores premultiplied pixels.
        for p in self.pixels.as_chunks_mut::<4>().0 {
            if p[3] != 0 && p[3] != 255 {
                for i in 0..3 {
                    p[i] = (u32::from(p[i]) * 255 / u32::from(p[3])).min(255) as u8;
                }
            }
        }
        true
    }
}

fn control_json(doc: &Document, area: &slml_render::layout::ControlArea) -> String {
    let mut container = &doc.page;
    let mut form = None;
    let mut node = None;
    for (depth, &index) in area.path.iter().enumerate() {
        node = container.children.get(index);
        if let Some(Node::Container(child)) = node {
            if let ContainerKind::Form(spec) = &child.kind {
                form = Some((
                    spec,
                    area.path[..=depth]
                        .iter()
                        .map(usize::to_string)
                        .collect::<Vec<_>>()
                        .join("."),
                ));
            }
            container = child;
        }
    }
    let id = area
        .path
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(".");
    let r = area.rect;
    let b = area.input_rect;
    let mut out = format!(
        "{{\"id\":{},\"x\":{},\"y\":{},\"w\":{},\"h\":{},\"box\":{{\"x\":{},\"y\":{},\"w\":{},\"h\":{}}},\"size\":{},\"color\":\"#{:08x}\",\"bold\":{},\"italic\":{},\"mono\":{}",
        json_string(&id),
        r.x,
        r.y,
        r.w,
        r.h,
        b.x,
        b.y,
        b.w,
        b.h,
        area.size,
        area.color.0,
        area.bold,
        area.italic,
        area.mono
    );
    if let Some((spec, form_id)) = form {
        let method = if spec.method == Method::Post {
            "post"
        } else {
            "get"
        };
        let _ = write!(
            out,
            ",\"form\":{{\"id\":{},\"to\":{},\"method\":{}}}",
            json_string(&form_id),
            json_string(&spec.to),
            json_string(method)
        );
    }
    match node {
        Some(Node::Field(field)) => {
            let _ = write!(
                out,
                ",\"name\":{},\"label\":{},\"required\":{},\"fill\":\"#{:08x}\",\"border\":{},\"borderColor\":\"#{:08x}\",\"radius\":{}",
                json_string(&field.name),
                json_string(&field.label),
                field.required,
                field
                    .frame
                    .fill
                    .unwrap_or(slml_syntax::model::Color::WHITE)
                    .0,
                field.frame.border.max(1.0),
                field
                    .frame
                    .border_color
                    .unwrap_or(slml_syntax::model::Color(0xc4c4ccff))
                    .0,
                if field.frame.radius > 0.0 {
                    field.frame.radius
                } else {
                    6.0
                }
            );
            match &field.kind {
                FieldKind::Input {
                    kind,
                    placeholder,
                    value,
                } => {
                    let _ = write!(
                        out,
                        ",\"kind\":{},\"value\":{},\"placeholder\":{}",
                        json_string(kind.name()),
                        json_string(value.as_deref().unwrap_or("")),
                        json_string(placeholder.as_deref().unwrap_or(""))
                    );
                }
                FieldKind::TextArea {
                    rows,
                    placeholder,
                    value,
                } => {
                    let _ = write!(
                        out,
                        ",\"kind\":\"textarea\",\"rows\":{rows},\"value\":{},\"placeholder\":{}",
                        json_string(value.as_deref().unwrap_or("")),
                        json_string(placeholder.as_deref().unwrap_or(""))
                    );
                }
                FieldKind::Checkbox { checked } => {
                    let _ = write!(out, ",\"kind\":\"checkbox\",\"checked\":{checked}");
                }
                FieldKind::Select { options } => {
                    out.push_str(",\"kind\":\"select\",\"options\":[");
                    for (i, option) in options.iter().enumerate() {
                        if i > 0 {
                            out.push(',');
                        }
                        let _ = write!(
                            out,
                            "{{\"label\":{},\"value\":{},\"selected\":{}}}",
                            json_string(&option.label),
                            json_string(option.value.as_deref().unwrap_or(&option.label)),
                            option.selected
                        );
                    }
                    out.push(']');
                }
            }
        }
        Some(Node::Text(text)) => {
            let _ = write!(
                out,
                ",\"kind\":\"submit\",\"label\":{}",
                json_string(&plain_text(&text.content))
            );
        }
        _ => {}
    }
    out.push('}');
    out
}

fn plain_text(content: &[Inline]) -> String {
    content
        .iter()
        .map(|i| match i {
            Inline::Text(s) => s.clone(),
            Inline::Span(s) => plain_text(&s.content),
        })
        .collect()
}

fn collect_images(c: &Container, images: &mut Vec<String>) {
    for node in &c.children {
        match node {
            Node::Container(c) => collect_images(c, images),
            Node::Image(i) if !images.contains(&i.src) => images.push(i.src.clone()),
            _ => {}
        }
    }
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

thread_local! { static ENGINE: RefCell<Engine> = RefCell::new(Engine::new()); }

#[unsafe(no_mangle)]
pub extern "C" fn slml_input(len: usize) -> *mut u8 {
    ENGINE.with_borrow_mut(|e| {
        e.input.resize(len, 0);
        e.input.as_mut_ptr()
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn slml_reset() {
    ENGINE.with_borrow_mut(|e| {
        e.files = Files::default();
        e.renderer.images.clear();
        e.doc = None;
        e.layout = None;
        e.pixels.clear();
        e.info = "{}".into();
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn slml_file() -> bool {
    ENGINE.with_borrow_mut(|e| {
        let Some(split) = e.input.iter().position(|b| *b == 0) else {
            return false;
        };
        let (Ok(name), Ok(source)) = (
            std::str::from_utf8(&e.input[..split]),
            std::str::from_utf8(&e.input[split + 1..]),
        ) else {
            return false;
        };
        e.files.sources.insert(name.to_string(), source.to_string());
        true
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn slml_image() -> bool {
    ENGINE.with_borrow_mut(|e| {
        let Some(split) = e.input.iter().position(|b| *b == 0) else {
            return false;
        };
        let Ok(name) = std::str::from_utf8(&e.input[..split]) else {
            return false;
        };
        let ok = e
            .renderer
            .images
            .insert_encoded(name, &e.input[split + 1..]);
        e.layout = None;
        ok
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn slml_open() -> bool {
    ENGINE.with_borrow_mut(|e| {
        let Ok(name) = String::from_utf8(e.input.clone()) else {
            e.error("the file name is not UTF-8");
            return false;
        };
        e.open(&name)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn slml_render(width: u32, height: u32, scale: f32, scroll: f32) -> bool {
    ENGINE.with_borrow_mut(|e| e.render(width, height, scale, scroll))
}

#[unsafe(no_mangle)]
pub extern "C" fn slml_info() -> *const u8 {
    ENGINE.with_borrow(|e| e.info.as_ptr())
}
#[unsafe(no_mangle)]
pub extern "C" fn slml_info_len() -> usize {
    ENGINE.with_borrow(|e| e.info.len())
}
#[unsafe(no_mangle)]
pub extern "C" fn slml_pixels() -> *const u8 {
    ENGINE.with_borrow(|e| e.pixels.as_ptr())
}
#[unsafe(no_mangle)]
pub extern "C" fn slml_pixels_len() -> usize {
    ENGINE.with_borrow(|e| e.pixels.len())
}

/// Font data comes from the same embedded files as the canvas renderer.
#[unsafe(no_mangle)]
pub extern "C" fn slml_font(index: usize) -> *const u8 {
    slml_render::fonts::FONT_FILES
        .get(index)
        .map_or(std::ptr::null(), |f| f.4.as_ptr())
}
#[unsafe(no_mangle)]
pub extern "C" fn slml_font_len(index: usize) -> usize {
    slml_render::fonts::FONT_FILES
        .get(index)
        .map_or(0, |f| f.4.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_render_and_errors_clear_previous_pixels() {
        let mut e = Engine::new();
        e.files.sources.insert(
            "/site/index.slml".into(),
            "slml 0.1 use \"./parts.slml\" page(fill: #123456) { greeting }".into(),
        );
        assert!(!e.open("/site/index.slml"));
        assert_eq!(e.files.requested, ["/site/parts.slml"]);
        e.files.sources.insert(
            "/site/parts.slml".into(),
            "slml 0.1 define greeting { text \"Hello\" }".into(),
        );
        assert!(e.open("/site/index.slml"));
        assert!(e.render(320, 240, 1.0, 0.0));
        assert_eq!(&e.pixels[..4], &[0x12, 0x34, 0x56, 255]);
        assert_eq!(e.pixels.len(), 320 * 240 * 4);
        e.files.sources.insert(
            "/site/parts.slml".into(),
            "slml 0.1 define greeting { colum }".into(),
        );
        assert!(!e.open("/site/index.slml"));
        assert!(e.pixels.is_empty());
        assert!(e.info.contains("/site/parts.slml:1"));
        assert!(e.info.contains("did you mean `column`?"));
    }

    #[test]
    fn metadata_keeps_links_anchors_and_image_requests() {
        let mut e = Engine::new();
        e.files.sources.insert("/index.slml".into(), r##"slml 0.1 meta(title: "A & B") page { text { link(to: "#end") "Jump" } image(src: "a.png", alt: "A") heading(id: "end") "End" }"##.into());
        assert!(e.open("/index.slml"));
        assert!(e.render(320, 240, 2.0, 0.0));
        assert!(e.info.contains("\"images\":[\"a.png\"]"));
        assert!(e.info.contains("\"to\":\"#end\""));
        assert!(e.info.contains("\"anchors\":{\"end\":"));
        assert!(!e.render(0, 240, 1.0, 0.0));
        assert!(e.pixels.is_empty());
    }
}
