//! A minimal browser window for SLML documents: scrolling, links, history
//! and live reload on save.

mod cursor;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use minifb::{CursorStyle, Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use slml_render::images::ImageStore;
use slml_render::{Layout, Pixmap, Renderer};
use slml_syntax::model::Document;

use crate::project::{base_dir, compile_file, open_external, resolve};

const LINE_STEP: f32 = 48.0;

struct Page {
    path: PathBuf,
    doc: Document,
    revision: String,
}

/// What clicking a link does.
enum Target {
    /// `https:`, `mailto:` …: handed to the system's default program.
    External(String),
    /// Another document, optionally scrolled to an anchor.
    Document(PathBuf, Option<String>),
    /// `#id` on the current page.
    Anchor(String),
}

fn classify(to: &str, page: &Path, root: &Path) -> Target {
    if slml_syntax::analyze::is_external(to) {
        return Target::External(to.to_string());
    }
    match to.split_once('#') {
        Some(("", id)) => Target::Anchor(id.to_string()),
        Some((path, id)) => Target::Document(resolve(root, page, path), Some(id.to_string())),
        None => Target::Document(resolve(root, page, to), None),
    }
}

fn open(path: &Path, root: &Path) -> Page {
    let revision = crate::serve::version(root);
    let doc = compile_file(path, root).unwrap_or_else(|errors| {
        eprintln!("{errors}");
        error_page(path, &errors)
    });
    Page {
        path: path.to_path_buf(),
        doc,
        revision,
    }
}

/// Errors are shown as an SLML page, written in SLML.
fn error_page(path: &Path, errors: &str) -> Document {
    let src = format!(
        r#"slml 0.1
meta(title: "Error")
page(fill: #fff8f7, gap: 14) {{
  heading(level: 2, color: #b42318) "This document has errors"
  text(color: #55555f) {}
  text(font: mono, size: 13, fill: #ffffff, padding: 16, radius: 8, border: 1, border-color: #f1c4c0) {}
  text(color: #55555f) "Fix the file and save it; this window reloads automatically."
}}"#,
        quote(&path.display().to_string()),
        quote(errors),
    );
    slml_syntax::compile(&src).expect("the built-in error page is valid SLML")
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("    "),
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn title_of(page: &Page) -> String {
    let file = page
        .path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();
    match &page.doc.meta.title {
        Some(t) => format!("{t} — SLML"),
        None => format!("{file} — SLML"),
    }
}

pub fn run(path: PathBuf, root: PathBuf, scale: f32) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("error: {} does not exist", path.display()));
    }
    let mut page = open(&path, &root);
    let mut renderer = Renderer::new(base_dir(&path));
    renderer.images = ImageStore::with_root(base_dir(&path), &root);
    let mut pending_anchor: Option<String> = None;
    let mut history: Vec<PathBuf> = Vec::new();

    let hotspots = cursor::prepare();
    let mut window = Window::new(
        &title_of(&page),
        1000,
        760,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )
    .map_err(|e| format!("error: cannot open a window: {e}"))?;
    window.set_target_fps(60);

    let mut size = (0usize, 0usize);
    let mut layout: Option<Layout> = None;
    let mut pixmap: Option<Pixmap> = None;
    let mut buffer: Vec<u32> = Vec::new();
    let mut scroll = 0.0f32;
    let mut dirty = true;
    let mut last_poll = Instant::now();
    let mut mouse_was_down = false;
    let mut cursor_style = CursorStyle::Arrow;

    while window.is_open() {
        let (w, h) = window.get_size();
        let (w, h) = (w.max(1), h.max(1));
        let (view_w, view_h) = (w as f32 / scale, h as f32 / scale);

        if (w, h) != size {
            size = (w, h);
            layout = None;
            pixmap = Pixmap::new(w as u32, h as u32);
            buffer = vec![0; w * h];
        }

        // Live reload: re-read the file when it changes on disk.
        let mut reload = window.is_key_pressed(Key::F5, KeyRepeat::No);
        if last_poll.elapsed() > Duration::from_millis(300) {
            last_poll = Instant::now();
            reload |= crate::serve::version(&root) != page.revision;
        }
        if reload {
            page = open(&page.path, &root);
            renderer.images.clear();
            window.set_title(&title_of(&page));
            layout = None;
        }

        let lay = layout.get_or_insert_with(|| {
            dirty = true;
            renderer.layout(&page.doc, view_w, view_h)
        });
        let max_scroll = (lay.height - view_h).max(0.0);
        if let Some(id) = pending_anchor.take() {
            scroll = lay.anchor(&id).unwrap_or(0.0);
        }

        // Scrolling.
        let before = scroll;
        if let Some((_, dy)) = window.get_scroll_wheel() {
            // X11 reports notches (±1, up is positive); Wayland reports
            // pixels (down is positive).
            scroll += if dy.abs() > 3.0 {
                dy * 2.0
            } else {
                -dy * LINE_STEP
            };
        }
        for key in window.get_keys_pressed(KeyRepeat::Yes) {
            match key {
                Key::Down => scroll += LINE_STEP,
                Key::Up => scroll -= LINE_STEP,
                Key::PageDown | Key::Space => scroll += view_h * 0.9,
                Key::PageUp => scroll -= view_h * 0.9,
                Key::Home => scroll = 0.0,
                Key::End => scroll = max_scroll,
                _ => {}
            }
        }
        scroll = scroll.clamp(0.0, max_scroll);
        dirty |= scroll != before;

        // Links. The pointer is where the user sees the cursor's hot point.
        let raw = window.get_mouse_pos(MouseMode::Discard);
        let at = |style: CursorStyle| {
            let (hx, hy) = hotspots.offset(style);
            raw.map(|(mx, my)| ((mx + hx) / scale, (my + hy) / scale + scroll))
        };
        let style_at = |p: Option<(f32, f32)>| match p {
            Some((x, y)) if lay.link_at(x, y).is_some() => CursorStyle::ClosedHand,
            _ => CursorStyle::Arrow,
        };
        // Switch only when the new cursor's own hot point agrees, so the
        // cursor never flickers between two shapes at an edge.
        let wanted = style_at(at(cursor_style));
        if wanted != cursor_style && style_at(at(wanted)) == wanted {
            cursor_style = wanted;
            window.set_cursor_style(cursor_style);
        }
        let target = at(cursor_style).and_then(|(x, y)| lay.link_at(x, y).map(str::to_string));
        let down = window.get_mouse_down(MouseButton::Left);
        let clicked = down && !mouse_was_down;
        mouse_was_down = down;

        let mut go_to: Option<(PathBuf, Option<String>)> = None;
        if clicked && let Some(t) = &target {
            match classify(t, &page.path, &root) {
                Target::External(url) => open_external(&url),
                Target::Anchor(id) => {
                    scroll = lay.anchor(&id).unwrap_or(scroll).clamp(0.0, max_scroll);
                    dirty = true;
                }
                Target::Document(path, anchor) => go_to = Some((path, anchor)),
            }
        }
        if window.is_key_pressed(Key::Backspace, KeyRepeat::No)
            && let Some(prev) = history.pop()
        {
            page = open(&prev, &root);
            go_to = None;
            layout = None;
            scroll = 0.0;
            renderer.images = ImageStore::with_root(base_dir(&page.path), &root);
            window.set_title(&title_of(&page));
        }
        if let Some((next, anchor)) = go_to {
            history.push(page.path.clone());
            page = open(&next, &root);
            layout = None;
            scroll = 0.0;
            pending_anchor = anchor;
            renderer.images = ImageStore::with_root(base_dir(&page.path), &root);
            window.set_title(&title_of(&page));
        }

        if layout.is_none() {
            // Navigated or reloaded this frame; lay out on the next turn.
            window.update();
            continue;
        }

        if dirty && let (Some(lay), Some(pm)) = (&layout, pixmap.as_mut()) {
            renderer.paint(lay, pm, scroll, scale);
            for (dst, px) in buffer.iter_mut().zip(pm.pixels()) {
                *dst = (px.red() as u32) << 16 | (px.green() as u32) << 8 | px.blue() as u32;
            }
            window
                .update_with_buffer(&buffer, w, h)
                .map_err(|e| format!("error: cannot draw: {e}"))?;
            dirty = false;
        } else {
            window.update();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_reload_tracks_shared_components_and_images() {
        let root = std::env::temp_dir().join(format!("slml-native-reload-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("index.slml");
        let shared = root.join("parts.slml");
        std::fs::write(&file, "slml 0.1 use \"parts.slml\" page { shared }").unwrap();
        std::fs::write(&shared, "slml 0.1 define shared { text \"First\" }").unwrap();
        let first = open(&file, &root);
        std::fs::write(
            &shared,
            "slml 0.1 define shared { text \"Updated shared component\" }",
        )
        .unwrap();
        assert_ne!(crate::serve::version(&root), first.revision);
        let second = open(&file, &root);
        assert!(format!("{:?}", second.doc).contains("Updated shared component"));
        std::fs::write(root.join("image.png"), b"changed asset").unwrap();
        assert_ne!(crate::serve::version(&root), second.revision);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn error_page_survives_any_message() {
        let doc = error_page(Path::new("a\"b.slml"), "line \"1\"\n\tback\\slash \u{7}");
        assert_eq!(doc.meta.title.as_deref(), Some("Error"));
    }
}
