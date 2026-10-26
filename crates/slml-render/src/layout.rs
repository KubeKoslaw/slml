//! The layout engine: turns a [`Document`] into positioned paint items.
//!
//! The model is deliberately small (see SPEC.md §5):
//! - `column`/`page`/`form` stack children vertically, `row` horizontally
//!   (optionally wrapping), `grid` in equal columns.
//! - Sizes are `auto` (content), `fill` (share free space) or pixels.
//! - Text wraps at spaces; nothing ever overlaps unless a child is wider
//!   than its parent allows.
//!
//! Each function returns a [`Fragment`] positioned at (0, 0); parents move
//! fragments into place. There are no global passes and no floats.

use slml_syntax::model::*;

use crate::fonts::{Face, Fonts, Glyph, LINE_HEIGHT};
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
    Shadow {
        rect: Rect,
        radius: f32,
        blur: f32,
        offset_y: f32,
        /// Total opacity, 0–1.
        strength: f32,
    },
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
        /// Glyphs with x offsets from `x`.
        glyphs: Vec<Glyph>,
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
            Item::Fill { rect, .. }
            | Item::Border { rect, .. }
            | Item::Image { rect, .. }
            | Item::Shadow { rect, .. } => {
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
            Item::Shadow {
                rect,
                blur,
                offset_y,
                ..
            } => (rect.y - blur, rect.y + rect.h + blur + offset_y),
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
    /// Element ids and the y position of their top edge.
    pub anchors: Vec<(String, f32)>,
}

impl Layout {
    pub fn link_at(&self, x: f32, y: f32) -> Option<&str> {
        self.links
            .iter()
            .rev()
            .find(|l| l.rect.contains(x, y))
            .map(|l| l.target.as_str())
    }

    pub fn anchor(&self, id: &str) -> Option<f32> {
        self.anchors.iter().find(|(a, _)| a == id).map(|(_, y)| *y)
    }
}

#[derive(Debug, Default)]
struct Fragment {
    w: f32,
    h: f32,
    items: Vec<Item>,
    links: Vec<LinkArea>,
    anchors: Vec<(String, f32)>,
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
        for (id, y) in child.anchors {
            self.anchors.push((id, y + dy));
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
    /// The parent (a `row` or `grid`) already decided.
    Exact(f32),
}

impl WidthMode {
    fn avail(self) -> f32 {
        match self {
            WidthMode::Stretch(a) | WidthMode::Shrink(a) | WidthMode::Exact(a) => a,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    Vertical,
    Horizontal,
}

const DEFAULT_BORDER: Color = Color(0xd4d4d8ff);
const FIELD_BORDER: Color = Color(0xc4c4ccff);
const MUTED: Color = Color(0x8a8a94ff);
const DEFAULT_FIELD_WIDTH: f32 = 320.0;

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
        anchors: frag.anchors,
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
            Node::List(l) => self.list(l, style, mode, forced_h),
            Node::Field(f) => self.field(f, style, mode),
            Node::Divider(d) => {
                let avail = mode.avail();
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
                if let Some(id) = &d.id {
                    f.anchors.push((id.clone(), 0.0));
                }
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
        f: &Frame,
        mode: WidthMode,
        max_content: impl FnOnce(&mut Self) -> f32,
    ) -> f32 {
        let w = match mode {
            WidthMode::Exact(w) => w,
            WidthMode::Stretch(avail) => match f.width {
                Size::Px(p) => p,
                _ => avail,
            },
            WidthMode::Shrink(avail) => match f.width {
                Size::Px(p) => p,
                Size::Fill => avail,
                Size::Auto => max_content(self).min(avail),
            },
        };
        let w = f.max_width.map_or(w, |m| w.min(m));
        f.min_width.map_or(w, |m| w.max(m)).max(0.0)
    }

    /// Paints a frame's shadow, background and border around already
    /// laid-out content, and registers its link and anchor.
    fn framed(f: &Frame, w: f32, h: f32, content: Fragment) -> Fragment {
        let mut out = Fragment::new(w, h);
        let rect = Rect::new(0.0, 0.0, w, h);
        let shadow = match f.shadow {
            Shadow::None => None,
            Shadow::Small => Some((6.0, 2.0, 0.14)),
            Shadow::Large => Some((18.0, 8.0, 0.18)),
        };
        if let Some((blur, offset_y, strength)) = shadow {
            out.items.push(Item::Shadow {
                rect,
                radius: f.radius,
                blur,
                offset_y,
                strength,
            });
        }
        if let Some(fill) = f.fill {
            out.items.push(Item::Fill {
                rect,
                color: fill,
                radius: f.radius,
            });
        }
        out.place(content, f.padding_x + f.border, f.padding_y + f.border);
        if f.border > 0.0 {
            out.items.push(Item::Border {
                rect,
                width: f.border,
                color: f.border_color.unwrap_or(DEFAULT_BORDER),
                radius: f.radius,
            });
        }
        if let Some(to) = &f.link {
            out.links.push(LinkArea {
                rect,
                target: to.clone(),
            });
        }
        if let Some(id) = &f.id {
            out.anchors.push((id.clone(), 0.0));
        }
        out
    }

    fn explicit_height(f: &Frame, forced_h: Option<f32>) -> Option<f32> {
        forced_h.or(match f.height {
            Size::Px(h) => Some(h),
            _ => None,
        })
    }

    fn insets(f: &Frame) -> (f32, f32) {
        (f.padding_x + f.border, f.padding_y + f.border)
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
        let w = self.resolve_width(f, mode, |e| e.container_max_content(c, style));
        let (ix, iy) = Self::insets(f);
        let inner_w = (w - 2.0 * ix).max(0.0);
        let explicit_h = Self::explicit_height(f, forced_h);
        let inner_h = explicit_h.map(|h| (h - 2.0 * iy).max(0.0));
        let content = match &c.kind {
            ContainerKind::Row => self.row(c, style, inner_w, inner_h),
            ContainerKind::Grid(spec) => self.grid(c, spec, style, inner_w),
            ContainerKind::Column | ContainerKind::Page | ContainerKind::Form(_) => {
                self.column(c, style, inner_w, inner_h)
            }
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

        // 1. Every child's base width.
        let mut basis = vec![0.0f32; n];
        let mut shrinkable = vec![false; n];
        let mut grower = vec![false; n];
        for (i, ch) in c.children.iter().enumerate() {
            grower[i] = grows(ch, Axis::Horizontal);
            basis[i] = match ch {
                Node::Divider(d) => d.thickness,
                Node::Spacer(s) => match s.width {
                    Size::Px(p) => p,
                    _ => 0.0,
                },
                _ => {
                    let f = frame_of(ch).expect("framed node");
                    match f.width {
                        Size::Px(p) => p,
                        Size::Fill => f.min_width.unwrap_or(0.0),
                        Size::Auto => {
                            shrinkable[i] = true;
                            let w = self.max_content(ch, style);
                            if flow.wrap { w.min(inner_w) } else { w }
                        }
                    }
                }
            };
        }

        // 2. Break into lines (only when wrapping).
        let mut lines: Vec<Vec<usize>> = Vec::new();
        if flow.wrap {
            let mut line: Vec<usize> = Vec::new();
            let mut used = 0.0;
            for (i, &b) in basis.iter().enumerate() {
                let needed = if line.is_empty() {
                    b
                } else {
                    used + flow.gap + b
                };
                if !line.is_empty() && needed > inner_w + 0.01 {
                    lines.push(std::mem::take(&mut line));
                    used = b;
                } else {
                    used = needed;
                }
                line.push(i);
            }
            if !line.is_empty() {
                lines.push(line);
            }
        } else {
            lines.push((0..n).collect());
        }

        // 3. Lay out each line.
        let mut out = Fragment::new(inner_w, 0.0);
        let mut y = 0.0;
        let line_count = lines.len();
        for (li, line) in lines.iter().enumerate() {
            let fixed_h = if flow.wrap { None } else { inner_h };
            let frag = self.row_line(
                c,
                line,
                &basis,
                &shrinkable,
                &grower,
                style,
                inner_w,
                fixed_h,
            );
            let h = frag.h;
            out.place(frag, 0.0, y);
            y += h;
            if li + 1 < line_count {
                y += flow.gap;
            }
        }
        out.h = if flow.wrap { y } else { inner_h.unwrap_or(y) };
        out
    }

    #[allow(clippy::too_many_arguments)]
    fn row_line(
        &mut self,
        c: &Container,
        line: &[usize],
        basis: &[f32],
        shrinkable: &[bool],
        grower: &[bool],
        style: Computed,
        inner_w: f32,
        inner_h: Option<f32>,
    ) -> Fragment {
        let flow = &c.flow;
        let gaps = flow.gap * line.len().saturating_sub(1) as f32;
        let mut widths: Vec<f32> = line.iter().map(|&i| basis[i]).collect();
        let mut free = inner_w - gaps - widths.iter().sum::<f32>();
        let growers: Vec<usize> = (0..line.len()).filter(|&k| grower[line[k]]).collect();

        if free < 0.0 {
            // Too wide: shrink content-sized children proportionally, never
            // below their `min-width`; their text wraps.
            let total: f32 = (0..line.len())
                .filter(|&k| shrinkable[line[k]])
                .map(|k| widths[k])
                .sum();
            if total > 0.0 {
                let factor = ((total + free) / total).max(0.0);
                for k in (0..line.len()).filter(|&k| shrinkable[line[k]]) {
                    let min = frame_of(&c.children[line[k]])
                        .and_then(|f| f.min_width)
                        .unwrap_or(0.0);
                    widths[k] = (widths[k] * factor).max(min);
                }
            }
            free = 0.0;
        } else if !growers.is_empty() {
            let share = free / growers.len() as f32;
            for &k in &growers {
                let w = widths[k] + share;
                widths[k] = match frame_of(&c.children[line[k]]).and_then(|f| f.max_width) {
                    Some(m) => w.min(m),
                    None => w,
                };
            }
            free = (inner_w - gaps - widths.iter().sum::<f32>()).max(0.0);
        }

        let mut frags: Vec<Fragment> = line
            .iter()
            .zip(&widths)
            .map(|(&i, w)| {
                self.node(
                    &c.children[i],
                    style,
                    WidthMode::Exact(*w),
                    None,
                    Axis::Horizontal,
                )
            })
            .collect();
        let row_h = inner_h.unwrap_or_else(|| frags.iter().map(|f| f.h).fold(0.0, f32::max));

        // Stretch children to the line's height.
        for (k, &i) in line.iter().enumerate() {
            let ch = &c.children[i];
            if stretches(ch, flow.align) && frags[k].h != row_h {
                frags[k] = self.node(
                    ch,
                    style,
                    WidthMode::Exact(widths[k]),
                    Some(row_h),
                    Axis::Horizontal,
                );
            }
        }

        let (mut x, extra) = distribute(flow.justify, free, line.len());
        let mut out = Fragment::new(inner_w, row_h);
        for frag in frags {
            let y = cross_offset(flow.align, row_h, frag.h);
            let w = frag.w;
            out.place(frag, x, y);
            x += w + flow.gap + extra;
        }
        out
    }

    fn grid(&mut self, c: &Container, spec: &GridSpec, style: Computed, inner_w: f32) -> Fragment {
        let gap = c.flow.gap;
        let cols = spec.count(inner_w, gap);
        let col_w = ((inner_w - gap * (cols - 1) as f32) / cols as f32).max(0.0);
        let mut out = Fragment::new(inner_w, 0.0);
        let mut y = 0.0;
        let rows = c.children.chunks(cols).count();
        for (ri, chunk) in c.children.chunks(cols).enumerate() {
            let mut frags: Vec<Fragment> = chunk
                .iter()
                .map(|ch| self.node(ch, style, WidthMode::Exact(col_w), None, Axis::Horizontal))
                .collect();
            let row_h = frags.iter().map(|f| f.h).fold(0.0, f32::max);
            for (k, ch) in chunk.iter().enumerate() {
                if stretches(ch, c.flow.align) && frags[k].h != row_h {
                    frags[k] = self.node(
                        ch,
                        style,
                        WidthMode::Exact(col_w),
                        Some(row_h),
                        Axis::Horizontal,
                    );
                }
            }
            for (k, frag) in frags.into_iter().enumerate() {
                let dy = cross_offset(c.flow.align, row_h, frag.h);
                out.place(frag, k as f32 * (col_w + gap), y + dy);
            }
            y += row_h;
            if ri + 1 < rows {
                y += gap;
            }
        }
        out.h = y;
        out
    }

    // ---- intrinsic widths ----------------------------------------------------

    /// Width a node would take if it never had to wrap.
    fn max_content(&mut self, n: &Node, style: Computed) -> f32 {
        let w = match n {
            Node::Container(c) => return self.container_max_content(c, style),
            Node::Text(t) => {
                let s = style.with(&t.text);
                match t.frame.width {
                    Size::Px(p) => p,
                    _ => self.text_max_content(&t.content, s) + 2.0 * Self::insets(&t.frame).0,
                }
            }
            Node::Image(i) => self.image_size(i, f32::INFINITY).0,
            Node::List(l) => {
                let s = style.with(&l.text);
                match l.frame.width {
                    Size::Px(p) => p,
                    _ => self.list_max_content(l, s) + 2.0 * Self::insets(&l.frame).0,
                }
            }
            Node::Field(f) => {
                let s = style.with(&f.text);
                match (&f.kind, f.frame.width) {
                    (_, Size::Px(p)) => p,
                    (FieldKind::Checkbox { .. }, _) => {
                        self.fonts.width(s.face(), s.size, &f.label) + checkbox_size(s) * 1.5
                    }
                    _ => DEFAULT_FIELD_WIDTH,
                }
            }
            Node::Divider(d) => match d.width {
                Size::Px(p) => p,
                _ => 0.0,
            },
            Node::Spacer(s) => match s.width {
                Size::Px(p) => p,
                _ => 0.0,
            },
        };
        match frame_of(n) {
            Some(f) => {
                let w = f.max_width.map_or(w, |m| w.min(m));
                f.min_width.map_or(w, |m| w.max(m))
            }
            None => w,
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
        let gaps = |n: usize| c.flow.gap * n.saturating_sub(1) as f32;
        let inner = match &c.kind {
            ContainerKind::Row => widths.iter().sum::<f32>() + gaps(widths.len()),
            ContainerKind::Grid(spec) => {
                let cols = spec
                    .columns
                    .map_or(1, |n| n as usize)
                    .min(widths.len().max(1));
                let widest = widths.iter().copied().fold(0.0, f32::max);
                let cell = spec.cell_width.map_or(widest, |cw| widest.max(cw));
                cell * cols as f32 + gaps(cols)
            }
            _ => widths.iter().copied().fold(0.0, f32::max),
        };
        let w = inner + 2.0 * Self::insets(f).0;
        let w = f.max_width.map_or(w, |m| w.min(m));
        f.min_width.map_or(w, |m| w.max(m))
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
        let (ix, iy) = Self::insets(f);
        let w = self.resolve_width(f, mode, |e| {
            e.text_max_content(&t.content, style) + 2.0 * ix
        });
        let inner_w = (w - 2.0 * ix).max(0.0);
        let mut content = self.paragraph(&t.content, style, inner_w, t.align);
        let h = Self::explicit_height(f, forced_h).unwrap_or(content.h + 2.0 * iy);
        if matches!(t.kind, TextKind::Button | TextKind::Submit) {
            // Button labels sit in the vertical middle when stretched.
            let extra = (h - 2.0 * iy - content.h).max(0.0) / 2.0;
            let mut shifted = Fragment::new(content.w, content.h);
            shifted.place(content, 0.0, extra);
            content = shifted;
        }
        Self::framed(f, w, h, content)
    }

    fn text_max_content(&mut self, content: &[Inline], style: Computed) -> f32 {
        let atoms = self.atoms(content, style);
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
    fn atoms(&mut self, content: &[Inline], style: Computed) -> Vec<Atom> {
        let mut pieces = Vec::new();
        flatten(content, style, None, &mut pieces);
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
    ) -> Fragment {
        let atoms = self.atoms(content, style);

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

    /// A single line of plain text (labels, placeholders, list markers).
    fn label(&mut self, text: &str, style: Computed, max_w: f32) -> Fragment {
        self.paragraph(
            &[Inline::Text(text.to_string())],
            style,
            max_w,
            TextAlign::Start,
        )
    }

    // ---- lists ---------------------------------------------------------------

    fn markers(l: &List) -> Vec<String> {
        let mut n = 0;
        l.items
            .iter()
            .map(|e| match (e, l.marker) {
                (ListEntry::List(_), _) | (_, Marker::None) => String::new(),
                (ListEntry::Item(_), Marker::Bullet) => "•".to_string(),
                (ListEntry::Item(_), Marker::Number) => {
                    n += 1;
                    format!("{n}.")
                }
            })
            .collect()
    }

    fn list_indent(&self, l: &List, style: Computed) -> (f32, f32) {
        if l.marker == Marker::None {
            return (0.0, 0.0);
        }
        let widest = Self::markers(l)
            .iter()
            .map(|m| self.fonts.width(style.face(), style.size, m))
            .fold(0.0, f32::max);
        (widest, widest + style.size * 0.6)
    }

    fn list_max_content(&mut self, l: &List, style: Computed) -> f32 {
        let (_, indent) = self.list_indent(l, style);
        let widest = l
            .items
            .iter()
            .map(|e| match e {
                ListEntry::Item(t) => {
                    let s = style.with(&t.text);
                    self.text_max_content(&t.content, s)
                }
                ListEntry::List(sub) => {
                    let s = style.with(&sub.text);
                    self.list_max_content(sub, s)
                }
            })
            .fold(0.0, f32::max);
        indent + widest
    }

    fn list(
        &mut self,
        l: &List,
        parent: Computed,
        mode: WidthMode,
        forced_h: Option<f32>,
    ) -> Fragment {
        let style = parent.with(&l.text);
        let f = &l.frame;
        let (ix, iy) = Self::insets(f);
        let w = self.resolve_width(f, mode, |e| e.list_max_content(l, style) + 2.0 * ix);
        let inner_w = (w - 2.0 * ix).max(0.0);
        let (marker_w, indent) = self.list_indent(l, style);
        let markers = Self::markers(l);
        let mut content = Fragment::new(inner_w, 0.0);
        let mut y = 0.0;
        for (k, entry) in l.items.iter().enumerate() {
            if k > 0 {
                y += l.gap;
            }
            let body_w = (inner_w - indent).max(0.0);
            let frag = match entry {
                ListEntry::Item(t) => {
                    let s = style.with(&t.text);
                    let marker = &markers[k];
                    if !marker.is_empty() {
                        let (glyphs, mw) = self.fonts.shape(s.face(), s.size, marker);
                        content.items.push(Item::Glyphs {
                            face: s.face(),
                            size: s.size,
                            color: s.color,
                            x: marker_w - mw,
                            baseline: y + self.fonts.baseline(s.face(), s.size),
                            glyphs,
                        });
                    }
                    self.text_block(t, style, WidthMode::Exact(body_w), None)
                }
                ListEntry::List(sub) => self.list(sub, style, WidthMode::Exact(body_w), None),
            };
            let h = frag.h;
            content.place(frag, indent, y);
            y += h;
        }
        content.h = y;
        let h = Self::explicit_height(f, forced_h).unwrap_or(y + 2.0 * iy);
        Self::framed(f, w, h, content)
    }

    // ---- form fields ---------------------------------------------------------

    fn field(&mut self, field: &Field, parent: Computed, mode: WidthMode) -> Fragment {
        let style = parent.with(&field.text);
        let f = &field.frame;
        let label_style = Computed {
            bold: true,
            size: style.size * 0.875,
            ..style
        };
        let required = if field.required { " *" } else { "" };

        if let FieldKind::Checkbox { checked } = field.kind {
            let w = self.resolve_width(f, mode, |e| {
                e.fonts.width(style.face(), style.size, &field.label) + checkbox_size(style) * 1.5
            });
            let bs = checkbox_size(style);
            let text_x = bs + style.size * 0.5;
            let label = self.label(
                &format!("{}{required}", field.label),
                style,
                (w - text_x).max(0.0),
            );
            let h = label.h.max(bs);
            let mut out = Fragment::new(w, h);
            let top = ((style.size * LINE_HEIGHT - bs) / 2.0).max(0.0);
            let rect = Rect::new(0.0, top, bs, bs);
            let accent = Color(0x2457d6ff);
            out.items.push(Item::Fill {
                rect,
                color: if checked { accent } else { Color::WHITE },
                radius: 4.0,
            });
            if checked {
                let mark = Rect::new(rect.x + bs * 0.3, rect.y + bs * 0.3, bs * 0.4, bs * 0.4);
                out.items.push(Item::Fill {
                    rect: mark,
                    color: Color::WHITE,
                    radius: 1.0,
                });
            } else {
                out.items.push(Item::Border {
                    rect,
                    width: 1.0,
                    color: FIELD_BORDER,
                    radius: 4.0,
                });
            }
            out.place(label, text_x, 0.0);
            if let Some(id) = &f.id {
                out.anchors.push((id.clone(), 0.0));
            }
            return out;
        }

        let w = self.resolve_width(f, mode, |_| DEFAULT_FIELD_WIDTH);
        let mut out = Fragment::new(w, 0.0);
        let label = self.label(&format!("{}{required}", field.label), label_style, w);
        let mut y = label.h + 6.0;
        out.place(label, 0.0, 0.0);

        let line_h = style.size * LINE_HEIGHT;
        let (pad_x, pad_y) = (12.0, 9.0);
        let box_h = match &field.kind {
            FieldKind::TextArea { rows, .. } => *rows as f32 * line_h + 2.0 * pad_y,
            _ => line_h + 2.0 * pad_y,
        };
        let rect = Rect::new(0.0, y, w, box_h);
        let radius = if f.radius > 0.0 { f.radius } else { 6.0 };
        out.items.push(Item::Fill {
            rect,
            color: f.fill.unwrap_or(Color::WHITE),
            radius,
        });
        out.items.push(Item::Border {
            rect,
            width: if f.border > 0.0 { f.border } else { 1.0 },
            color: f.border_color.unwrap_or(FIELD_BORDER),
            radius,
        });
        let placeholder = Computed {
            color: MUTED,
            ..style
        };
        let inner_w = (w - 2.0 * pad_x).max(0.0);
        let shown = match &field.kind {
            FieldKind::Input {
                kind,
                value,
                placeholder: ph,
            } => match (value, ph) {
                (Some(v), _) if *kind == InputKind::Password => {
                    Some(("•".repeat(v.chars().count()), style))
                }
                (Some(v), _) => Some((v.clone(), style)),
                (None, Some(p)) => Some((p.clone(), placeholder)),
                _ => None,
            },
            FieldKind::TextArea {
                value,
                placeholder: ph,
                ..
            } => match (value, ph) {
                (Some(v), _) => Some((v.clone(), style)),
                (None, Some(p)) => Some((p.clone(), placeholder)),
                _ => None,
            },
            FieldKind::Select { options } => options
                .iter()
                .find(|o| o.selected)
                .or(options.first())
                .map(|o| (o.label.clone(), style)),
            FieldKind::Checkbox { .. } => None,
        };
        if let FieldKind::Select { .. } = field.kind {
            // A small chevron drawn from two bars.
            let cx = w - pad_x - 6.0;
            let cy = y + box_h / 2.0;
            for (dx, sign) in [(-4.0f32, 1.0f32), (0.0, -1.0)] {
                for i in 0..4 {
                    let fi = i as f32;
                    out.items.push(Item::Fill {
                        rect: Rect::new(
                            cx + dx + fi,
                            cy - 2.0 + if sign > 0.0 { fi } else { 3.0 - fi },
                            1.5,
                            1.5,
                        ),
                        color: MUTED,
                        radius: 0.0,
                    });
                }
            }
        }
        if let Some((text, s)) = shown {
            let frag = self.paragraph(&[Inline::Text(text)], s, inner_w, TextAlign::Start);
            out.place(frag, pad_x, y + pad_y);
        }
        y += box_h;
        out.h = y;
        if let Some(id) = &f.id {
            out.anchors.push((id.clone(), 0.0));
        }
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
        let (ix, iy) = Self::insets(f);
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
            let text = self.paragraph(&label, muted, (inner.w - 16.0).max(0.0), TextAlign::Center);
            let ty = ((inner.h - text.h) / 2.0).max(0.0);
            content.place(text, 8.0, ty);
        }
        Self::framed(f, w, h, content)
    }
}

fn checkbox_size(style: Computed) -> f32 {
    (style.size * 1.1).round()
}

/// Is a child stretched to its line's height (in rows and grids)?
fn stretches(n: &Node, align: Align) -> bool {
    match n {
        Node::Divider(_) => true,
        Node::Image(_) | Node::Spacer(_) | Node::Field(_) => false,
        _ => align == Align::Stretch && frame_of(n).is_some_and(|f| f.height == Size::Auto),
    }
}

fn cross_offset(align: Align, line: f32, h: f32) -> f32 {
    match align {
        Align::Center => (line - h) / 2.0,
        Align::End => line - h,
        Align::Start | Align::Stretch => 0.0,
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
        Node::List(l) => Some(&l.frame),
        Node::Field(f) => Some(&f.frame),
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
    glyphs: Vec<Glyph>,
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
            if let Some(k) = current
                && i > start
            {
                out.push((k, &text[start..i]));
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
    fn wrapping_row_moves_children_to_new_lines() {
        let src = "slml 0.1 page(padding: 0) { row(wrap: true, gap: 10) { column(width: fill, min-width: 100, height: 10, fill: #f00) column(width: fill, min-width: 100, height: 10, fill: #0f0) column(width: fill, min-width: 100, height: 10, fill: #00f) } }";
        let wide = fills(&lay(src, 400.0));
        assert!(wide[1..].iter().all(|r| r.y == 0.0), "{wide:?}");
        let narrow = fills(&lay(src, 250.0));
        // Two fit on the first line, the third wraps and fills the width.
        assert_eq!(narrow[1].y, 0.0);
        assert_eq!(narrow[2].y, 0.0);
        assert_eq!(narrow[3], Rect::new(0.0, 20.0, 250.0, 10.0));
    }

    #[test]
    fn grid_with_cell_width_is_responsive() {
        let src = "slml 0.1 page(padding: 0) { grid(cell-width: 100, gap: 10) { column(height: 10, fill: #f00) column(height: 10, fill: #0f0) column(height: 10, fill: #00f) } }";
        let wide = fills(&lay(src, 400.0));
        assert_eq!(wide[1].w, wide[2].w);
        assert!(wide[1..].iter().all(|r| r.y == 0.0));
        let narrow = fills(&lay(src, 210.0));
        assert_eq!(narrow[1], Rect::new(0.0, 0.0, 100.0, 10.0));
        assert_eq!(narrow[3], Rect::new(0.0, 20.0, 100.0, 10.0));
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
    fn links_and_anchors() {
        let l = lay(
            r##"slml 0.1 page(padding: 0) { text { link(to: "a.slml") "here" } button(to: "b.slml") "Go" heading(id: "end") "End" text { link(to: "#end") "x" } }"##,
            400.0,
        );
        assert_eq!(l.link_at(2.0, 5.0), Some("a.slml"));
        let button = l.links.iter().find(|a| a.target == "b.slml").unwrap();
        assert_eq!(
            l.link_at(button.rect.x + 1.0, button.rect.y + 1.0),
            Some("b.slml")
        );
        assert!(l.anchor("end").unwrap() > button.rect.y);
    }

    #[test]
    fn numbered_list_indents_items() {
        let l = lay(
            r#"slml 0.1 page(padding: 0) { list(marker: number) { item "one" item "two" list { item "deep" } } }"#,
            400.0,
        );
        let xs: Vec<f32> = l
            .items
            .iter()
            .filter_map(|i| match i {
                Item::Glyphs { x, .. } => Some(*x),
                _ => None,
            })
            .collect();
        // marker, text, marker, text, nested bullet, nested text
        assert_eq!(xs.len(), 6, "{xs:?}");
        assert!(xs[1] > xs[0] && xs[5] > xs[1], "{xs:?}");
    }

    #[test]
    fn form_fields_have_label_and_box() {
        let l = lay(
            r#"slml 0.1 page(padding: 0) { form(to: "/send") { input(name: "a", label: "Name", placeholder: "Ann") checkbox(name: "b", label: "Yes", checked: true) submit "Send" } }"#,
            400.0,
        );
        let borders = l
            .items
            .iter()
            .filter(|i| matches!(i, Item::Border { .. }))
            .count();
        assert_eq!(borders, 1, "input box border (checked box has none)");
        assert!(l.height > 100.0);
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
