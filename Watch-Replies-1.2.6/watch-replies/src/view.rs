//! Every screen of Watch Replies: the empty screen with the three app icons, the message cards, the keyboard and the
//! short status pages. Everything is drawn with rectangles and `font.rs` text over a black screen.

use crate::font::{Font, poly};
use crate::fonts::{B, T};
use crate::keys::{self, Cell, Key, Page};
use crate::proto::{self, Chat, Model};
use cmf_wp2_ui::color::{BLACK, WHITE};
use cmf_wp2_ui::draw::{disc, line, ring};
use cmf_wp2_ui::{C, Rgb, SIZE, Surface};

const GREY: Rgb = Rgb(150, 150, 160);
const DIM: Rgb = Rgb(84, 86, 96);
const CARD: Rgb = Rgb(38, 46, 66);
const CARD_DOWN: Rgb = Rgb(52, 62, 88);
const KEY_DOWN: Rgb = Rgb(104, 110, 124);
const CHIP: Rgb = Rgb(52, 56, 61);
const KEY_FACE: Rgb = Rgb(52, 56, 61);
const SPACE_FACE: Rgb = Rgb(44, 48, 52);
const KEY_SPECIAL: Rgb = Rgb(52, 56, 61);
const BAR_TRACK: Rgb = Rgb(60, 62, 70);
const BLUE_LIGHT: Rgb = Rgb(138, 180, 248);
const GREEN: Rgb = Rgb(60, 200, 110);
const RED: Rgb = Rgb(235, 80, 70);

// ---- shapes ----

/// A rectangle with round corners of radius `r`, antialiased against black.
fn rounded(s: &mut impl Surface, x: i32, y: i32, w: i32, h: i32, r: i32, c: Rgb) {
    // the four discs must not overlap each other (their soft edges would show inside the shape)
    let r = r.min(h / 4).min(w / 4);
    disc(s, x + r, y + r, r, c, BLACK);
    disc(s, x + w - 1 - r, y + r, r, c, BLACK);
    disc(s, x + r, y + h - 1 - r, r, c, BLACK);
    disc(s, x + w - 1 - r, y + h - 1 - r, r, c, BLACK);
    let c = c.rgb565();
    s.rect(x + r, y, w - 2 * r, h, c);
    s.rect(x, y + r, w, h - 2 * r, c);
}

fn scaled(cx: i32, cy: i32, r: i32, pts: &[(i32, i32)], out: &mut [(i32, i32); 8]) -> usize {
    // points are in 1/100 of the radius
    for (o, p) in out.iter_mut().zip(pts) {
        *o = (cx + (p.0 * r + 50 * p.0.signum()) / 100, cy + (p.1 * r + 50 * p.1.signum()) / 100);
    }
    pts.len()
}

// ---- the three app icons (drawn, not pictures) ----

/// The sign of an app on a disc of radius `r`; `behind` is the colour round it.
pub fn app_icon(s: &mut impl Surface, app: u8, cx: i32, cy: i32, r: i32, behind: Rgb) {
    let white = WHITE.rgb565();
    let mut p = [(0, 0); 8];
    // the real-looking pictures (64 px, on black); the drawn fallback below is for any other app
    if behind == BLACK && crate::icons::draw(s, app, cx, cy) {
        return;
    }
    match app {
        proto::APP_WHATSAPP => {
            let green = Rgb(37, 211, 102);
            disc(s, cx, cy, r, green, behind);
            let g = green.rgb565();
            // a speech bubble: a ring with a tail at the bottom left, and a handset inside
            let n = scaled(cx, cy, r, &[(-50, 30), (-66, 68), (-14, 56)], &mut p);
            poly(s, &p[..n], g, |_| white);
            ring(s, cx, cy - r / 40, r * 5 / 10, r * 65 / 100, WHITE, green);
            let w = (r * 22 / 100).max(3);
            line(s, cx - r * 16 / 100, cy - r * 18 / 100, cx + r * 18 / 100, cy + r * 16 / 100, w, WHITE, green);
        }
        proto::APP_MESSENGER => {
            let blue = Rgb(0, 132, 255);
            disc(s, cx, cy, r, blue, behind);
            let b = blue.rgb565();
            let n = scaled(cx, cy, r, &[(-34, 40), (-50, 70), (0, 50)], &mut p);
            poly(s, &p[..n], white, |_| white);
            disc(s, cx, cy - r / 25, r * 58 / 100, WHITE, blue);
            let n = scaled(cx, cy - r / 25, r, &[(-38, 14), (6, -36), (6, -8), (38, -8), (-6, 36), (-6, 6)], &mut p);
            poly(s, &p[..n], white, |_| b);
        }
        proto::APP_TELEGRAM => {
            let sky = Rgb(42, 171, 238);
            disc(s, cx, cy, r, sky, behind);
            let n = scaled(cx, cy, r, &[(-46, -2), (46, -38), (24, 42), (0, 20), (-16, 34), (-18, 8)], &mut p);
            poly(s, &p[..n], sky.rgb565(), |_| white);
        }
        _ => {
            let grey = Rgb(110, 112, 124);
            disc(s, cx, cy, r, grey, behind);
            disc(s, cx, cy - r / 25, r * 52 / 100, WHITE, grey);
            let n = scaled(cx, cy, r, &[(-30, 36), (-44, 62), (0, 48)], &mut p);
            poly(s, &p[..n], grey.rgb565(), |_| white);
            disc(s, cx, cy - r / 25, r * 38 / 100, Rgb(110, 112, 124), WHITE);
        }
    }
}

// ---- text ----

fn put(s: &mut impl Surface, f: &Font, x: i32, base: i32, t: &str, fg: Rgb, bg: Rgb) -> i32 {
    f.draw(s, x, base, t, fg.rgb565(), bg.rgb565())
}

fn centred(s: &mut impl Surface, f: &Font, cx: i32, base: i32, t: &str, max: i32, fg: Rgb, bg: Rgb) {
    f.centre(s, cx, base, t, max, fg.rgb565(), bg.rgb565());
}

/// `t` cut into a first line that fits `max` px (at a space if it can) and what is left.
pub fn first_line<'a>(f: &Font, t: &'a str, max: i32) -> (&'a str, &'a str) {
    if f.width(t) <= max {
        return (t, "");
    }
    let (mut w, mut end, mut space) = (0, 0, None);
    for (i, c) in t.char_indices() {
        let next = i + c.len_utf8();
        let measured = f.width(&t[..next]);
        if measured > max { break; }
        w = measured;
        end = next;
        if c == ' ' { space = Some(i); }
    }
    let at = space.filter(|&i| i > 0).unwrap_or(end);
    (t[..at].trim_end(), t[at..].trim_start())
}

// ---- the empty screen and the status pages ----

pub fn idle(s: &mut impl Surface, heard: bool) {
    s.rect(0, 0, SIZE, SIZE, 0);
    for (i, app) in [proto::APP_WHATSAPP, proto::APP_MESSENGER, proto::APP_TELEGRAM].into_iter().enumerate() {
        app_icon(s, app, C - 84 + i as i32 * 84, 176, 32, BLACK);
    }
    centred(s, &T, C, 272, "No messages", 340, WHITE, BLACK);
    let sub = if heard { "New ones show up here" } else { "Waiting for the phone…" };
    centred(s, &B, C, 314, sub, 340, GREY, BLACK);
}

/// A page with one line of big text, a smaller line under it and a sign above: `kind` 0 none, 1 tick, 2 cross.
pub fn status(s: &mut impl Surface, kind: u8, title: &str, sub: &str) {
    s.rect(0, 0, SIZE, SIZE, 0);
    match kind {
        1 => {
            disc(s, C, 170, 40, GREEN, BLACK);
            line(s, C - 18, 172, C - 5, 186, 7, WHITE, GREEN);
            line(s, C - 6, 186, C + 20, 156, 7, WHITE, GREEN);
        }
        2 => {
            disc(s, C, 170, 40, RED, BLACK);
            line(s, C - 15, 155, C + 15, 185, 7, WHITE, RED);
            line(s, C + 15, 155, C - 15, 185, 7, WHITE, RED);
        }
        _ => {}
    }
    centred(s, &T, C, 262, title, 320, WHITE, BLACK);
    if !sub.is_empty() {
        centred(s, &B, C, 300, sub, 320, GREY, BLACK);
    }
}

// ---- one message on a page ----

pub const PAGE_X: i32 = 56;
pub const PAGE_Y: i32 = 98;
pub const PAGE_W: i32 = 354;
pub const PAGE_H: i32 = 306;
const BODY_LINES: usize = 5;
const BODY_W: i32 = 296;

/// The last page the list can show.
pub fn max_top(n: usize) -> usize {
    n.saturating_sub(1)
}

/// A touch is on the message's page.
pub fn page_at(x: i32, y: i32) -> bool {
    x >= PAGE_X && x < PAGE_X + PAGE_W && y >= PAGE_Y - 34 && y < PAGE_Y + PAGE_H
}

/// One chat on its own page, like a phone's notification: the app's sign in the middle at the top, then who wrote, when,
/// and the message in big letters. `n` chats in all, this is number `i`; `down`: a finger is on the page.
pub fn page(s: &mut impl Surface, c: &Chat, i: usize, n: usize, down: bool) {
    s.rect(0, 0, SIZE, SIZE, 0);
    let body = if down { CARD_DOWN } else { CARD };
    rounded(s, PAGE_X, PAGE_Y, PAGE_W, PAGE_H, 62, body);
    // the sign sits on the card's top edge, half above it, with a thin black gap round it
    disc(s, C, PAGE_Y, 38, BLACK, BLACK);
    app_icon(s, c.app, C, PAGE_Y, 32, BLACK);
    centred(s, &T, C, PAGE_Y + 76, c.name(), 300, WHITE, body);
    let mut clock = [0u8; 10];
    centred(s, &B, C, PAGE_Y + 108, c.clock(&mut clock), 300, GREY, body);
    // the message, centred, as many lines as fit; the last one ends in "…" if there is more
    let mut rest = c.msg();
    for line in 0..BODY_LINES {
        let base = PAGE_Y + 156 + line as i32 * 31;
        let (l, more) = first_line(&B, rest, BODY_W);
        if line + 1 == BODY_LINES && !more.is_empty() {
            let (cut, _) = B.fit(rest, BODY_W - 20);
            let w = B.width(cut) + B.width("…");
            let x = C - w / 2;
            let end = put(s, &B, x, base, cut, WHITE, body);
            put(s, &B, end, base, "…", WHITE, body);
            break;
        }
        if !l.is_empty() {
            centred(s, &B, C, base, l, BODY_W, WHITE, body);
        }
        rest = more;
        if rest.is_empty() {
            break;
        }
    }
    if n > 1 {
        let gap = 18;
        let y0 = C - (n as i32 - 1) * gap / 2;
        for k in 0..n {
            let on = k == i;
            disc(s, 438, y0 + k as i32 * gap, if on { 5 } else { 3 }, if on { WHITE } else { DIM }, BLACK);
        }
    }
}


// ---- the whole message ----

/// Lines of the whole message that fit between the name and the reply button.
pub const DETAIL_LINES: usize = 6;
const DETAIL_W: i32 = 330;
const DETAIL_BASE: i32 = 192;
const DETAIL_PITCH: i32 = 27;
const REPLY_X: i32 = 128;
const REPLY_Y: i32 = 366;
const REPLY_W: i32 = 210;
const REPLY_H: i32 = 52;

/// A touch is on the reply button.
pub fn reply_at(x: i32, y: i32) -> bool {
    x >= REPLY_X - 10 && x < REPLY_X + REPLY_W + 10 && y >= REPLY_Y - 10 && y < REPLY_Y + REPLY_H + 14
}

/// The message cut into lines of the detail screen: `f(number, line)` for each.
fn detail_each(c: &Chat, mut f: impl FnMut(usize, &str)) -> usize {
    let mut n = 0;
    for (k, msg) in c.log().enumerate() {
        if k > 0 && n < 96 {
            f(n, ""); // a gap between two messages
            n += 1;
        }
        let mut rest = msg;
        while !rest.is_empty() && n < 96 {
            let (l, more) = first_line(&B, rest, DETAIL_W);
            if l.is_empty() {
                break;
            }
            f(n, l);
            n += 1;
            rest = more;
        }
    }
    n
}

/// How far the message can scroll (in lines).
pub fn detail_max(c: &Chat) -> usize {
    detail_each(c, |_, _| {}).saturating_sub(DETAIL_LINES)
}

/// One message in full, like a watch notification: the app's sign, the name, the text (scrolled by `scroll` lines) and a
/// reply button under it; a thin bar on the right shows where you are when the text is longer than the screen.
pub fn detail(s: &mut impl Surface, c: &Chat, scroll: usize, reply_down: bool) {
    s.rect(0, 0, SIZE, SIZE, 0);
    app_icon(s, c.app, C, 66, 32, BLACK);
    centred(s, &T, C, 150, c.name(), 340, Rgb(205, 207, 212), BLACK);
    let total = detail_each(c, |i, l| {
        if i >= scroll && i < scroll + DETAIL_LINES {
            centred(s, &B, C, DETAIL_BASE + (i - scroll) as i32 * DETAIL_PITCH, l, DETAIL_W, Rgb(218, 220, 224), BLACK);
        }
    });
    if total > DETAIL_LINES {
        let (y0, h) = (176, 150);
        s.rect(436, y0, 4, h, BAR_TRACK.rgb565());
        let thumb = (h * DETAIL_LINES as i32 / total as i32).max(14);
        let at = y0 + (h - thumb) * scroll.min(total - DETAIL_LINES) as i32 / (total - DETAIL_LINES) as i32;
        s.rect(436, at, 4, thumb, WHITE.rgb565());
    }
    let face = if reply_down { KEY_DOWN } else { CHIP };
    pill(s, REPLY_X, REPLY_Y, REPLY_W, REPLY_H, face);
    centred(s, &B, REPLY_X + REPLY_W / 2, REPLY_Y + 35, "reply", REPLY_W - 20, WHITE, face);
}

// ---- the keyboard ----

pub const SEND_X: i32 = 340;
pub const BAR_Y: i32 = 115;
pub const BAR_H: i32 = 46;
const CHIPS_FROM: i32 = 126;
const CHIPS_TO: i32 = 334;

/// The send button is on a touch.
pub fn send_at(x: i32, y: i32) -> bool {
    x >= SEND_X - 8 && y >= BAR_Y - 10 && y < BAR_Y + BAR_H + 8
}

/// A capsule: round ends, `h` high, one fill per row (two discs side by side would show their soft edges inside).
fn pill(s: &mut impl Surface, x: i32, y: i32, w: i32, h: i32, c: Rgb) {
    let c = c.rgb565();
    let r2 = h * h / 4;
    for i in 0..h {
        let dy2 = (2 * i + 1 - h) * (2 * i + 1 - h) / 4;
        let mut d = 0;
        while (d + 1) * (d + 1) + dy2 <= r2 {
            d += 1;
        }
        let inset = h / 2 - d;
        s.rect(x + inset, y + i, w - 2 * inset, 1, c);
    }
}

/// The quick replies' left edges and widths, from the left; the ones that do not fit are left out.
pub fn chips(items: &[&str]) -> ([(i32, i32); proto::QUICK_MAX], usize) {
    let mut w = [(0, 0); proto::QUICK_MAX];
    let mut n = 0;
    let mut x = CHIPS_FROM;
    for t in items {
        let cw = (B.width(t) + 22).min(200);
        if x + cw > CHIPS_TO {
            break;
        }
        w[n] = (x, cw);
        x += cw + 6;
        n += 1;
    }
    (w, n)
}

/// The quick reply under a touch.
pub fn chip_at(items: &[&str], x: i32, y: i32) -> Option<usize> {
    let (w, n) = chips(items);
    (0..n).find(|&i| x >= w[i].0 - 4 && x < w[i].0 + w[i].1 + 4 && y >= BAR_Y - 8 && y < BAR_Y + BAR_H + 8)
}

/// The text so far, as in a messaging app: a line at the top, left aligned, its end in view.
pub fn field(s: &mut impl Surface, to: &str, text: &[u8]) {
    let (x, base) = (98, 100);
    let t = core::str::from_utf8(text).unwrap_or("");
    if t.is_empty() {
        s.rect(x - 5, 76, 3, 28, BLUE_LIGHT.rgb565());
        let mut head = [0u8; 48];
        let pre = b"Reply to ";
        head[..pre.len()].copy_from_slice(pre);
        let name = to.as_bytes();
        let n = name.len().min(head.len() - pre.len());
        let n = (0..=n).rev().find(|&i| to.is_char_boundary(i)).unwrap_or(0);
        head[pre.len()..pre.len() + n].copy_from_slice(&name[..n]);
        let label = core::str::from_utf8(&head[..pre.len() + n]).unwrap_or("");
        let (label, cut) = B.fit(label, 260);
        let end = put(s, &B, x, base, label, DIM, BLACK);
        if cut { put(s, &B, end, base, "…", DIM, BLACK); }
    } else if crate::arabic::contains_rtl(t) {
        // Arabic reads from right to left; keep the newest tail visible and put the caret at its left edge.
        let mut at = 0;
        if B.width(t) > 262 {
            for (i, ch) in t.char_indices() {
                let next = i + ch.len_utf8();
                if B.width(&t[next..]) <= 262 { at = i; break; }
            }
        }
        let shown = &t[at..];
        let width = B.width(shown).min(262);
        let left = x + 262 - width;
        put(s, &B, left, base, shown, BLUE_LIGHT, BLACK);
        s.rect((left - 5).max(x - 5), 76, 3, 28, BLUE_LIGHT.rgb565());
    } else {
        // Keep a complete UTF-8 suffix in view; never start in the middle of a character.
        let mut at = 0;
        if B.width(t) > 262 {
            for (i, ch) in t.char_indices() {
                if B.width(&t[i..]) <= 262 { at = i; break; }
            }
        }
        let shown = &t[at..];
        let split = shown.rfind(' ').map_or(0, |i| i + 1);
        let end = put(s, &B, x, base, &shown[..split], WHITE, BLACK);
        let end = put(s, &B, end, base, &shown[split..], BLUE_LIGHT, BLACK);
        s.rect(end + 2, 76, 3, 28, BLUE_LIGHT.rgb565());
    }
}

/// The quick replies (while there is no text yet) and the send button.
pub fn bar(s: &mut impl Surface, m: &Model, text: &[u8]) {
    s.rect(60, BAR_Y - 2, SIZE - 120, BAR_H + 4, 0);
    // the ">" tile that opens the suggestions in the Wear OS keyboard
    tile(s, 74, BAR_Y, 46, BAR_H, CHIP);
    let mut p = [(0, 0); 8];
    let n = scaled(97, BAR_Y + BAR_H / 2, 30, &[(-30, -50), (30, 0), (-30, 50), (-30, 34), (6, 0), (-30, -34)], &mut p);
    poly(s, &p[..n], CHIP.rgb565(), |_| WHITE.rgb565());
    {
        let (items, k) = m.items(text);
        let (w, n) = chips(&items[..k]);
        for (i, &(x, cw)) in w[..n].iter().enumerate() {
            tile(s, x, BAR_Y, cw, BAR_H, CHIP);
            centred(s, &B, x + cw / 2, BAR_Y + 31, items[i], cw - 16, WHITE, CHIP);
        }
    }
    // the send button: a light-blue capsule with an outlined arrow
    let on = !text.is_empty();
    let face = if on { BLUE_LIGHT } else { CHIP };
    pill(s, SEND_X, BAR_Y, 63, BAR_H, face);
    let ink = if on { Rgb(28, 40, 76) } else { DIM };
    let (cx, cy) = (SEND_X + 31, BAR_Y + BAR_H / 2);
    // an outlined paper plane: four thin lines
    let (a, b, c, d) = ((cx - 11, cy - 12), (cx + 13, cy), (cx - 11, cy + 12), (cx - 5, cy));
    for (p, q) in [(a, b), (b, c), (c, d), (d, a)] {
        line(s, p.0, p.1, q.0, q.1, 2, ink, face);
    }
}

/// A thin closed outline through `pts`.
fn outline(s: &mut impl Surface, pts: &[(i32, i32)], ink: Rgb, bg: Rgb) {
    for i in 0..pts.len() {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        line(s, a.0, a.1, b.0, b.1, 2, ink, bg);
    }
}

/// A rounded tile (radius about 12): thirteen fills, cheap to draw.
fn tile(s: &mut impl Surface, x: i32, y: i32, w: i32, h: i32, c: Rgb) {
    tile_with(s, x, y, w, h, c, &[9, 5, 3, 2, 1, 1]);
}

/// A tile whose corners are cut by `steps` (how far in each row from the edge starts).
fn tile_with(s: &mut impl Surface, x: i32, y: i32, w: i32, h: i32, c: Rgb, steps: &[i32]) {
    let c = c.rgb565();
    for (i, &inset) in steps.iter().enumerate() {
        s.rect(x + inset, y + i as i32, w - 2 * inset, 1, c);
        s.rect(x + inset, y + h - 1 - i as i32, w - 2 * inset, 1, c);
    }
    let n = steps.len() as i32;
    s.rect(x, y + n, w, h - 2 * n, c);
}

/// One key: a dark rounded tile (like a phone keyboard on a watch), a small gap round it. `down`: a finger is on it;
/// `shift`: the letters are capitals.
pub fn key(s: &mut impl Surface, c: &Cell, page: Page, shift: bool, down: bool) {
    s.rect(c.x, c.y, c.w, c.h, 0);
    let (cx, cy) = (c.x + c.w / 2, c.y + c.h / 2);
    let special = !matches!(c.key, Key::Ch(_));
    let under = if down { KEY_DOWN } else if c.key == Key::Space { SPACE_FACE } else if special { KEY_SPECIAL } else { KEY_FACE };
    let (ix, iy) = if matches!(c.key, Key::Space | Key::Page) || c.w > 60 { (4, 5) } else { (3, 5) };
    if c.key == Key::Space {
        // a big soft bar reaching down along the edge of the screen
        tile_with(s, c.x + ix, c.y + iy, c.w - 2 * ix, c.h - iy - 2, under, &[14, 9, 6, 4, 3, 2, 2, 1, 1, 1]);
    } else {
        tile(s, c.x + ix, c.y + iy, c.w - 2 * ix, c.h - 2 * iy, under);
    }
    match c.key {
        Key::Ch(code) => {
            // Internal Arabic key codes are converted to real Unicode for their key labels too.
            let code = if page == Page::Letters && shift { code.to_ascii_uppercase() } else { code };
            let ch = keys::character(code);
            let mut buf = [0u8; 4];
            let t = ch.encode_utf8(&mut buf);
            put(s, &B, cx - B.width(t) / 2, cy + B.cap / 2, t, Rgb(232, 234, 237), under);
        }
        Key::Shift => {
            // an outlined up arrow
            let ink = if shift { BLUE_LIGHT } else { Rgb(232, 234, 237) };
            let mut p = [(0, 0); 8];
            let n = scaled(cx, cy, 34, &[(0, -34), (34, 2), (15, 2), (15, 34), (-15, 34), (-15, 2), (-34, 2)], &mut p);
            outline(s, &p[..n], ink, under);
        }
        Key::Back => {
            // an outlined delete sign (a tag pointing left) with a cross in it
            let ink = Rgb(232, 234, 237);
            let mut p = [(0, 0); 8];
            let n = scaled(cx, cy, 36, &[(-40, 0), (-14, -32), (40, -32), (40, 32), (-14, 32)], &mut p);
            outline(s, &p[..n], ink, under);
            let x = cx + 5;
            line(s, x - 4, cy - 4, x + 4, cy + 4, 2, ink, under);
            line(s, x + 4, cy - 4, x - 4, cy + 4, 2, ink, under);
        }
        Key::Space => {
            let t = if page.is_arabic() { "EN" } else { "ع" };
            put(s, &B, cx - B.width(t) / 2, cy + B.cap / 2, t, WHITE, under);
        }
        Key::Page => {
            let t = match page {
                Page::Letters => "?123",
                Page::Numbers => "abc",
                Page::ArabicLetters => "١٢٣",
                Page::ArabicNumbers => "أبج",
            };
            put(s, &B, cx - B.width(t) / 2, cy + B.cap / 2, t, WHITE, under);
        }
    }
}

/// Every key of a page (a new page, or the letters changed case).
pub fn all_keys(s: &mut impl Surface, page: Page, shift: bool) {
    let (cells, n) = keys::layout(page);
    for c in &cells[..n] {
        key(s, c, page, shift, false);
    }
}

/// The keyboard: the text line, the quick replies and send button, and the keys over the rest of the screen.
pub fn keyboard(s: &mut impl Surface, m: &Model, to: &str, text: &[u8], page: Page, shift: bool) {
    s.rect(0, 0, SIZE, SIZE, 0);
    field(s, to, text);
    bar(s, m, text);
    all_keys(s, page, shift);
}

/// Only the top part (the text changed).
pub fn text_changed(s: &mut impl Surface, m: &Model, to: &str, text: &[u8]) {
    s.rect(40, 30, SIZE - 80, keys::TOP - 34, 0);
    field(s, to, text);
    bar(s, m, text);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::layout;
    use cmf_wp2_ui::Fb;
    use std::vec;

    fn model() -> Model {
        let mut m = Model::new();
        for (i, (app, name, msg)) in [
            (1u8, "Jane Appleseed", "I'm on my way! Can you pick up some milk on the way home, please? Thanks a lot, see you very soon at the house."),
            (2, "Bob", "ok"),
            (3, "A very long contact name", "Hello there"),
            (0, "Mum", "Call me when you can, it is not urgent but I would like to hear from you soon"),
        ]
        .into_iter()
        .enumerate()
        {
            let mut v = vec![b'r', b'1', b'L', i as u8, app, 9, 5 + i as u8, name.len() as u8];
            v.extend_from_slice(name.as_bytes());
            v.extend_from_slice(msg.as_bytes());
            m.apply(&v);
        }
        m.apply(b"r1Q\x03\x02Ok\x03Yes\x07Thanks!");
        m
    }

    fn lit_outside(fb: &Fb) -> Option<(i32, i32)> {
        for y in 0..SIZE {
            for x in 0..SIZE {
                let (dx, dy) = (x - C, y - C);
                if dx * dx + dy * dy > (SIZE / 2) * (SIZE / 2) && fb.at(x, y) != 0 {
                    return Some((x, y));
                }
            }
        }
        None
    }

    fn lit(fb: &Fb, x0: i32, y0: i32, x1: i32, y1: i32) -> usize {
        let mut n = 0;
        for y in y0..y1 {
            for x in x0..x1 {
                if fb.at(x, y) != 0 {
                    n += 1;
                }
            }
        }
        n
    }

    /// With WP2_SHOTS=dir set, the tests leave pictures there.
    fn shot(name: &str, fb: &Fb) {
        let Ok(dir) = std::env::var("WP2_SHOTS") else { return };
        let mut out = std::format!("P6\n{SIZE} {SIZE}\n255\n").into_bytes();
        for y in 0..SIZE {
            for x in 0..SIZE {
                let c = fb.at(x, y);
                let (r, g, b) = (((c >> 11) & 31) as u32, ((c >> 5) & 63) as u32, (c & 31) as u32);
                out.extend_from_slice(&[(r * 255 / 31) as u8, (g * 255 / 63) as u8, (b * 255 / 31) as u8]);
            }
        }
        std::fs::write(std::format!("{dir}/{name}.ppm"), out).unwrap();
    }

    #[test]
    fn fonts_have_what_the_screens_need() {
        assert!(T.covers("Jane Appleseed qwertyuiop QWERTY 1234567890 .,?!'\"-/:;()$&@#+ No messages"));
        assert!(B.covers("Reply to Type a message 12:05 a.m. p.m. Waiting for the phone… 123 abc ·"));
        assert!(T.covers("مرحبا"));
        assert!(B.covers("أهلاً، كيف حالك؟ 123"));
    }

    #[test]
    fn nothing_is_drawn_outside_the_round_screen() {
        let m = model();
        let mut px = vec![0u16; (SIZE * SIZE) as usize];
        let mut fb = Fb::new(&mut px);
        idle(&mut fb, true);
        assert_eq!(lit_outside(&fb), None, "idle");
        shot("idle", &fb);
        for i in 0..m.n {
            page(&mut fb, &m.chats[i], i, m.n, i == 1);
            assert_eq!(lit_outside(&fb), None, "page {i}");
            shot(&std::format!("page{i}"), &fb);
        }
        page(&mut fb, &m.chats[0], 0, 1, false);
        assert_eq!(lit_outside(&fb), None, "single");
        for (p, shift) in [(Page::Letters, false), (Page::Letters, true), (Page::Numbers, false)] {
            keyboard(&mut fb, &m, "Jane Appleseed", b"", p, shift);
            assert_eq!(lit_outside(&fb), None, "{p:?}");
            shot(&std::format!("kb_{p:?}_{shift}"), &fb);
        }
        keyboard(&mut fb, &m, "A very long contact name", b"Hello how are you doing today my friend?", Page::Letters, false);
        assert_eq!(lit_outside(&fb), None, "typed");
        shot("typed", &fb);
        keyboard(&mut fb, &m, "أحمد", "مرحبا".as_bytes(), Page::ArabicLetters, false);
        assert_eq!(lit_outside(&fb), None, "Arabic keyboard");
        shot("typed_arabic", &fb);
        for (k, t, sub) in [(0, "Sending…", ""), (1, "Sent", "Jane"), (2, "Not sent", "Is Watch Menu open?")] {
            status(&mut fb, k, t, sub);
            assert_eq!(lit_outside(&fb), None, "status {k}");
            shot(&std::format!("status{k}"), &fb);
        }
    }

    #[test]
    fn a_page_keeps_to_its_card_and_the_message_fits() {
        let m = model();
        let mut px = vec![0u16; (SIZE * SIZE) as usize];
        let mut fb = Fb::new(&mut px);
        page(&mut fb, &m.chats[3], 3, m.n, false); // chat 3 is the long first message
        assert_eq!(lit(&fb, 0, 0, SIZE, PAGE_Y - 34), 0);
        assert_eq!(lit(&fb, 0, 0, PAGE_X, SIZE), 0);
        assert_eq!(lit(&fb, 0, PAGE_Y + PAGE_H, SIZE, SIZE), 0);
        // five lines at most
        let mut rest = m.chats[3].msg();
        for _ in 0..BODY_LINES {
            let (l, more) = first_line(&B, rest, BODY_W);
            assert!(B.width(l) <= BODY_W);
            rest = more;
        }
        let _ = rest;
    }

    #[test]
    fn the_whole_message_scrolls_and_stays_on_the_screen() {
        let m = model();
        let mut px = vec![0u16; (SIZE * SIZE) as usize];
        let mut fb = Fb::new(&mut px);
        for i in 0..m.n {
            let max = detail_max(&m.chats[i]);
            for sc in [0, max] {
                detail(&mut fb, &m.chats[i], sc, false);
                assert_eq!(lit_outside(&fb), None, "chat {i} scroll {sc}");
            }
        }
        detail(&mut fb, &m.chats[0], 0, false);
        shot("detail", &fb);
        detail(&mut fb, &m.chats[0], detail_max(&m.chats[0]), true);
        shot("detail_end", &fb);
        assert_eq!(detail_max(&m.chats[1]), 0);
        // the longest the phone can send (96 bytes) of the widest letters runs past six lines and scrolls
        let mut wide = Model::new();
        let mut v = vec![b'r', b'1', b'L', 9, 1, 9, 5, 3];
        v.extend_from_slice(b"Ann");
        v.extend_from_slice(&[b'W'; 96]);
        wide.apply(&v);
        let max = detail_max(&wide.chats[0]);
        assert!(max > 0, "{max}");
        for sc in [0, 1, max] {
            detail(&mut fb, &wide.chats[0], sc, false);
            assert_eq!(lit_outside(&fb), None, "wide {sc}");
        }
        shot("detail_scroll", &fb);
        assert!(reply_at(233, 390) && !reply_at(233, 200) && !reply_at(40, 390));
    }

    #[test]
    fn lines_break_at_spaces() {
        assert_eq!(first_line(&B, "hello", 500), ("hello", ""));
        let (a, b) = first_line(&B, "one two three four five six seven", 100);
        assert!(B.width(a) <= 100 && a.ends_with(|c: char| c.is_alphabetic()) && !a.contains("  "), "{a}");
        assert!(!b.is_empty() && !b.starts_with(' '));
        let (a, b) = first_line(&B, "abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz", 100);
        assert!(B.width(a) <= 100 && !a.is_empty() && !b.is_empty());
    }

    #[test]
    fn touches_find_the_page_chips_and_the_send_button() {
        let m = model();
        assert!(page_at(C, C) && !page_at(10, C) && !page_at(C, 20));
        let (items, k) = m.items(b"");
        let (w, n) = chips(&items[..k]);
        assert!(n >= 2, "{n}");
        assert_eq!(chip_at(&items[..k], w[1].0 + 5, BAR_Y + 10), Some(1));
        assert_eq!(chip_at(&items[..k], 5, BAR_Y + 10), None);
        assert!(send_at(SEND_X + 30, BAR_Y + 20) && !send_at(C, BAR_Y + 20));
        for i in 1..n {
            assert!(w[i].0 >= w[i - 1].0 + w[i - 1].1, "chips overlap");
        }
        assert!(w[n - 1].0 + w[n - 1].1 <= SEND_X, "chips run into the send button");
        assert_eq!((max_top(0), max_top(1), max_top(4)), (0, 0, 3));
    }

    #[test]
    fn a_key_press_changes_only_that_key() {
        let m = model();
        let mut px = vec![0u16; (SIZE * SIZE) as usize];
        let mut fb = Fb::new(&mut px);
        keyboard(&mut fb, &m, "Jane", b"", Page::Letters, false);
        let (cells, n) = layout(Page::Letters);
        for k in [Key::Ch(b'g'), Key::Space, Key::Back, Key::Shift] {
            let g = cells[..n].iter().find(|c| c.key == k).unwrap();
            let before: std::vec::Vec<u16> = (0..SIZE * SIZE).map(|i| fb.at(i % SIZE, i / SIZE)).collect();
            key(&mut fb, g, Page::Letters, false, true);
            let mut changed = 0;
            for i in 0..SIZE * SIZE {
                let (x, y) = (i % SIZE, i / SIZE);
                if fb.at(x, y) != before[i as usize] {
                    changed += 1;
                    assert!(x >= g.x && x < g.x + g.w && y >= g.y && y < g.y + g.h, "{k:?} {x},{y}");
                }
            }
            assert!(changed > 100, "{k:?} {changed}");
            key(&mut fb, g, Page::Letters, false, false);
            let after: std::vec::Vec<u16> = (0..SIZE * SIZE).map(|i| fb.at(i % SIZE, i / SIZE)).collect();
            assert!(before == after, "{k:?} is not drawn back as it was");
        }
    }

    #[test]
    fn typing_updates_the_top_and_nothing_below_it() {
        let m = model();
        let mut px = vec![0u16; (SIZE * SIZE) as usize];
        let mut fb = Fb::new(&mut px);
        keyboard(&mut fb, &m, "Jane", b"", Page::Letters, false);
        let before: std::vec::Vec<u16> = (0..SIZE * SIZE).map(|i| fb.at(i % SIZE, i / SIZE)).collect();
        text_changed(&mut fb, &m, "Jane", b"Hi");
        for i in 0..SIZE * SIZE {
            let (x, y) = (i % SIZE, i / SIZE);
            if fb.at(x, y) != before[i as usize] {
                assert!(y < keys::TOP, "{x},{y}");
            }
        }
        // "Hi" offers words like "hill"... or nothing; either way the chips are what `items` says
        let (items, k) = m.items(b"Hi");
        assert!(chips(&items[..k]).1 <= k);
        // words that continue "Hi" are drawn in the row; after "Hi " (a space) there are none
        assert!(lit(&fb, CHIPS_FROM, BAR_Y, CHIPS_TO, BAR_Y + BAR_H) > 0);
        text_changed(&mut fb, &m, "Jane", b"Hi ");
        assert_eq!(lit(&fb, CHIPS_FROM, BAR_Y, CHIPS_TO, BAR_Y + BAR_H), 0);
    }
}
