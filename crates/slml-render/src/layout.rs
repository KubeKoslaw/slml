//! The layout engine: turns a [`Document`] into positioned paint items.
//!
//! The model is deliberately small (see SPEC.md §5):
//! - `column`/`page` stack children vertically, `row` horizontally.
//! - Sizes are `auto` (content), `fill` (share free space) or pixels.
//! - Text wraps at spaces; nothing ever overlaps unless a child is wider
//!   than its parent allows.
//!
//! Each function returns a [`Fragment`] positioned at (0, 0); parents move
//! fragments into place. There are no global passes and no floats.

use slml_syntax::model::*;

use crate::fonts::{Face, Fonts, LINE_HEIGHT};
use crate::images::ImageStore;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

/// One thing to draw. Coordinates are in logical pixels from the top-left
/// of the page.
#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    Fill {
        rect: Rect,
        color: Color,
        radius: f32,
    },
    Border {
        rect: Rect,
        width: f32,
        color: Color,
        radius: f32,
    },
    Glyphs {
        face: Face,
        size: f32,
        color: Color,
        x: f32,
        baseline: f32,
        /// Glyph index and x offset from `x`.
        glyphs: Vec<(u16, f32)>,
    },
    Image {
        rect: Rect,
        src: String,
        radius: f32,
    },
}

impl Item {
    fn translate(&mut self, dx: f32, dy: f32) {
        match self {
            Item::Fill { rect, .. } | Item::Border { rect, .. } | Item::Image { rect, .. } => {
                rect.x += dx;
                rect.y += dy;
            }
            Item::Glyphs { x, baseline, .. } => {
                *x += dx;
                *baseline += dy;
            }
        }
    }

    /// Vertical extent, used to skip items outside the viewport.
    pub fn y_range(&self) -> (f32, f32) {
        match self {
            Item::Fill { rect, .. } | Item::Border { rect, .. } | Item::Image { rect, .. } => {
                (rect.y, rect.y + rect.h)
            }
            Item::Glyphs { size, baseline, .. } => (baseline - size * 1.2, baseline + size * 0.5),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinkArea {
    pub rect: Rect,
    pub target: String,
}

/// A laid-out page, ready to paint.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub width: f32,
    pub height: f32,
    pub background: Color,
    pub items: Vec<Item>,
    pub links: Vec<LinkArea>,
}

impl Layout {
    pub fn link_at(&self, x: f32, y: f32) -> Option<&str> {
        self.links
            .iter()
            .rev()
            .find(|l| l.rect.contains(x, y))
            .map(|l| l.target.as_str())
    }
}

#[derive(Debug, Default)]
struct Fragment {
    w: f32,
    h: f32,
    items: Vec<Item>,
    links: Vec<LinkArea>,
}

impl Fragment {
    fn new(w: f32, h: f32) -> Self {
        Self {
            w,
            h,
            ..Self::default()
        }
    }

    fn place(&mut self, child: Fragment, dx: f32, dy: f32) {
        for mut item in child.items {
            item.translate(dx, dy);
            self.items.push(item);
        }
        for mut link in child.links {
            link.rect.x += dx;
            link.rect.y += dy;
            self.links.push(link);
        }
    }
}

/// Fully resolved text properties at some point in the tree.
#[derive(Debug, Clone, Copy)]
struct Computed {
    color: Color,
    size: f32,
    bold: bool,
    italic: bool,
    mono: bool,
    underline: bool,
}

impl Computed {
    fn with(&self, t: &TextStyle) -> Computed {
        Computed {
            color: t.color.unwrap_or(self.color),
            size: t.size.unwrap_or(self.size),
            bold: t.bold.unwrap_or(self.bold),
            italic: t.italic.unwrap_or(self.italic),
            mono: t.mono.unwrap_or(self.mono),
            underline: t.underline.unwrap_or(self.underline),
        }
    }

    fn face(&self) -> Face {
        Face::select(self.bold, self.italic, self.mono)
    }
}

/// How a parent constrains a child's width.
#[derive(Debug, Clone, Copy)]
enum WidthMode {
    /// `auto` width takes all available space.
    Stretch(f32),
    /// `auto` width hugs content, up to the available space.
    Shrink(f32),
    /// The parent (a `row`) already decided.
    Exact(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    Vertical,
    Horizontal,
}

const DEFAULT_BORDER: Color = Color(0xd4d4d8ff);

pub fn layout(
    doc: &Document,
    fonts: &Fonts,
    images: &mut ImageStore,
    width: f32,
    min_height: f32,
) -> Layout {
    let root = Computed {
        color: Color(0x1b1b1fff),
        size: 16.0,
        bold: false,
        italic: false,
        mono: false,
        underline: false,
    };
    let mut engine = Engine { fonts, images };
    let page = &doc.page;
    let mut frag = engine.container(page, root, WidthMode::Stretch(width), None);
    if frag.h < min_height {
        // Let `justify` and spacers work against the window height.
        frag = engine.container(page, root, WidthMode::Stretch(width), Some(min_height));
    }
    Layout {
        width,
        height: frag.h,
        background: page.frame.fill.unwrap_or(Color::WHITE),
        items: frag.items,
        links: frag.links,
    }
}

struct Engine<'a> {
    fonts: &'a Fonts,
    images: &'a mut ImageStore,
}

impl Engine<'_> {
    fn node(
        &mut self,
        n: &Node,
        style: Computed,
        mode: WidthMode,
        forced_h: Option<f32>,
        axis: Axis,
    ) -> Fragment {
        match n {
            Node::Container(c) => self.container(c, style, mode, forced_h),
            Node::Text(t) => self.text_block(t, style, mode, forced_h),
            Node::Image(i) => self.image(i, style, mode),
            Node::Divider(d) => {
                let avail = match mode {
                    WidthMode::Stretch(a) | WidthMode::Shrink(a) | WidthMode::Exact(a) => a,
                };
                let (w, h) = match axis {
                    Axis::Vertical => (
                        match d.width {
                            Size::Px(p) => p.min(avail),
                            _ => avail,
                        },
                        d.thickness,
                    ),
                    Axis::Horizontal => (d.thickness, forced_h.unwrap_or(0.0)),
                };
                let mut f = Fragment::new(w, h);
                f.items.push(Item::Fill {
                    rect: Rect::new(0.0, 0.0, w, h),
                    color: d.color,
                    radius: 0.0,
                });
                f
            }
            Node::Spacer(s) => {
                let w = match (mode, s.width) {
                    (WidthMode::Exact(w), _) => w,
                    (_, Size::Px(p)) => p,
                    _ => 0.0,
                };
                let h = forced_h.unwrap_or(match s.height {
                    Size::Px(p) => p,
                    _ => 0.0,
                });
                Fragment::new(w, h)
            }
        }
    }

    fn resolve_width(
        &mut self,
        width: Size,
        max_width: Option<f32>,
        mode: WidthMode,
        max_content: impl FnOnce(&mut Self) -> f32,
    ) -> f32 {
        let w = match mode {
            WidthMode::Exact(w) => w,
            WidthMode::Stretch(avail) => match width {
                Size::Px(p) => p,
                _ => avail,
            },
            WidthMode::Shrink(avail) => match width {
                Size::Px(p) => p,
                Size::Fill => avail,
                Size::Auto => max_content(self).min(avail),
            },
        };
        max_width.map_or(w, |m| w.min(m)).max(0.0)
    }

    /// Paints a frame's background and border around already laid-out content.
    fn framed(f: &Frame, w: f32, h: f32, content: Fragment) -> Fragment {
        let mut out = Fragment::new(w, h);
        if let Some(fill) = f.fill {
            out.items.push(Item::Fill {
                rect: Rect::new(0.0, 0.0, w, h),
                color: fill,
                radius: f.radius,
            });
        }
        out.place(content, f.padding_x + f.border, f.padding_y + f.border);
        if f.border > 0.0 {
            out.items.push(Item::Border {
                rect: Rect::new(0.0, 0.0, w, h),
                width: f.border,
                color: f.border_color.unwrap_or(DEFAULT_BORDER),
                radius: f.radius,
            });
        }
        out
    }

    fn explicit_height(f: &Frame, forced_h: Option<f32>) -> Option<f32> {
        forced_h.or(match f.height {
            Size::Px(h) => Some(h),
            _ => None,
        })
    }

    // ---- containers --------------------------------------------------------

    fn container(
        &mut self,
        c: &Container,
        parent: Computed,
        mode: WidthMode,
        forced_h: Option<f32>,
    ) -> Fragment {
        let style = parent.with(&c.text);
        let f = &c.frame;
        let w = self.resolve_width(f.width, f.max_width, mode, |e| {
            e.container_max_content(c, style)
        });
        let (ix, iy) = (f.padding_x + f.border, f.padding_y + f.border);
        let inner_w = (w - 2.0 * ix).max(0.0);
        let explicit_h = Self::explicit_height(f, forced_h);
        let inner_h = explicit_h.map(|h| (h - 2.0 * iy).max(0.0));
        let content = match c.kind {
            ContainerKind::Row => self.row(c, style, inner_w, inner_h),
            ContainerKind::Column | ContainerKind::Page => self.column(c, style, inner_w, inner_h),
        };
        let h = explicit_h.unwrap_or(content.h + 2.0 * iy);
        Self::framed(f, w, h, content)
    }

    fn column(
        &mut self,
        c: &Container,
        style: Computed,
        inner_w: f32,
        inner_h: Option<f32>,
    ) -> Fragment {
        let flow = &c.flow;
        let mode = match flow.align {
            Align::Stretch => WidthMode::Stretch(inner_w),
            _ => WidthMode::Shrink(inner_w),
        };
        let mut frags: Vec<Fragment> = c
            .children
            .iter()
            .map(|ch| self.node(ch, style, mode, None, Axis::Vertical))
            .collect();
        let gaps = flow.gap * c.children.len().saturating_sub(1) as f32;
        let total = |frags: &[Fragment]| frags.iter().map(|f| f.h).sum::<f32>() + gaps;

        // Hand free height to spacers and `height: fill` children.
        if let Some(ih) = inner_h {
            let free = ih - total(&frags);
            let growers: Vec<usize> = c
                .children
                .iter()
                .enumerate()
                .filter(|(_, ch)| grows(ch, Axis::Vertical))
                .map(|(i, _)| i)
                .collect();
            if free > 0.0 && !growers.is_empty() {
                let share = free / growers.len() as f32;
                for i in growers {
                    let target = frags[i].h + share;
                    frags[i] = self.node(&c.children[i], style, mode, Some(target), Axis::Vertical);
                }
            }
        }

        let used = total(&frags);
        let free = inner_h.map_or(0.0, |ih| (ih - used).max(0.0));
        let (mut y, extra) = distribute(flow.justify, free, frags.len());
        let mut out = Fragment::new(inner_w, inner_h.unwrap_or(used));
        for frag in frags {
            let x = match flow.align {
                Align::Center => (inner_w - frag.w) / 2.0,
                Align::End => inner_w - frag.w,
                Align::Start | Align::Stretch => 0.0,
            };
            let h = frag.h;
            out.place(frag, x, y);
            y += h + flow.gap + extra;
        }
        out
    }

    fn row(
        &mut self,
        c: &Container,
        style: Computed,
        inner_w: f32,
        inner_h: Option<f32>,
    ) -> Fragment {
        let flow = &c.flow;
        let n = c.children.len();
        let gaps = flow.gap * n.saturating_sub(1) as f32;

        // 1. Decide every child's width.
        let mut widths = vec![0.0f32; n];
        let mut shrinkable = vec![false; n];
        let mut growers = Vec::new();
        for (i, ch) in c.children.iter().enumerate() {
            if grows(ch, Axis::Horizontal) {
                growers.push(i);
                continue;
            }
            widths[i] = match ch {
                Node::Divider(d) => d.thickness,
                Node::Spacer(s) => match s.width {
                    Size::Px(p) => p,
                    _ => 0.0,
                },
                _ => match frame_of(ch).map(|f| f.width) {
                    Some(Size::Px(p)) => p,
                    _ => {
                        shrinkable[i] = true;
                        self.max_content(ch, style)
                    }
                },
            };
        }
        let mut free = inner_w - gaps - widths.iter().sum::<f32>();
        if free < 0.0 {
            // Too wide: shrink content-sized children proportionally; their
            // text wraps.
            let total: f32 = (0..n).filter(|i| shrinkable[*i]).map(|i| widths[i]).sum();
            if total > 0.0 {
                let factor = ((total + free) / total).max(0.0);
                for i in (0..n).filter(|i| shrinkable[*i]) {
                    widths[i] *= factor;
                }
            }
            free = 0.0;
        } else if !growers.is_empty() {
            let share = free / growers.len() as f32;
            for &i in &growers {
                widths[i] = match frame_of(&c.children[i]).and_then(|f| f.max_width) {
                    Some(m) => share.min(m),
                    None => share,
                };
            }
            free = inner_w - gaps - widths.iter().sum::<f32>();
        }

        // 2. Lay out at those widths.
        let mut frags: Vec<Fragment> = c
            .children
            .iter()
            .zip(&widths)
            .map(|(ch, w)| self.node(ch, style, WidthMode::Exact(*w), None, Axis::Horizontal))
            .collect();
        let row_h = inner_h.unwrap_or_else(|| frags.iter().map(|f| f.h).fold(0.0, f32::max));

        // 3. Stretch children to the row's height.
        for (i, ch) in c.children.iter().enumerate() {
            let stretch = match ch {
                Node::Divider(_) => true,
                Node::Image(_) => false,
                Node::Spacer(_) => false,
                _ => {
                    flow.align == Align::Stretch
                        && frame_of(ch).is_some_and(|f| f.height == Size::Auto)
                }
            };
            if stretch && frags[i].h != row_h {
                frags[i] = self.node(
                    ch,
                    style,
                    WidthMode::Exact(widths[i]),
                    Some(row_h),
                    Axis::Horizontal,
                );
            }
        }

        // 4. Position.
        let (mut x, extra) = distribute(flow.justify, free.max(0.0), n);
        let mut out = Fragment::new(inner_w, row_h);
        for frag in frags {
            let y = match flow.align {
                Align::Center => (row_h - frag.h) / 2.0,
                Align::End => row_h - frag.h,
                Align::Start | Align::Stretch => 0.0,
            };
            let w = frag.w;
            out.place(frag, x, y);
            x += w + flow.gap + extra;
        }
        out
    }

    // ---- intrinsic widths ----------------------------------------------------

    /// Width a node would take if it never had to wrap.
    fn max_content(&mut self, n: &Node, style: Computed) -> f32 {
        match n {
            Node::Container(c) => self.container_max_content(c, style),
            Node::Text(t) => {
                let s = style.with(&t.text);
                let f = &t.frame;
                let w = match f.width {
                    Size::Px(p) => p,
                    _ => {
                        self.text_max_content(&t.content, s, t.link.as_deref())
                            + 2.0 * (f.padding_x + f.border)
                    }
                };
                f.max_width.map_or(w, |m| w.min(m))
            }
            Node::Image(i) => self.image_size(i, f32::INFINITY).0,
            Node::Divider(d) => match d.width {
                Size::Px(p) => p,
                _ => 0.0,
            },
            Node::Spacer(s) => match s.width {
                Size::Px(p) => p,
                _ => 0.0,
            },
        }
    }

    fn container_max_content(&mut self, c: &Container, parent: Computed) -> f32 {
        let f = &c.frame;
        if let Size::Px(p) = f.width {
            return p;
        }
        let style = parent.with(&c.text);
        let widths: Vec<f32> = c
            .children
            .iter()
            .map(|ch| self.max_content(ch, style))
            .collect();
        let inner = match c.kind {
            ContainerKind::Row => {
                widths.iter().sum::<f32>() + c.flow.gap * c.children.len().saturating_sub(1) as f32
            }
            _ => widths.iter().copied().fold(0.0, f32::max),
        };
        let w = inner + 2.0 * (f.padding_x + f.border);
        f.max_width.map_or(w, |m| w.min(m))
    }

    // ---- text ----------------------------------------------------------------

    fn text_block(
        &mut self,
        t: &TextBlock,
        parent: Computed,
        mode: WidthMode,
        forced_h: Option<f32>,
    ) -> Fragment {
        let style = parent.with(&t.text);
        let f = &t.frame;
        let w = self.resolve_width(f.width, f.max_width, mode, |e| {
            e.text_max_content(&t.content, style, t.link.as_deref())
                + 2.0 * (f.padding_x + f.border)
        });
        let (ix, iy) = (f.padding_x + f.border, f.padding_y + f.border);
        let inner_w = (w - 2.0 * ix).max(0.0);
        let mut content = self.paragraph(&t.content, style, inner_w, t.align, t.link.as_deref());
        let h = Self::explicit_height(f, forced_h).unwrap_or(content.h + 2.0 * iy);
        if t.kind == TextKind::Button {
            // Button labels sit in the vertical middle when stretched.
            let extra = (h - 2.0 * iy - content.h).max(0.0) / 2.0;
            let mut shifted = Fragment::new(content.w, content.h);
            shifted.place(content, 0.0, extra);
            content = shifted;
        }
        let mut frag = Self::framed(f, w, h, content);
        if let Some(to) = &t.link {
            frag.links.clear();
            frag.links.push(LinkArea {
                rect: Rect::new(0.0, 0.0, w, h),
                target: to.clone(),
            });
        }
        frag
    }

    fn text_max_content(&mut self, content: &[Inline], style: Computed, link: Option<&str>) -> f32 {
        let atoms = self.atoms(content, style, link);
        let mut best: f32 = 0.0;
        let mut line = 0.0;
        for atom in &atoms {
            match atom.kind {
                AtomKind::Break => {
                    best = best.max(line);
                    line = 0.0;
                }
                _ => line += atom.width(),
            }
        }
        best.max(line).ceil()
    }

    /// Splits inline content into words, spaces and line breaks. A word may
    /// consist of several differently styled parts (`"Hel" bold "lo"`).
    fn atoms(&mut self, content: &[Inline], style: Computed, link: Option<&str>) -> Vec<Atom> {
        let mut pieces = Vec::new();
        flatten(content, style, link, &mut pieces);
        let mut atoms: Vec<Atom> = Vec::new();
        for piece in pieces {
            for (kind, text) in segments(&piece.text) {
                if kind == AtomKind::Break {
                    atoms.push(Atom {
                        kind,
                        parts: Vec::new(),
                    });
                    continue;
                }
                let face = piece.style.face();
                let (glyphs, width) = self.fonts.shape(face, piece.style.size, text);
                let part = Part {
                    face,
                    style: piece.style,
                    link: piece.link.clone(),
                    glyphs,
                    width,
                    visible: kind == AtomKind::Word,
                };
                match atoms.last_mut() {
                    Some(last) if last.kind == kind && kind == AtomKind::Word => {
                        last.parts.push(part)
                    }
                    _ => atoms.push(Atom {
                        kind,
                        parts: vec![part],
                    }),
                }
            }
        }
        atoms
    }

    fn paragraph(
        &mut self,
        content: &[Inline],
        style: Computed,
        max_w: f32,
        align: TextAlign,
        link: Option<&str>,
    ) -> Fragment {
        let atoms = self.atoms(content, style, link);

        // Greedy line breaking.
        let mut lines: Vec<Vec<&Atom>> = vec![Vec::new()];
        let mut width = 0.0;
        for atom in &atoms {
            let line = lines.last_mut().expect("at least one line");
            match atom.kind {
                AtomKind::Break => {
                    lines.push(Vec::new());
                    width = 0.0;
                }
                // Text is literal: spaces are kept, also at the start of a
                // line. Only spaces where a line wraps are dropped.
                AtomKind::Space => {
                    line.push(atom);
                    width += atom.width();
                }
                AtomKind::Word => {
                    let has_word = line.iter().any(|a| a.kind == AtomKind::Word);
                    if has_word && width + atom.width() > max_w + 0.01 {
                        while line.last().is_some_and(|a| a.kind == AtomKind::Space) {
                            line.pop();
                        }
                        lines.push(vec![atom]);
                        width = atom.width();
                    } else {
                        line.push(atom);
                        width += atom.width();
                    }
                }
            }
        }

        let mut out = Fragment::new(max_w, 0.0);
        let mut y = 0.0;
        for mut line in lines {
            while line.last().is_some_and(|a| a.kind == AtomKind::Space) {
                line.pop();
            }
            let parts = || line.iter().flat_map(|a| a.parts.iter());
            let line_h = parts()
                .map(|p| p.style.size * LINE_HEIGHT)
                .fold(style.size * LINE_HEIGHT, f32::max);
            let baseline = parts()
                .map(|p| self.fonts.baseline(p.face, p.style.size))
                .fold(self.fonts.baseline(style.face(), style.size), f32::max);
            let line_w: f32 = parts().map(|p| p.width).sum();
            let mut x = match align {
                TextAlign::Start => 0.0,
                TextAlign::Center => ((max_w - line_w) / 2.0).max(0.0),
                TextAlign::End => (max_w - line_w).max(0.0),
            };
            for part in parts() {
                let s = part.style;
                if part.visible {
                    out.items.push(Item::Glyphs {
                        face: part.face,
                        size: s.size,
                        color: s.color,
                        x,
                        baseline: y + baseline,
                        glyphs: part.glyphs.clone(),
                    });
                }
                if s.underline {
                    out.items.push(Item::Fill {
                        rect: Rect::new(
                            x,
                            y + baseline + s.size * 0.12,
                            part.width,
                            (s.size / 15.0).max(1.0),
                        ),
                        color: s.color,
                        radius: 0.0,
                    });
                }
                if let Some(target) = &part.link {
                    out.links.push(LinkArea {
                        rect: Rect::new(x, y, part.width, line_h),
                        target: target.clone(),
                    });
                }
                x += part.width;
            }
            y += line_h;
        }
        out.h = y;
        out
    }

    // ---- images --------------------------------------------------------------

    /// Outer size of an image given the space available.
    fn image_size(&mut self, img: &Image, avail: f32) -> (f32, f32) {
        let f = &img.frame;
        let insets = 2.0 * (f.padding_x + f.border);
        let natural = self.images.natural_size(&img.src);
        let (nw, nh) = natural.unwrap_or((240.0, 160.0));
        let ratio = if nw > 0.0 { nh / nw } else { 1.0 };
        let w = match (f.width, f.height) {
            (Size::Px(w), _) => w,
            (Size::Fill, _) => avail,
            (Size::Auto, Size::Px(h)) => (h - 2.0 * (f.padding_y + f.border)) / ratio + insets,
            (Size::Auto, _) => (nw + insets).min(avail),
        };
        let w = f.max_width.map_or(w, |m| w.min(m));
        let h = match f.height {
            Size::Px(h) => h,
            _ => (w - insets) * ratio + 2.0 * (f.padding_y + f.border),
        };
        (w, h)
    }

    fn image(&mut self, img: &Image, style: Computed, mode: WidthMode) -> Fragment {
        let f = &img.frame;
        let (w, h) = match mode {
            WidthMode::Exact(w) => {
                let fixed = Image {
                    frame: Frame {
                        width: Size::Px(w),
                        ..f.clone()
                    },
                    ..img.clone()
                };
                self.image_size(&fixed, w)
            }
            WidthMode::Stretch(a) | WidthMode::Shrink(a) => self.image_size(img, a),
        };
        let (ix, iy) = (f.padding_x + f.border, f.padding_y + f.border);
        let inner = Rect::new(0.0, 0.0, (w - 2.0 * ix).max(0.0), (h - 2.0 * iy).max(0.0));
        let mut content = Fragment::new(inner.w, inner.h);
        if self.images.get(&img.src).is_some() {
            content.items.push(Item::Image {
                rect: inner,
                src: img.src.clone(),
                radius: (f.radius - f.padding_x.max(f.padding_y)).max(0.0),
            });
        } else {
            // Missing image: show the alt text, which is required for exactly
            // this reason.
            content.items.push(Item::Fill {
                rect: inner,
                color: Color(0xececf1ff),
                radius: 0.0,
            });
            let muted = Computed {
                color: Color(0x6b6b76ff),
                size: 14.0,
                italic: true,
                ..style
            };
            let label = [Inline::Text(img.alt.clone())];
            let text = self.paragraph(
                &label,
                muted,
                (inner.w - 16.0).max(0.0),
                TextAlign::Center,
                None,
            );
            let ty = ((inner.h - text.h) / 2.0).max(0.0);
            content.place(text, 8.0, ty);
        }
        Self::framed(f, w, h, content)
    }
}

/// Does this child absorb free space along `axis`?
fn grows(n: &Node, axis: Axis) -> bool {
    let pick = |w: Size, h: Size| if axis == Axis::Horizontal { w } else { h };
    match n {
        Node::Spacer(s) => matches!(pick(s.width, s.height), Size::Auto | Size::Fill),
        Node::Divider(_) => false,
        other => frame_of(other).is_some_and(|f| pick(f.width, f.height) == Size::Fill),
    }
}

fn frame_of(n: &Node) -> Option<&Frame> {
    match n {
        Node::Container(c) => Some(&c.frame),
        Node::Text(t) => Some(&t.frame),
        Node::Image(i) => Some(&i.frame),
        Node::Divider(_) | Node::Spacer(_) => None,
    }
}

/// Start offset and extra gap for `justify`.
fn distribute(justify: Justify, free: f32, count: usize) -> (f32, f32) {
    match justify {
        Justify::Start => (0.0, 0.0),
        Justify::Center => (free / 2.0, 0.0),
        Justify::End => (free, 0.0),
        Justify::Between if count > 1 => (0.0, free / (count - 1) as f32),
        Justify::Between => (0.0, 0.0),
    }
}

struct Piece {
    text: String,
    style: Computed,
    link: Option<String>,
}

fn flatten(content: &[Inline], style: Computed, link: Option<&str>, out: &mut Vec<Piece>) {
    for inline in content {
        match inline {
            Inline::Text(s) => out.push(Piece {
                text: s.clone(),
                style,
                link: link.map(str::to_string),
            }),
            Inline::Span(span) => flatten(
                &span.content,
                style.with(&span.text),
                span.link.as_deref().or(link),
                out,
            ),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AtomKind {
    Word,
    Space,
    Break,
}

struct Part {
    face: Face,
    style: Computed,
    link: Option<String>,
    glyphs: Vec<(u16, f32)>,
    width: f32,
    visible: bool,
}

struct Atom {
    kind: AtomKind,
    parts: Vec<Part>,
}

impl Atom {
    fn width(&self) -> f32 {
        self.parts.iter().map(|p| p.width).sum()
    }
}

/// Splits text into runs of word characters, runs of spaces, and `\n`.
fn segments(text: &str) -> Vec<(AtomKind, &str)> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut current: Option<AtomKind> = None;
    for (i, c) in text.char_indices() {
        let kind = if c == '\n' {
            AtomKind::Break
        } else if c.is_whitespace() {
            AtomKind::Space
        } else {
            AtomKind::Word
        };
        if current != Some(kind) || kind == AtomKind::Break {
            if let Some(k) = current {
                if i > start {
                    out.push((k, &text[start..i]));
                }
            }
            start = i;
            current = Some(kind);
        }
    }
    if let Some(k) = current {
        out.push((k, &text[start..]));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lay(src: &str, width: f32) -> Layout {
        let doc = slml_syntax::compile(src).unwrap();
        let fonts = Fonts::load();
        let mut images = ImageStore::new(".");
        layout(&doc, &fonts, &mut images, width, 0.0)
    }

    fn fills(l: &Layout) -> Vec<Rect> {
        l.items
            .iter()
            .filter_map(|i| match i {
                Item::Fill { rect, .. } => Some(*rect),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn segments_split_words_spaces_breaks() {
        let s = segments("ab  c\n\nd");
        let kinds: Vec<_> = s.iter().map(|(k, t)| (*k, *t)).collect();
        assert_eq!(
            kinds,
            vec![
                (AtomKind::Word, "ab"),
                (AtomKind::Space, "  "),
                (AtomKind::Word, "c"),
                (AtomKind::Break, "\n"),
                (AtomKind::Break, "\n"),
                (AtomKind::Word, "d"),
            ]
        );
    }

    #[test]
    fn column_stacks_with_gap_and_padding() {
        let l = lay(
            "slml 0.1 page(padding: 10, gap: 5) { column(height: 20, fill: #f00) column(height: 30, fill: #0f0) }",
            200.0,
        );
        // fills[0] is the page background.
        let f = fills(&l);
        assert_eq!(f[1], Rect::new(10.0, 10.0, 180.0, 20.0));
        assert_eq!(f[2], Rect::new(10.0, 35.0, 180.0, 30.0));
        assert_eq!(l.height, 75.0);
    }

    #[test]
    fn row_shares_space_between_fill_children() {
        let l = lay(
            "slml 0.1 page(padding: 0) { row(gap: 10) { column(width: 50, height: 10, fill: #f00) column(width: fill, height: 10, fill: #0f0) column(width: fill, height: 10, fill: #00f) } }",
            310.0,
        );
        let f = fills(&l);
        assert_eq!(f[1], Rect::new(0.0, 0.0, 50.0, 10.0));
        assert_eq!(f[2], Rect::new(60.0, 0.0, 120.0, 10.0));
        assert_eq!(f[3], Rect::new(190.0, 0.0, 120.0, 10.0));
    }

    #[test]
    fn text_wraps_to_width() {
        let one = lay(
            r#"slml 0.1 page(padding: 0) { text "lorem ipsum dolor sit amet" }"#,
            1000.0,
        );
        let narrow = lay(
            r#"slml 0.1 page(padding: 0) { text "lorem ipsum dolor sit amet" }"#,
            60.0,
        );
        assert!(
            narrow.height >= one.height * 3.0,
            "{} vs {}",
            narrow.height,
            one.height
        );
    }

    #[test]
    fn leading_spaces_after_line_break_are_kept() {
        let glyph_x = |src: &str| {
            let l = lay(src, 400.0);
            l.items
                .iter()
                .rev()
                .find_map(|i| match i {
                    Item::Glyphs { x, .. } => Some(*x),
                    _ => None,
                })
                .unwrap()
        };
        let plain = glyph_x(r#"slml 0.1 page(padding: 0) { text "a\nb" }"#);
        let indented = glyph_x(r#"slml 0.1 page(padding: 0) { text "a\n    b" }"#);
        assert!(indented > plain + 10.0, "{indented} vs {plain}");
    }

    #[test]
    fn spacer_pushes_to_the_end() {
        let l = lay(
            "slml 0.1 page(padding: 0) { row { column(width: 10, height: 10, fill: #f00) spacer column(width: 10, height: 10, fill: #00f) } }",
            100.0,
        );
        assert_eq!(fills(&l)[2].x, 90.0);
    }

    #[test]
    fn links_are_clickable_areas() {
        let l = lay(
            r#"slml 0.1 page(padding: 0) { text { link(to: "a.slml") "here" } button(to: "b.slml") "Go" }"#,
            400.0,
        );
        assert_eq!(l.link_at(2.0, 5.0), Some("a.slml"));
        let button = l.links.iter().find(|a| a.target == "b.slml").unwrap();
        assert_eq!(
            l.link_at(button.rect.x + 1.0, button.rect.y + 1.0),
            Some("b.slml")
        );
    }

    #[test]
    fn missing_image_shows_alt_placeholder() {
        let l = lay(
            r#"slml 0.1 page { image(src: "nope.png", alt: "A cat") }"#,
            400.0,
        );
        assert!(l.items.iter().any(|i| matches!(i, Item::Glyphs { .. })));
    }
}
