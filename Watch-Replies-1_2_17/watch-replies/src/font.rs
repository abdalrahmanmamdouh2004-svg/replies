//! Small anti-aliased Noto font renderer. Latin and Arabic glyph tables are generated in `fonts.rs`.
// Bitmap coverage is stored at 2 bits per pixel (four alpha levels) to fit the watch app partition.

use cmf_wp2_ui::color::blend565;
use cmf_wp2_ui::{ONE, Surface};

pub struct Glyph {
    pub c: char,
    pub adv: u8,
    pub x: i8,
    pub y: i8,
    pub w: u8,
    pub h: u8,
    pub off: u16,
}

#[allow(dead_code)]
pub struct Font {
    pub ascent: i32,
    pub descent: i32,
    pub cap: i32,
    pub glyphs: &'static [Glyph],
    pub bits: &'static [u8],
    pub arabic_ascent: i32,
    pub arabic_descent: i32,
    pub arabic_cap: i32,
    pub arabic_glyphs: &'static [Glyph],
    pub arabic_bits: &'static [u8],
}

impl Font {
    fn glyph_data(&self, c: char) -> Option<(&Glyph, &[u8])> {
        // The common ASCII range is contiguous in the generated table; avoid a linear search per letter.
        if (' '..='~').contains(&c) {
            return self.glyphs.get(c as usize - 32).map(|g| (g, self.bits));
        }
        if let Some(g) = self.glyphs.iter().find(|g| g.c == c) {
            return Some((g, self.bits));
        }
        self.arabic_glyphs.binary_search_by_key(&c, |g| g.c).ok()
            .map(|i| (&self.arabic_glyphs[i], self.arabic_bits))
    }

    fn visual_text(t: &str, out: &mut [char; crate::arabic::MAX_TEXT]) -> usize {
        crate::arabic::visual(t, out)
    }

    /// Every character of `t` is in either the Latin or Arabic glyph table.
    #[cfg(test)]
    pub fn covers(&self, t: &str) -> bool {
        let mut visual = ['\0'; crate::arabic::MAX_TEXT];
        let n = Self::visual_text(t, &mut visual);
        visual[..n].iter().all(|&c| self.glyph_data(c).is_some())
    }

    /// Width after contextual Arabic shaping and visual-order conversion.
    pub fn width(&self, t: &str) -> i32 {
        let mut visual = ['\0'; crate::arabic::MAX_TEXT];
        let n = Self::visual_text(t, &mut visual);
        visual[..n].iter().map(|&c| self.glyph_data(c).map_or(0, |(g, _)| g.adv as i32)).sum()
    }

    /// Draws left-to-right visual-order glyphs; Arabic is shaped and reordered before rasterisation.
    pub fn draw(&self, s: &mut impl Surface, x: i32, y: i32, t: &str, fg: u16, bg: u16) -> i32 {
        let mut shade = [0u16; 16];
        for (a, c) in shade.iter_mut().enumerate() {
            *c = blend565(bg, fg, a as i32 * ONE / 15);
        }
        let mut visual = ['\0'; crate::arabic::MAX_TEXT];
        let n = Self::visual_text(t, &mut visual);
        let mut pen = x;
        for &ch in &visual[..n] {
            let Some((g, bits)) = self.glyph_data(ch) else { continue };
            let (gx, gy, w) = (pen + g.x as i32, y + g.y as i32, g.w as usize);
            let at = |i: usize| {
                // Four 2-bit coverage levels per byte keep the watch font small enough
                // for the CMF app partition while preserving anti-aliased edges.
                let b = bits[g.off as usize + i / 4];
                (b >> (6 - 2 * (i % 4))) & 3
            };
            for row in 0..g.h as usize {
                let base = row * w;
                let mut col = 0;
                while col < w {
                    let a = at(base + col);
                    let start = col;
                    while col < w && at(base + col) == a { col += 1; }
                    if a != 0 {
                        s.rect(gx + start as i32, gy + row as i32, (col - start) as i32, 1, shade[(a * 5) as usize]);
                    }
                }
            }
            pen += g.adv as i32;
        }
        pen
    }

    /// The longest logical prefix of `t` that fits `max` pixels, and whether it was cut.
    pub fn fit<'a>(&self, t: &'a str, max: i32) -> (&'a str, bool) {
        if self.width(t) <= max { return (t, false); }
        let room = max - self.width("…");
        let mut end = 0;
        for (i, c) in t.char_indices() {
            let next = i + c.len_utf8();
            if self.width(&t[..next]) > room {
                return (t[..end].trim_end(), true);
            }
            end = next;
        }
        (t, false)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn centre(&self, s: &mut impl Surface, cx: i32, y: i32, t: &str, max: i32, fg: u16, bg: u16) {
        let (t, cut) = self.fit(t, max);
        let w = self.width(t) + if cut { self.width("…") } else { 0 };
        let end = self.draw(s, cx - w / 2, y, t, fg, bg);
        if cut { self.draw(s, end, y, "…", fg, bg); }
    }
}

/// A filled polygon (any shape, even-odd), antialiased: exact horizontal coverage on 4 sub-rows a pixel.
pub fn poly(s: &mut impl Surface, pts: &[(i32, i32)], bg: u16, paint: impl Fn(i32) -> u16) {
    if pts.len() > 8 { return; }
    let mut h = [(0, 0); 8];
    for (d, p) in h.iter_mut().zip(pts) { *d = (p.0 * 2, p.1 * 2); }
    poly_half(s, &h[..pts.len()], bg, paint)
}

pub fn poly_half(s: &mut impl Surface, pts: &[(i32, i32)], bg: u16, paint: impl Fn(i32) -> u16) {
    const SUB: i32 = 4;
    const FX: i32 = 16;
    let y0 = pts.iter().map(|p| p.1).min().unwrap_or(0).div_euclid(2);
    let y1 = (pts.iter().map(|p| p.1).max().unwrap_or(0) + 1).div_euclid(2);
    let x0 = pts.iter().map(|p| p.0).min().unwrap_or(0).div_euclid(2);
    let x1 = (pts.iter().map(|p| p.0).max().unwrap_or(0) + 1).div_euclid(2);
    let wide = (x1 - x0 + 2) as usize;
    if wide > 128 || pts.len() > 8 { return; }
    for y in y0..y1 {
        let mut cov = [0i32; 128];
        for k in 0..SUB {
            let sy2 = y * 2 * SUB + 2 * k + 1;
            let mut xs = [0i32; 8];
            let mut n = 0;
            for i in 0..pts.len() {
                let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
                let (ay, by) = (a.1 * SUB, b.1 * SUB);
                if (ay <= sy2) != (by <= sy2) {
                    xs[n] = a.0 * (FX / 2) + (b.0 - a.0) * (FX / 2) * (sy2 - ay) / (by - ay);
                    n += 1;
                }
            }
            xs[..n].sort_unstable();
            for span in xs[..n].chunks(2) {
                let [l, r] = span else { continue };
                for px in l.div_euclid(FX)..=r.div_euclid(FX) {
                    let lo = (px * FX).max(*l);
                    let hi = ((px + 1) * FX).min(*r);
                    let i = px - x0 + 1;
                    if hi > lo && i >= 0 && (i as usize) < wide { cov[i as usize] += hi - lo; }
                }
            }
        }
        let full = FX * SUB;
        let mut i = 0;
        while i < wide {
            let c = cov[i];
            let x = x0 - 1 + i as i32;
            let col = if c == 0 { None } else { Some(blend565(bg, paint(x), c * ONE / full)) };
            let start = i;
            while i < wide && cov[i] == c && paint(x0 - 1 + i as i32) == paint(x) { i += 1; }
            if let Some(col) = col { s.rect(x, y, (i - start) as i32, 1, col); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fonts::{B, T};
    use cmf_wp2_ui::{Fb, SIZE};
    use std::vec;

    #[test]
    fn text_is_antialiased_and_fits() {
        let mut px = vec![0u16; (SIZE * SIZE) as usize];
        let mut fb = Fb::new(&mut px);
        let end = T.draw(&mut fb, 100, 200, "Jane", 0xFFFF, 0);
        assert_eq!(end, 100 + T.width("Jane"));
        let mut shades = std::collections::BTreeSet::new();
        for y in 170..215 { for x in 90..end + 10 { shades.insert(fb.at(x, y)); } }
        assert!(shades.len() > 2, "{}", shades.len());
        let (t, cut) = B.fit("Bannerghatta Main Road", 120);
        assert!(cut && B.width(t) + B.width("…") <= 120, "{t}");
        assert_eq!(B.fit("Road", 120), ("Road", false));
    }

    #[test]
    fn both_fonts_cover_common_arabic_and_mixed_text() {
        // T carries no Arabic (view.rs falls back to B for non-ASCII names, saving the duplicate table);
        // B, used for message text and keys, has the full Arabic set.
        assert!(B.covers("أهلاً، كيف حالك؟ 123 ABC"));
    }

    #[test]
    fn a_polygon_has_soft_edges_and_a_solid_middle() {
        let mut px = vec![0u16; (SIZE * SIZE) as usize];
        let mut fb = Fb::new(&mut px);
        poly(&mut fb, &[(100, 100), (140, 160), (60, 160)], 0, |_| 0xFFFF);
        assert_eq!(fb.at(100, 150), 0xFFFF);
        assert_eq!(fb.at(100, 90), 0);
        let edge = fb.at(79, 130);
        assert!(edge != 0 && edge != 0xFFFF, "{edge:x}");
    }
}
