//! Crosshair geometry and software rendering into an ARGB8888 (premultiplied) buffer.
//!
//! Everything here works in *physical* buffer pixels. The config is in logical pixels;
//! `Layout::new` multiplies by the output scale (e.g. 1.25 or 1.5) and snaps the
//! shapes to whole pixels so lines stay crisp and the crosshair is centred exactly.

use crate::config::{Color, Config, Shape};

/// Axis-aligned rectangle, half-open: x0 <= x < x1, y0 <= y < y1 (physical pixels).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x0: i64,
    pub y0: i64,
    pub x1: i64,
    pub y1: i64,
}

impl Rect {
    fn grow(&self, by: i64) -> Rect {
        Rect {
            x0: self.x0 - by,
            y0: self.y0 - by,
            x1: self.x1 + by,
            y1: self.y1 + by,
        }
    }
    fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x0 as f64 && x < self.x1 as f64 && y >= self.y0 as f64 && y < self.y1 as f64
    }
}

/// The crosshair as concrete pixel primitives for one buffer size and scale.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    /// Exact centre (physical px, may be fractional): screen centre + offset.
    pub cx: f64,
    pub cy: f64,
    /// Arms of the cross (empty for a dot).
    pub rects: Vec<Rect>,
    /// Dot radius in physical px (0 = no dot).
    pub radius: f64,
    /// Outline width in physical px (0 = no outline).
    pub outline: i64,
}

/// Round a length in physical px to whole pixels, at least 1.
fn px(len: f64) -> i64 {
    (len.round() as i64).max(1)
}

/// A run of `len` whole pixels centred on `c` (as close as the pixel grid allows).
fn centred(c: f64, len: i64) -> (i64, i64) {
    let start = (c - len as f64 / 2.0).round() as i64;
    (start, start + len)
}

impl Layout {
    pub fn new(cfg: &Config, buf_w: u32, buf_h: u32, scale: f64) -> Layout {
        let cx = buf_w as f64 / 2.0 + cfg.offset_x * scale;
        let cy = buf_h as f64 / 2.0 + cfg.offset_y * scale;
        let outline = if cfg.outline && cfg.outline_thickness > 0.0 {
            px(cfg.outline_thickness * scale)
        } else {
            0
        };
        // All lengths share the parity of the line thickness, so every part is centred
        // on the same point (an odd and an even run can't share a centre on the pixel
        // grid — that showed as a half-pixel skew at 125 %).
        let thick = px(cfg.thickness * scale);
        let same_parity = |len: i64| if (len - thick) % 2 == 0 { len } else { len + 1 };
        let size = same_parity(px(cfg.size * scale).max(thick));
        let mut rects = Vec::new();
        let mut radius = 0.0;
        match cfg.shape {
            Shape::Dot => radius = (cfg.size * scale / 2.0).max(0.5),
            Shape::Cross => {
                let (x0, x1) = centred(cx, size);
                let (ty0, ty1) = centred(cy, thick);
                rects.push(Rect {
                    x0,
                    y0: ty0,
                    x1,
                    y1: ty1,
                });
                let (y0, y1) = centred(cy, size);
                let (tx0, tx1) = centred(cx, thick);
                rects.push(Rect {
                    x0: tx0,
                    y0,
                    x1: tx1,
                    y1,
                });
            }
            Shape::CrossGap => {
                // Full extent and the empty middle, both centred like the arms.
                let gap = if cfg.gap > 0.0 {
                    same_parity((cfg.gap * scale).round() as i64)
                } else {
                    0
                };
                let gap = gap.min(size - 2);
                let (x0, x1) = centred(cx, size);
                let (y0, y1) = centred(cy, size);
                let (gx0, gx1) = centred(cx, gap);
                let (gy0, gy1) = centred(cy, gap);
                let (tx0, tx1) = centred(cx, thick);
                let (ty0, ty1) = centred(cy, thick);
                // left, right, top, bottom
                rects.push(Rect {
                    x0,
                    y0: ty0,
                    x1: gx0,
                    y1: ty1,
                });
                rects.push(Rect {
                    x0: gx1,
                    y0: ty0,
                    x1,
                    y1: ty1,
                });
                rects.push(Rect {
                    x0: tx0,
                    y0,
                    x1: tx1,
                    y1: gy0,
                });
                rects.push(Rect {
                    x0: tx0,
                    y0: gy1,
                    x1: tx1,
                    y1,
                });
                rects.retain(|r| r.x1 > r.x0 && r.y1 > r.y0);
            }
        }
        Layout {
            cx,
            cy,
            rects,
            radius,
            outline,
        }
    }

    /// Bounding box including the outline, clipped to the buffer.
    pub fn bounds(&self, buf_w: u32, buf_h: u32) -> Option<Rect> {
        let mut b: Option<Rect> = None;
        let mut add = |r: Rect| {
            b = Some(match b {
                None => r,
                Some(o) => Rect {
                    x0: o.x0.min(r.x0),
                    y0: o.y0.min(r.y0),
                    x1: o.x1.max(r.x1),
                    y1: o.y1.max(r.y1),
                },
            })
        };
        for r in &self.rects {
            add(r.grow(self.outline));
        }
        if self.radius > 0.0 {
            let r = self.radius + self.outline as f64 + 1.0;
            add(Rect {
                x0: (self.cx - r).floor() as i64,
                y0: (self.cy - r).floor() as i64,
                x1: (self.cx + r).ceil() as i64,
                y1: (self.cy + r).ceil() as i64,
            });
        }
        let b = b?;
        let c = Rect {
            x0: b.x0.max(0),
            y0: b.y0.max(0),
            x1: b.x1.min(buf_w as i64),
            y1: b.y1.min(buf_h as i64),
        };
        (c.x1 > c.x0 && c.y1 > c.y0).then_some(c)
    }

    /// Coverage (0..1) of pixel (px, py) by the shape grown by `grow` px.
    fn coverage(&self, px: i64, py: i64, grow: i64) -> f64 {
        if self.radius > 0.0 {
            // 4x4 supersampling for a smooth circle edge.
            let r = self.radius + grow as f64;
            let mut hits = 0;
            for sy in 0..4 {
                for sx in 0..4 {
                    let x = px as f64 + (sx as f64 + 0.5) / 4.0 - self.cx;
                    let y = py as f64 + (sy as f64 + 0.5) / 4.0 - self.cy;
                    if x * x + y * y <= r * r {
                        hits += 1;
                    }
                }
            }
            return hits as f64 / 16.0;
        }
        let (x, y) = (px as f64 + 0.5, py as f64 + 0.5);
        if self.rects.iter().any(|r| r.grow(grow).contains(x, y)) {
            1.0
        } else {
            0.0
        }
    }

    /// Colour of one pixel as straight RGBA (alpha 0..1), or None if transparent.
    pub fn pixel(&self, x: i64, y: i64, cfg: &Config) -> Option<(Color, f64)> {
        let fill = self.coverage(x, y, 0);
        let outer = if self.outline > 0 {
            self.coverage(x, y, self.outline)
        } else {
            0.0
        };
        if fill <= 0.0 && outer <= 0.0 {
            return None;
        }
        // Fill over outline: blend the two by fill coverage.
        let (fc, oc) = (cfg.color, cfg.outline_color);
        let mix = |a: u8, b: u8| (a as f64 * fill + b as f64 * (1.0 - fill)).round() as u8;
        let color = if outer > 0.0 {
            Color {
                r: mix(fc.r, oc.r),
                g: mix(fc.g, oc.g),
                b: mix(fc.b, oc.b),
            }
        } else {
            fc
        };
        let alpha = fill.max(outer) * cfg.opacity;
        (alpha > 0.0).then_some((color, alpha))
    }
}

/// Render the crosshair into `buf` (ARGB8888 little-endian = B,G,R,A bytes, premultiplied).
/// `visible = false` clears the buffer to fully transparent.
pub fn render(buf: &mut [u8], buf_w: u32, buf_h: u32, scale: f64, cfg: &Config, visible: bool) {
    buf.fill(0);
    if !visible {
        return;
    }
    let layout = Layout::new(cfg, buf_w, buf_h, scale);
    let Some(b) = layout.bounds(buf_w, buf_h) else {
        return;
    };
    let stride = buf_w as usize * 4;
    for y in b.y0..b.y1 {
        for x in b.x0..b.x1 {
            if let Some((c, a)) = layout.pixel(x, y, cfg) {
                let i = y as usize * stride + x as usize * 4;
                let pm = |v: u8| (v as f64 * a).round() as u8;
                buf[i] = pm(c.b);
                buf[i + 1] = pm(c.g);
                buf[i + 2] = pm(c.r);
                buf[i + 3] = (a * 255.0).round() as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(shape: Shape) -> Config {
        Config {
            shape,
            ..Config::default()
        }
    }

    /// Centre of mass of all drawn pixels (alpha-weighted), in physical px.
    fn centroid(buf: &[u8], w: u32) -> (f64, f64) {
        let (mut sx, mut sy, mut sa) = (0.0, 0.0, 0.0);
        for (i, px) in buf.as_chunks::<4>().0.iter().enumerate() {
            let a = px[3] as f64;
            if a > 0.0 {
                let (x, y) = ((i as u32 % w) as f64 + 0.5, (i as u32 / w) as f64 + 0.5);
                sx += x * a;
                sy += y * a;
                sa += a;
            }
        }
        (sx / sa, sy / sa)
    }

    fn draw(c: &Config, w: u32, h: u32, scale: f64) -> Vec<u8> {
        let mut buf = vec![0u8; (w * h * 4) as usize];
        render(&mut buf, w, h, scale, c, true);
        buf
    }

    #[test]
    fn every_shape_is_centred_on_even_and_odd_buffers_at_common_scales() {
        for shape in [Shape::Dot, Shape::Cross, Shape::CrossGap] {
            // 1920x1080 at 100 %, 125 % (2400x1350 physical for 1536x864 logical? -> use direct sizes),
            // and odd sizes to hit the rounding paths.
            for (w, h, scale) in [
                (1920, 1080, 1.0),
                (2560, 1440, 1.25),
                (2880, 1620, 1.5),
                (1921, 1081, 1.0),
                (3001, 1687, 1.5),
            ] {
                let buf = draw(&cfg(shape), w, h, scale);
                let (x, y) = centroid(&buf, w);
                assert!(
                    (x - w as f64 / 2.0).abs() <= 0.5,
                    "{shape:?} {w}x{h}@{scale}: x={x}"
                );
                assert!(
                    (y - h as f64 / 2.0).abs() <= 0.5,
                    "{shape:?} {w}x{h}@{scale}: y={y}"
                );
            }
        }
    }

    #[test]
    fn offset_moves_the_centre_by_logical_pixels_times_scale() {
        let c = Config {
            offset_x: 10.0,
            offset_y: -4.0,
            ..cfg(Shape::Cross)
        };
        let buf = draw(&c, 2000, 1000, 1.5);
        let (x, y) = centroid(&buf, 2000);
        assert!((x - (1000.0 + 15.0)).abs() <= 0.5, "x={x}");
        assert!((y - (500.0 - 6.0)).abs() <= 0.5, "y={y}");
    }

    #[test]
    fn size_and_thickness_scale_with_hidpi() {
        let c = Config {
            outline: false,
            ..cfg(Shape::Cross)
        };
        let l1 = Layout::new(&c, 1000, 1000, 1.0);
        let l2 = Layout::new(&c, 1000, 1000, 1.5);
        let width = |l: &Layout| l.rects[0].x1 - l.rects[0].x0;
        let thick = |l: &Layout| l.rects[0].y1 - l.rects[0].y0;
        assert_eq!(width(&l1), 20);
        assert_eq!(
            width(&l2),
            31,
            "30 px bumped to 31 to share the odd 3 px line's centre"
        );
        assert_eq!(thick(&l1), 2);
        assert_eq!(thick(&l2), 3);
    }

    #[test]
    fn cross_gap_leaves_the_middle_empty() {
        let c = Config {
            outline: false,
            ..cfg(Shape::CrossGap)
        };
        let buf = draw(&c, 200, 200, 1.0);
        let at = |x: usize, y: usize| buf[(y * 200 + x) * 4 + 3];
        assert_eq!(at(100, 100), 0, "centre must be empty");
        assert_eq!(at(100 + 8, 100), 255, "right arm drawn");
        assert_eq!(at(100, 100 - 8), 255, "top arm drawn");
    }

    #[test]
    fn outline_surrounds_the_fill_in_the_outline_colour() {
        let c = cfg(Shape::Cross); // green + 1px black outline
        let buf = draw(&c, 200, 200, 1.0);
        let px = |x: usize, y: usize| &buf[(y * 200 + x) * 4..(y * 200 + x) * 4 + 4];
        // Horizontal arm covers rows 99..101 → row 98 and 101 are outline (black, opaque).
        assert_eq!(px(95, 100), &[0x14, 0xff, 0x39, 255]); // B,G,R,A = neon green
        assert_eq!(px(95, 98), &[0, 0, 0, 255]);
        assert_eq!(px(95, 101), &[0, 0, 0, 255]);
        assert_eq!(px(95, 102)[3], 0);
    }

    #[test]
    fn opacity_premultiplies_and_hidden_is_fully_transparent() {
        let c = Config {
            opacity: 0.5,
            outline: false,
            ..cfg(Shape::Cross)
        };
        let buf = draw(&c, 100, 100, 1.0);
        let i = (50 * 100 + 45) * 4;
        assert_eq!(buf[i + 3], 128);
        assert_eq!(buf[i + 1], 128); // 0xff * 0.5
        let mut hidden = vec![7u8; 100 * 100 * 4];
        render(&mut hidden, 100, 100, 1.0, &c, false);
        assert!(hidden.iter().all(|&b| b == 0));
    }

    #[test]
    fn dot_is_round_and_never_touches_far_pixels() {
        let c = Config {
            size: 10.0,
            outline: false,
            ..cfg(Shape::Dot)
        };
        let buf = draw(&c, 100, 100, 1.0);
        let a = |x: usize, y: usize| buf[(y * 100 + x) * 4 + 3];
        assert_eq!(a(50, 50), 255);
        assert_eq!(a(50, 40), 0);
        assert_eq!(
            a(45, 45),
            0,
            "corner of the bounding box stays empty for a circle"
        );
    }

    #[test]
    fn crosshair_partly_off_screen_is_clipped_not_panicking() {
        let c = Config {
            offset_x: 5000.0,
            ..cfg(Shape::Cross)
        };
        let buf = draw(&c, 100, 100, 1.0);
        assert!(buf.iter().all(|&b| b == 0));
    }
}
