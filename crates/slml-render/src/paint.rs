//! Rasterizes a [`Layout`] into a pixel buffer (CPU only, via tiny-skia).

use std::collections::HashMap;

use fontdue::Metrics;
use slml_syntax::model::Color;
use tiny_skia::{
    FillRule, Mask, Paint, PathBuilder, Pixmap, PixmapPaint, PremultipliedColorU8, Stroke,
    Transform,
};

use crate::fonts::{Face, Fonts};
use crate::images::ImageStore;
use crate::layout::{Item, Layout, Rect};

#[derive(Default)]
pub struct Painter {
    glyphs: HashMap<(Face, u16, u32), (Metrics, Vec<u8>)>,
}

impl Painter {
    /// Draws the part of `layout` visible at `scroll_y` into `target`.
    /// `scale` is the device pixel ratio (2.0 on HiDPI screens).
    pub fn paint(
        &mut self,
        layout: &Layout,
        fonts: &Fonts,
        images: &mut ImageStore,
        target: &mut Pixmap,
        scroll_y: f32,
        scale: f32,
    ) {
        target.fill(sk_color(layout.background));
        self.draw(&layout.items, fonts, images, target, scroll_y, scale);
    }

    /// Draws `items` over the current contents of `target`, skipping those
    /// outside the visible part.
    pub fn draw(
        &mut self,
        items: &[Item],
        fonts: &Fonts,
        images: &mut ImageStore,
        target: &mut Pixmap,
        scroll_y: f32,
        scale: f32,
    ) {
        let view_h = target.height() as f32 / scale;
        for item in items {
            let (top, bottom) = item.y_range();
            if bottom < scroll_y || top > scroll_y + view_h {
                continue;
            }
            match item {
                Item::Shadow {
                    rect,
                    radius,
                    blur,
                    offset_y,
                    strength,
                } => {
                    // Approximate a blurred shadow with stacked translucent
                    // rounded rectangles, largest first.
                    let layers = (blur / 2.0).ceil().max(1.0) as usize;
                    let alpha = (strength / layers as f32 * 255.0).round().clamp(1.0, 255.0) as u8;
                    for k in 0..layers {
                        let spread = blur * (1.0 - k as f32 / layers as f32) - blur / 2.0;
                        let r = Rect::new(
                            rect.x - spread,
                            rect.y - spread + offset_y,
                            rect.w + 2.0 * spread,
                            rect.h + 2.0 * spread,
                        );
                        let r = scaled(r, scroll_y, scale);
                        if let Some(path) = rounded_rect(r, (radius + spread).max(0.0) * scale) {
                            target.fill_path(
                                &path,
                                &paint(Color(alpha as u32)),
                                FillRule::Winding,
                                Transform::identity(),
                                None,
                            );
                        }
                    }
                }
                Item::Fill {
                    rect,
                    color,
                    radius,
                } => {
                    let r = scaled(*rect, scroll_y, scale);
                    if let Some(path) = rounded_rect(r, radius * scale) {
                        target.fill_path(
                            &path,
                            &paint(*color),
                            FillRule::Winding,
                            Transform::identity(),
                            None,
                        );
                    }
                }
                Item::Border {
                    rect,
                    width,
                    color,
                    radius,
                } => {
                    let w = width * scale;
                    let r = scaled(*rect, scroll_y, scale);
                    let inset = Rect::new(r.x + w / 2.0, r.y + w / 2.0, r.w - w, r.h - w);
                    if let Some(path) = rounded_rect(inset, (radius * scale - w / 2.0).max(0.0)) {
                        let stroke = Stroke {
                            width: w,
                            ..Stroke::default()
                        };
                        target.stroke_path(
                            &path,
                            &paint(*color),
                            &stroke,
                            Transform::identity(),
                            None,
                        );
                    }
                }
                Item::Glyphs {
                    size,
                    color,
                    x,
                    baseline,
                    glyphs,
                    ..
                } => {
                    let px = size * scale;
                    for g in glyphs {
                        let font = fonts.font(g.face);
                        let (m, cov) = self
                            .glyphs
                            .entry((g.face, g.id, px.to_bits()))
                            .or_insert_with(|| font.rasterize_indexed(g.id, px));
                        let left = ((x + g.x) * scale + m.xmin as f32).round() as i32;
                        let top = ((baseline - scroll_y) * scale - m.height as f32 - m.ymin as f32)
                            .round() as i32;
                        blend_coverage(target, left, top, m.width, m.height, cov, *color);
                    }
                }
                Item::Image { rect, src, radius } => {
                    let Some(img) = images.get(src) else { continue };
                    let r = scaled(*rect, scroll_y, scale);
                    let t = Transform::from_row(
                        r.w / img.width() as f32,
                        0.0,
                        0.0,
                        r.h / img.height() as f32,
                        r.x,
                        r.y,
                    );
                    let mask = if *radius > 0.0 {
                        let mut mask = Mask::new(target.width(), target.height());
                        if let (Some(m), Some(path)) =
                            (mask.as_mut(), rounded_rect(r, radius * scale))
                        {
                            m.fill_path(&path, FillRule::Winding, true, Transform::identity());
                        }
                        mask
                    } else {
                        None
                    };
                    let pp = PixmapPaint {
                        quality: tiny_skia::FilterQuality::Bilinear,
                        ..PixmapPaint::default()
                    };
                    target.draw_pixmap(0, 0, img.as_ref(), &pp, t, mask.as_ref());
                }
            }
        }
    }
}

fn scaled(r: Rect, scroll_y: f32, scale: f32) -> Rect {
    Rect::new(
        r.x * scale,
        (r.y - scroll_y) * scale,
        r.w * scale,
        r.h * scale,
    )
}

fn sk_color(c: Color) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(c.r(), c.g(), c.b(), c.a())
}

fn paint(c: Color) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(c.r(), c.g(), c.b(), c.a());
    p.anti_alias = true;
    p
}

fn rounded_rect(r: Rect, radius: f32) -> Option<tiny_skia::Path> {
    if r.w <= 0.0 || r.h <= 0.0 {
        return None;
    }
    let rad = radius.min(r.w / 2.0).min(r.h / 2.0).max(0.0);
    if rad == 0.0 {
        return Some(PathBuilder::from_rect(tiny_skia::Rect::from_xywh(
            r.x, r.y, r.w, r.h,
        )?));
    }
    // Cubic approximation of a quarter circle.
    let k = rad * 0.552_284_8;
    let (l, t, rt, b) = (r.x, r.y, r.x + r.w, r.y + r.h);
    let mut pb = PathBuilder::new();
    pb.move_to(l + rad, t);
    pb.line_to(rt - rad, t);
    pb.cubic_to(rt - rad + k, t, rt, t + rad - k, rt, t + rad);
    pb.line_to(rt, b - rad);
    pb.cubic_to(rt, b - rad + k, rt - rad + k, b, rt - rad, b);
    pb.line_to(l + rad, b);
    pb.cubic_to(l + rad - k, b, l, b - rad + k, l, b - rad);
    pb.line_to(l, t + rad);
    pb.cubic_to(l, t + rad - k, l + rad - k, t, l + rad, t);
    pb.close();
    pb.finish()
}

/// Source-over blend of a glyph coverage mask in a solid color.
fn blend_coverage(
    target: &mut Pixmap,
    x0: i32,
    y0: i32,
    w: usize,
    h: usize,
    cov: &[u8],
    color: Color,
) {
    let (tw, th) = (target.width() as i32, target.height() as i32);
    let pixels = target.pixels_mut();
    let (cr, cg, cb, ca) = (
        color.r() as u32,
        color.g() as u32,
        color.b() as u32,
        color.a() as u32,
    );
    for gy in 0..h as i32 {
        let py = y0 + gy;
        if py < 0 || py >= th {
            continue;
        }
        for gx in 0..w as i32 {
            let px = x0 + gx;
            if px < 0 || px >= tw {
                continue;
            }
            let c = cov[(gy * w as i32 + gx) as usize] as u32;
            if c == 0 {
                continue;
            }
            let a = c * ca / 255;
            let inv = 255 - a;
            let idx = (py * tw + px) as usize;
            let d = pixels[idx];
            let mix = |s: u32, dv: u8| ((s * a + dv as u32 * inv + 127) / 255) as u8;
            let out = PremultipliedColorU8::from_rgba(
                mix(cr, d.red()),
                mix(cg, d.green()),
                mix(cb, d.blue()),
                (a + (d.alpha() as u32 * inv + 127) / 255) as u8,
            );
            if let Some(out) = out {
                pixels[idx] = out;
            }
        }
    }
}
