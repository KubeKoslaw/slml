//! The native SLML browser. It shows documents in a window and behaves like
//! a web browser: links react to hover and clicks, the status bubble shows
//! where a link goes, there is a back/forward history, reload, zoom,
//! keyboard scrolling and focus, and pages reload when their files change.

mod cursor;
mod ui;

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use minifb::{CursorStyle, Key, KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};
use slml_render::images::ImageStore;
use slml_render::layout::{Item, Rect};
use slml_render::{Layout, Pixmap, Renderer};
use slml_syntax::model::{Color, Document};

use crate::project::{base_dir, compile_file, open_external, resolve};
use ui::Ui;

/// Scroll distance of an arrow key or a wheel notch, in window pixels.
const LINE_STEP: f32 = 48.0;
const ZOOM_STEPS: [f32; 13] = [
    0.5, 0.67, 0.75, 0.8, 0.9, 1.0, 1.1, 1.25, 1.5, 1.75, 2.0, 2.5, 3.0,
];
/// Width of the scrollbar's hit area, in window pixels.
const SCROLLBAR: f32 = 14.0;
/// How long a status message stays visible.
const TOAST: Duration = Duration::from_millis(1800);
const STATUS_SIZE: f32 = 12.5;

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

/// The next zoom level up (`dir` > 0) or down from `zoom`.
fn zoom_step(zoom: f32, dir: i32) -> f32 {
    let i = ZOOM_STEPS
        .iter()
        .position(|z| (z - zoom).abs() < 0.001)
        .unwrap_or(5);
    let next = (i as i32 + dir).clamp(0, ZOOM_STEPS.len() as i32 - 1);
    ZOOM_STEPS[next as usize]
}

/// One link as the user sees it. A text link that wraps has several areas.
#[derive(Debug, Clone, PartialEq)]
struct LinkGroup {
    target: String,
    rects: Vec<Rect>,
    radius: f32,
}

fn group_links(layout: &Layout) -> Vec<LinkGroup> {
    let mut out: Vec<(usize, LinkGroup)> = Vec::new();
    for area in &layout.links {
        if let Some((_, group)) = out.iter_mut().find(|(id, _)| *id == area.id) {
            // Words of one line become one rectangle.
            let r = area.rect;
            match group.rects.last_mut() {
                Some(last)
                    if last.y == r.y && last.h == r.h && (last.x + last.w - r.x).abs() < 1.0 =>
                {
                    last.w = r.x + r.w - last.x;
                }
                _ => group.rects.push(r),
            }
            continue;
        }
        // A boxed link (button, linked column) takes its box's rounding;
        // text links get a small one.
        let radius = layout
            .elements
            .iter()
            .find(|e| e.rect == area.rect)
            .map_or(3.0, |e| e.radius);
        out.push((
            area.id,
            LinkGroup {
                target: area.target.clone(),
                rects: vec![area.rect],
                radius,
            },
        ));
    }
    out.into_iter().map(|(_, group)| group).collect()
}

struct Visit {
    path: PathBuf,
    scroll: f32,
}

/// Something that loads a page. It runs one frame after it is requested, so
/// the window can show that it is loading.
enum Nav {
    To(PathBuf, Option<String>),
    Back,
    Forward,
    Reload { changed: bool },
}

#[derive(Debug, Clone, PartialEq)]
enum Focus {
    Link(usize),
}

enum Drag {
    /// Dragging the scrollbar's thumb, grabbed `grab` pixels below its top.
    Thumb { grab: f32 },
}

/// Mouse buttons and keys as the window reported them for one frame.
#[derive(Default)]
struct Input {
    /// The pointer in window coordinates (window pixels / scale).
    mouse: Option<(f32, f32)>,
    left: bool,
    back: bool,
    forward: bool,
    /// Distance to scroll, in window pixels, positive is down.
    wheel: f32,
    /// Keys pressed this frame, with key repeat.
    keys: Vec<Key>,
    /// Keys pressed this frame, without key repeat.
    fresh: Vec<Key>,
    ctrl: bool,
    shift: bool,
    alt: bool,
}

impl Input {
    fn read(window: &Window, mouse: Option<(f32, f32)>) -> Self {
        let down = |a: Key, b: Key| window.is_key_down(a) || window.is_key_down(b);
        let wheel = window.get_scroll_wheel().map_or(0.0, |(_, dy)| {
            // X11 reports notches (±1, up is positive); Wayland reports
            // pixels (down is positive).
            if dy.abs() > 3.0 {
                dy * 2.0
            } else {
                -dy * LINE_STEP
            }
        });
        Input {
            mouse,
            left: window.get_mouse_down(MouseButton::Left),
            back: window.get_mouse_down(MouseButton::Back),
            forward: window.get_mouse_down(MouseButton::Forward),
            wheel,
            keys: window.get_keys_pressed(KeyRepeat::Yes),
            fresh: window.get_keys_pressed(KeyRepeat::No),
            ctrl: down(Key::LeftCtrl, Key::RightCtrl),
            shift: down(Key::LeftShift, Key::RightShift),
            alt: down(Key::LeftAlt, Key::RightAlt),
        }
    }
}

/// The scrollbar's geometry in window coordinates.
struct Scrollbar {
    track: Rect,
    thumb: Rect,
}

/// Everything that changes what the window shows. A frame is painted only
/// when this differs from the last painted one.
#[derive(Debug, Clone, PartialEq)]
struct Scene {
    generation: u64,
    scroll: f32,
    hover: Option<usize>,
    pressed: Option<usize>,
    focus: Option<Focus>,
    status: Option<String>,
    scrollbar: (bool, bool),
}

struct Browser {
    root: PathBuf,
    scale: f32,
    zoom: f32,
    page: Page,
    renderer: Renderer,
    back: Vec<Visit>,
    forward: Vec<Visit>,
    nav: Option<Nav>,
    layout: Option<Layout>,
    /// Bumped on every new layout.
    generation: u64,
    /// The page size the layout was made for.
    layout_size: (f32, f32),
    links: Vec<LinkGroup>,
    /// Page viewport size in window coordinates.
    view: (f32, f32),
    scroll: f32,
    pending_anchor: Option<String>,
    pending_scroll: Option<f32>,
    hover: Option<usize>,
    pressed: Option<usize>,
    focus: Option<Focus>,
    drag: Option<Drag>,
    on_scrollbar: bool,
    was: (bool, bool, bool),
    toast: Option<(String, Instant)>,
    close: bool,
}

impl Browser {
    fn new(path: PathBuf, root: PathBuf, scale: f32) -> Self {
        let page = open(&path, &root);
        let mut renderer = Renderer::new(base_dir(&path));
        renderer.images = ImageStore::with_root(base_dir(&path), &root);
        Browser {
            root,
            scale,
            zoom: 1.0,
            page,
            renderer,
            back: Vec::new(),
            forward: Vec::new(),
            nav: None,
            layout: None,
            generation: 0,
            layout_size: (0.0, 0.0),
            links: Vec::new(),
            view: (1.0, 1.0),
            scroll: 0.0,
            pending_anchor: None,
            pending_scroll: None,
            hover: None,
            pressed: None,
            focus: None,
            drag: None,
            on_scrollbar: false,
            was: (false, false, false),
            toast: None,
            close: false,
        }
    }

    fn title(&self) -> String {
        match &self.nav {
            Some(_) => format!("Loading… — {}", title_of(&self.page)),
            None => title_of(&self.page),
        }
    }

    fn say(&mut self, message: impl Into<String>) {
        self.toast = Some((message.into(), Instant::now()));
    }

    // ---- loading -------------------------------------------------------------

    fn request(&mut self, nav: Nav) {
        let ok = match &nav {
            Nav::Back => !self.back.is_empty(),
            Nav::Forward => !self.forward.is_empty(),
            _ => true,
        };
        if ok {
            self.nav = Some(nav);
        } else {
            self.say(match nav {
                Nav::Back => "No previous page",
                _ => "No next page",
            });
        }
    }

    fn visit(&self) -> Visit {
        Visit {
            path: self.page.path.clone(),
            scroll: self.scroll,
        }
    }

    fn go(&mut self, nav: Nav) {
        match nav {
            Nav::To(path, anchor) => {
                self.back.push(self.visit());
                self.forward.clear();
                self.load(&path);
                self.pending_anchor = anchor;
            }
            Nav::Back | Nav::Forward => {
                let back = matches!(nav, Nav::Back);
                let (from, to) = if back {
                    (&mut self.back, &mut self.forward)
                } else {
                    (&mut self.forward, &mut self.back)
                };
                let Some(visit) = from.pop() else { return };
                to.push(Visit {
                    path: self.page.path.clone(),
                    scroll: self.scroll,
                });
                self.load(&visit.path);
                self.pending_scroll = Some(visit.scroll);
            }
            Nav::Reload { changed } => {
                let (path, scroll) = (self.page.path.clone(), self.scroll);
                self.load(&path);
                self.pending_scroll = Some(scroll);
                self.say(if changed {
                    "Reloaded: the file changed"
                } else {
                    "Reloaded"
                });
            }
        }
    }

    fn load(&mut self, path: &Path) {
        self.page = open(path, &self.root);
        self.renderer.images = ImageStore::with_root(base_dir(path), &self.root);
        self.layout = None;
        self.scroll = 0.0;
        self.hover = None;
        self.pressed = None;
        self.focus = None;
        self.drag = None;
    }

    /// Live reload: re-read the page when a file of the site changes.
    fn check_files(&mut self) {
        if self.nav.is_none() && crate::serve::version(&self.root) != self.page.revision {
            self.nav = Some(Nav::Reload { changed: true });
        }
    }

    // ---- layout --------------------------------------------------------------

    fn ensure_layout(&mut self) {
        let (w, h) = (self.view.0 / self.zoom, self.view.1 / self.zoom);
        if self.layout.is_some() && self.layout_size == (w, h) {
            return;
        }
        let layout = self.renderer.layout(&self.page.doc, w, h);
        self.links = group_links(&layout);
        self.generation += 1;
        self.layout_size = (w, h);
        if let Some(id) = self.pending_anchor.take() {
            self.scroll = layout.anchor(&id).unwrap_or(0.0);
        }
        if let Some(scroll) = self.pending_scroll.take() {
            self.scroll = scroll;
        }
        self.layout = Some(layout);
        self.clamp_scroll();
    }

    /// The page's visible height in page pixels.
    fn page_view_h(&self) -> f32 {
        self.view.1 / self.zoom
    }

    fn max_scroll(&self) -> f32 {
        self.layout
            .as_ref()
            .map_or(0.0, |l| (l.height - self.page_view_h()).max(0.0))
    }

    fn clamp_scroll(&mut self) {
        self.scroll = self.scroll.clamp(0.0, self.max_scroll());
    }

    /// Scrolls just enough to show `rect` (page coordinates).
    fn reveal(&mut self, rect: Rect) {
        let margin = 24.0;
        let view_h = self.page_view_h();
        if rect.y < self.scroll + margin {
            self.scroll = rect.y - margin;
        } else if rect.y + rect.h > self.scroll + view_h - margin {
            self.scroll = (rect.y + rect.h + margin - view_h).min(rect.y - margin);
        }
        self.clamp_scroll();
    }

    /// A point in window coordinates as a point on the page.
    fn on_page(&self, (x, y): (f32, f32)) -> Option<(f32, f32)> {
        let inside = x >= 0.0 && y >= 0.0 && x < self.view.0 && y < self.view.1;
        inside.then(|| (x / self.zoom, y / self.zoom + self.scroll))
    }

    fn link_at(&self, (x, y): (f32, f32)) -> Option<usize> {
        self.links
            .iter()
            .rposition(|g| g.rects.iter().any(|r| r.contains(x, y)))
    }

    fn scrollbar(&self) -> Option<Scrollbar> {
        let layout = self.layout.as_ref()?;
        let (view_w, view_h) = self.view;
        let content = layout.height * self.zoom;
        if content <= view_h + 0.5 {
            return None;
        }
        let thumb_h = (view_h * view_h / content).clamp(28.0_f32.min(view_h), view_h);
        let travel = view_h - thumb_h;
        let max = self.max_scroll();
        let y = if max > 0.0 {
            travel * self.scroll / max
        } else {
            0.0
        };
        Some(Scrollbar {
            track: Rect::new(view_w - SCROLLBAR, 0.0, SCROLLBAR, view_h),
            thumb: Rect::new(view_w - SCROLLBAR, y, SCROLLBAR, thumb_h),
        })
    }

    // ---- input ---------------------------------------------------------------

    fn frame(&mut self, input: &Input, window: (f32, f32)) {
        if let Some(nav) = self.nav.take() {
            self.go(nav);
        }
        self.view = (window.0.max(1.0), window.1.max(1.0));
        self.ensure_layout();
        self.keys(input);
        self.mouse(input);
        self.clamp_scroll();
    }

    fn keys(&mut self, input: &Input) {
        let (ctrl, shift, alt) = (input.ctrl, input.shift, input.alt);
        for key in &input.fresh {
            match key {
                Key::F5 => self.request(Nav::Reload { changed: false }),
                Key::R if ctrl => self.request(Nav::Reload { changed: false }),
                Key::Left if alt => self.request(Nav::Back),
                Key::Right if alt => self.request(Nav::Forward),
                Key::Backspace if !ctrl && !alt => {
                    self.request(if shift { Nav::Forward } else { Nav::Back })
                }
                Key::Enter | Key::NumPadEnter => self.activate(),
                Key::Escape => {
                    self.focus = None;
                    self.pressed = None;
                }
                Key::W | Key::Q if ctrl => self.close = true,
                _ => {}
            }
        }
        let page = self.page_view_h() * 0.9;
        let line = LINE_STEP / self.zoom;
        for key in &input.keys {
            match key {
                Key::Down => self.scroll += line,
                Key::Up => self.scroll -= line,
                Key::PageDown => self.scroll += page,
                Key::PageUp => self.scroll -= page,
                Key::Space if shift => self.scroll -= page,
                Key::Space => self.scroll += page,
                Key::Home => self.scroll = 0.0,
                Key::End => self.scroll = self.max_scroll(),
                Key::Tab if !ctrl && !alt => self.move_focus(if shift { -1 } else { 1 }),
                Key::Equal | Key::NumPadPlus if ctrl => self.set_zoom(zoom_step(self.zoom, 1)),
                Key::Minus | Key::NumPadMinus if ctrl => self.set_zoom(zoom_step(self.zoom, -1)),
                Key::Key0 | Key::NumPad0 if ctrl => self.set_zoom(1.0),
                _ => {}
            }
        }
    }

    fn set_zoom(&mut self, zoom: f32) {
        if zoom == self.zoom {
            return;
        }
        // Keep the top of the view on the same part of the page.
        let fraction = self
            .layout
            .as_ref()
            .map_or(0.0, |l| self.scroll / l.height.max(1.0));
        self.zoom = zoom;
        self.layout = None;
        self.ensure_layout();
        if let Some(l) = &self.layout {
            self.scroll = fraction * l.height;
        }
        self.clamp_scroll();
        self.say(format!("Zoom {:.0}%", zoom * 100.0));
    }

    /// Links and fields in reading order, for Tab.
    fn focusables(&self) -> Vec<(Focus, Rect)> {
        let mut out: Vec<(Focus, Rect)> = self
            .links
            .iter()
            .enumerate()
            .map(|(i, g)| (Focus::Link(i), g.rects[0]))
            .collect();
        out.sort_by(|(_, a), (_, b)| (a.y, a.x).partial_cmp(&(b.y, b.x)).unwrap());
        out
    }

    fn move_focus(&mut self, dir: i32) {
        let all = self.focusables();
        if all.is_empty() {
            return;
        }
        let current = self
            .focus
            .as_ref()
            .and_then(|f| all.iter().position(|(g, _)| g == f));
        let next = match current {
            Some(i) => (i as i32 + dir).rem_euclid(all.len() as i32) as usize,
            None if dir > 0 => 0,
            None => all.len() - 1,
        };
        let (focus, rect) = all[next].clone();
        self.focus = Some(focus);
        self.reveal(rect);
    }

    fn activate(&mut self) {
        if let Some(Focus::Link(i)) = self.focus
            && let Some(group) = self.links.get(i)
        {
            let target = group.target.clone();
            self.follow(&target);
        }
    }

    fn follow(&mut self, to: &str) {
        match classify(to, &self.page.path, &self.root) {
            Target::External(url) => {
                open_external(&url);
                self.say(format!("Opened {url} in your web browser"));
            }
            Target::Anchor(id) => match self.layout.as_ref().and_then(|l| l.anchor(&id)) {
                Some(y) => {
                    self.scroll = y;
                    self.clamp_scroll();
                }
                None => self.say(format!("No element has id `{id}`")),
            },
            Target::Document(path, anchor) => self.request(Nav::To(path, anchor)),
        }
    }

    fn mouse(&mut self, input: &Input) {
        let pressed = input.left && !self.was.0;
        let released = !input.left && self.was.0;
        if input.back && !self.was.1 {
            self.request(Nav::Back);
        }
        if input.forward && !self.was.2 {
            self.request(Nav::Forward);
        }
        self.was = (input.left, input.back, input.forward);

        if input.wheel != 0.0 {
            if input.ctrl {
                self.set_zoom(zoom_step(self.zoom, if input.wheel < 0.0 { 1 } else { -1 }));
            } else {
                self.scroll += input.wheel / self.zoom;
            }
        }

        let bar = self.scrollbar();
        let on_bar = |m: (f32, f32)| bar.as_ref().is_some_and(|b| b.track.contains(m.0, m.1));
        self.on_scrollbar = input.mouse.is_some_and(on_bar) || self.drag.is_some();
        let point = input
            .mouse
            .filter(|m| !on_bar(*m))
            .and_then(|m| self.on_page(m));
        self.hover = match self.drag {
            None => point.and_then(|p| self.link_at(p)),
            Some(_) => None,
        };

        if pressed && let Some(m) = input.mouse {
            if let Some(bar) = bar.as_ref().filter(|_| on_bar(m)) {
                if bar.thumb.contains(m.0, m.1) {
                    self.drag = Some(Drag::Thumb {
                        grab: m.1 - bar.thumb.y,
                    });
                } else {
                    // Clicking the track moves one page towards the click.
                    let page = self.page_view_h() * 0.9;
                    self.scroll += if m.1 < bar.thumb.y { -page } else { page };
                }
            } else {
                // A mouse click hides the keyboard focus ring.
                self.focus = None;
                self.pressed = self.hover;
            }
        }
        if let (Some(Drag::Thumb { grab }), Some(m), Some(bar)) = (&self.drag, input.mouse, &bar) {
            let travel = (bar.track.h - bar.thumb.h).max(1.0);
            self.scroll = ((m.1 - grab) / travel).clamp(0.0, 1.0) * self.max_scroll();
        }
        if released {
            if let Some(i) = self.pressed.take()
                && self.hover == Some(i)
            {
                let target = self.links[i].target.clone();
                self.follow(&target);
            }
            self.drag = None;
        }
    }

    /// The cursor for a pointer position (window coordinates).
    fn cursor(&self, mouse: Option<(f32, f32)>) -> CursorStyle {
        let Some(m) = mouse else {
            return CursorStyle::Arrow;
        };
        if self.drag.is_some() || self.scrollbar().is_some_and(|b| b.track.contains(m.0, m.1)) {
            return CursorStyle::Arrow;
        }
        match self.on_page(m) {
            Some(p) if self.link_at(p).is_some() => CursorStyle::ClosedHand,
            _ => CursorStyle::Arrow,
        }
    }

    // ---- painting ------------------------------------------------------------

    fn status(&self) -> Option<String> {
        if let Some(nav) = &self.nav {
            let name = |p: &Path| {
                p.file_name()
                    .map_or_else(String::new, |f| f.to_string_lossy().into_owned())
            };
            return Some(match nav {
                Nav::To(path, _) => format!("Loading {}…", name(path)),
                Nav::Reload { .. } => "Reloading…".into(),
                Nav::Back | Nav::Forward => "Loading…".into(),
            });
        }
        // A hovered link wins over messages, messages over the focused link.
        let link = |i: Option<usize>| i.and_then(|i| self.links.get(i));
        if let Some(group) = link(self.hover) {
            return Some(self.describe(&group.target));
        }
        if let Some((text, _)) = self.toast.as_ref().filter(|(_, at)| at.elapsed() < TOAST) {
            return Some(text.clone());
        }
        match self.focus {
            Some(Focus::Link(i)) => link(Some(i)).map(|g| self.describe(&g.target)),
            _ => None,
        }
    }

    /// A link target as the status bubble shows it.
    fn describe(&self, to: &str) -> String {
        match classify(to, &self.page.path, &self.root) {
            Target::External(url) => format!("{url}  ↗"),
            Target::Anchor(id) => format!("#{id}"),
            Target::Document(path, anchor) => {
                let full = crate::build::normalize(&path);
                let root = crate::build::normalize(&self.root);
                let shown = full.strip_prefix(&root).unwrap_or(&full).display();
                let anchor = anchor.map(|a| format!("#{a}")).unwrap_or_default();
                let missing = if path.is_file() { "" } else { "  (missing)" };
                format!("{shown}{anchor}{missing}")
            }
        }
    }

    fn scene(&self) -> Scene {
        Scene {
            generation: self.generation,
            scroll: self.scroll,
            hover: self.hover,
            pressed: self.pressed,
            focus: self.focus.clone(),
            status: self.status(),
            scrollbar: (self.on_scrollbar, self.drag.is_some()),
        }
    }

    /// Highlights drawn over the page, in page coordinates.
    fn page_overlay(&self) -> Vec<Item> {
        let mut ui = Ui::new(self.renderer.fonts());
        let link = |i: Option<usize>| i.and_then(|i| self.links.get(i));
        if let Some(group) = link(self.hover) {
            let shade = if self.pressed == self.hover {
                Color(0x0000_002e)
            } else {
                Color(0x0000_0014)
            };
            for r in &group.rects {
                ui.fill(*r, shade, group.radius);
            }
        }
        if let Some(Focus::Link(i)) = self.focus
            && let Some(group) = link(Some(i))
        {
            for r in &group.rects {
                ui.border(ui::outset(*r, 2.0), 2.0, ui::ACCENT, group.radius + 2.0);
            }
        }
        ui.items
    }

    /// The browser's interface, in window coordinates.
    fn chrome(&self) -> Vec<Item> {
        let mut ui = Ui::new(self.renderer.fonts());
        if let Some(bar) = self.scrollbar() {
            let active = self.on_scrollbar;
            if active {
                ui.fill(bar.track, Color(0x0000_000c), 0.0);
            }
            let w = if active { 9.0 } else { 6.0 };
            let thumb = Rect::new(
                bar.thumb.x + bar.thumb.w - w - 3.0,
                bar.thumb.y + 2.0,
                w,
                bar.thumb.h - 4.0,
            );
            let color = if self.drag.is_some() {
                Color(0x0000_0099)
            } else if active {
                Color(0x0000_0077)
            } else {
                Color(0x0000_0055)
            };
            ui.fill(thumb, color, w / 2.0);
        }
        if let Some(text) = self.status() {
            let max_w = (self.view.0 * 0.7).clamp(120.0, 720.0);
            ui.bubble(6.0, self.view.1 - 6.0, max_w, &text, STATUS_SIZE);
        }
        ui.items
    }

    fn paint(&mut self, target: &mut Pixmap) {
        let overlay = self.page_overlay();
        let chrome = self.chrome();
        let Some(layout) = &self.layout else { return };
        let page_scale = self.scale * self.zoom;
        self.renderer.paint(layout, target, self.scroll, page_scale);
        self.renderer
            .paint_items(&overlay, target, self.scroll, page_scale);
        self.renderer.paint_items(&chrome, target, 0.0, self.scale);
    }
}

pub fn run(path: PathBuf, root: PathBuf, scale: f32) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("error: {} does not exist", path.display()));
    }
    let hotspots = cursor::prepare();
    let mut browser = Browser::new(path, root, scale);
    let mut title = browser.title();
    let mut window = Window::new(
        &title,
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
    let mut pixmap: Option<Pixmap> = None;
    let mut buffer: Vec<u32> = Vec::new();
    let mut painted: Option<Scene> = None;
    let mut last_poll = Instant::now();
    let mut cursor_style = CursorStyle::Arrow;

    while window.is_open() && !browser.close {
        let (w, h) = window.get_size();
        let (w, h) = (w.max(1), h.max(1));
        if (w, h) != size {
            size = (w, h);
            pixmap = Pixmap::new(w as u32, h as u32);
            buffer = vec![0; w * h];
            painted = None;
        }

        // While a button is held the pointer may leave the window (dragging
        // the scrollbar or a selection), so positions are not discarded.
        let mode = if browser.was.0 {
            MouseMode::Pass
        } else {
            MouseMode::Discard
        };
        let raw = window.get_mouse_pos(mode);
        // The pointer is where the user sees the cursor's hot point.
        let at = |style: CursorStyle| {
            let (hx, hy) = hotspots.offset(style);
            raw.map(|(x, y)| ((x + hx) / scale, (y + hy) / scale))
        };
        let input = Input::read(&window, at(cursor_style));
        if last_poll.elapsed() > Duration::from_millis(300) {
            last_poll = Instant::now();
            browser.check_files();
        }
        browser.frame(&input, (w as f32 / scale, h as f32 / scale));

        // Switch only when the new cursor's own hot point agrees, so the
        // cursor never flickers between two shapes at an edge.
        let wanted = browser.cursor(at(cursor_style));
        if wanted != cursor_style && browser.cursor(at(wanted)) == wanted {
            cursor_style = wanted;
            window.set_cursor_style(cursor_style);
        }
        if browser.title() != title {
            title = browser.title();
            window.set_title(&title);
        }

        let scene = browser.scene();
        if painted.as_ref() != Some(&scene)
            && let Some(pm) = pixmap.as_mut()
        {
            browser.paint(pm);
            for (dst, px) in buffer.iter_mut().zip(pm.pixels()) {
                *dst = (px.red() as u32) << 16 | (px.green() as u32) << 8 | px.blue() as u32;
            }
            window
                .update_with_buffer(&buffer, w, h)
                .map_err(|e| format!("error: cannot draw: {e}"))?;
            painted = Some(scene);
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

    #[test]
    fn zoom_steps_stop_at_both_ends() {
        assert_eq!(zoom_step(1.0, 1), 1.1);
        assert_eq!(zoom_step(1.0, -1), 0.9);
        assert_eq!(zoom_step(3.0, 1), 3.0);
        assert_eq!(zoom_step(0.5, -1), 0.5);
    }

    #[test]
    fn wrapped_text_links_are_one_link() {
        let doc = slml_syntax::compile(
            r#"slml 0.1 page(padding: 0) {
                text(width: 90) { link(to: "a.slml") "one two three four" " " link(to: "a.slml") "again" }
                button(to: "b.slml", radius: 9) "Go"
            }"#,
        )
        .unwrap();
        let layout = Renderer::new(".").layout(&doc, 400.0, 0.0);
        let groups = group_links(&layout);
        assert_eq!(groups.len(), 3, "{groups:#?}");
        assert!(
            groups[0].rects.len() > 1,
            "the wrapped link keeps its lines"
        );
        let lines = groups[0].rects.iter().map(|r| r.y);
        assert!(
            lines.clone().zip(lines.skip(1)).all(|(a, b)| b > a),
            "one rectangle per line: {:?}",
            groups[0].rects
        );
        assert_eq!(groups[0].target, "a.slml");
        assert_eq!(
            (groups[2].target.as_str(), groups[2].radius),
            ("b.slml", 9.0)
        );
    }

    #[test]
    fn tab_moves_focus_in_reading_order_and_reveals_it() {
        let root = std::env::temp_dir().join(format!("slml-native-tab-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("index.slml");
        std::fs::write(
            &file,
            r#"slml 0.1 page(gap: 900) { button(to: "a.slml") "A" button(to: "b.slml") "B" }"#,
        )
        .unwrap();
        let mut b = Browser::new(file, root.clone(), 1.0);
        let tab = |b: &mut Browser, shift| {
            let input = Input {
                keys: vec![Key::Tab],
                shift,
                ..Input::default()
            };
            b.frame(&input, (800.0, 600.0));
        };
        tab(&mut b, false);
        assert_eq!(b.focus, Some(Focus::Link(0)));
        assert_eq!(b.scroll, 0.0);
        tab(&mut b, false);
        assert_eq!(b.focus, Some(Focus::Link(1)));
        assert!(b.scroll > 300.0, "the second button is scrolled into view");
        assert_eq!(b.status().as_deref(), Some("b.slml  (missing)"));
        tab(&mut b, true);
        assert_eq!(b.focus, Some(Focus::Link(0)));
        std::fs::remove_dir_all(root).unwrap();
    }
}
