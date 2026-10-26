//! A minimal browser window for SLML documents: scrolling, links, history
//! and live reload on save.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use minifb::{CursorStyle, Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use slml_render::images::ImageStore;
use slml_render::{Layout, Pixmap, Renderer};
use slml_syntax::model::Document;

use crate::{base_dir, load};

const LINE_STEP: f32 = 48.0;

struct Page {
    path: PathBuf,
    doc: Document,
    modified: Option<SystemTime>,
}

fn open(path: &Path) -> Page {
    let modified = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let doc = load(path).unwrap_or_else(|errors| {
        eprintln!("{errors}");
        error_page(path, &errors)
    });
    Page {
        path: path.to_path_buf(),
        doc,
        modified,
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

pub fn run(path: PathBuf, scale: f32) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("error: {} does not exist", path.display()));
    }
    let mut page = open(&path);
    let mut renderer = Renderer::new(base_dir(&path));
    let mut history: Vec<PathBuf> = Vec::new();

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
    let mut over_link = false;

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
            let modified = std::fs::metadata(&page.path)
                .and_then(|m| m.modified())
                .ok();
            reload |= modified != page.modified;
        }
        if reload {
            page = open(&page.path);
            renderer.images.clear();
            window.set_title(&title_of(&page));
            layout = None;
        }

        let lay = layout.get_or_insert_with(|| {
            dirty = true;
            renderer.layout(&page.doc, view_w, view_h)
        });
        let max_scroll = (lay.height - view_h).max(0.0);

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

        // Links.
        let mouse = window.get_mouse_pos(MouseMode::Discard);
        let target = mouse.and_then(|(mx, my)| {
            lay.link_at(mx / scale, my / scale + scroll)
                .map(str::to_string)
        });
        if target.is_some() != over_link {
            over_link = target.is_some();
            window.set_cursor_style(if over_link {
                CursorStyle::ClosedHand
            } else {
                CursorStyle::Arrow
            });
        }
        let down = window.get_mouse_down(MouseButton::Left);
        let clicked = down && !mouse_was_down;
        mouse_was_down = down;

        let mut go_to: Option<PathBuf> = None;
        if clicked && let Some(t) = &target {
            if t.contains("://") {
                eprintln!("slml: external links are not supported yet: {t}");
            } else {
                go_to = Some(base_dir(&page.path).join(t));
            }
        }
        if window.is_key_pressed(Key::Backspace, KeyRepeat::No)
            && let Some(prev) = history.pop()
        {
            page = open(&prev);
            go_to = None;
            layout = None;
            scroll = 0.0;
            renderer.images = ImageStore::new(base_dir(&page.path));
            window.set_title(&title_of(&page));
        }
        if let Some(next) = go_to {
            history.push(page.path.clone());
            page = open(&next);
            layout = None;
            scroll = 0.0;
            renderer.images = ImageStore::new(base_dir(&page.path));
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
    fn error_page_survives_any_message() {
        let doc = error_page(Path::new("a\"b.slml"), "line \"1\"\n\tback\\slash \u{7}");
        assert_eq!(doc.meta.title.as_deref(), Some("Error"));
    }
}
