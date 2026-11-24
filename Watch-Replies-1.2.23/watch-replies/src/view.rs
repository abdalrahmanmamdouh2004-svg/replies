//! Every screen of Watch Replies: the empty screen with the three app icons, the message cards, the keyboard and the
//! short status pages. Everything is drawn with rectangles and `font.rs` text over a black screen.

use crate::font::{Font, poly};
use crate::fonts::{B, T};
use crate::fonts_small::N;
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
pub const PAGE_H: i32 = 276;
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
    page_card(s, c, down);
    page_dots(s, i, n);
}

fn page_card(s: &mut impl Surface, c: &Chat, down: bool) {
    let body = if down { CARD_DOWN } else { CARD };
    rounded(s, PAGE_X, PAGE_Y, PAGE_W, PAGE_H, 62, body);
    // the sign sits on the card's top edge, half above it, with a thin black gap round it
    if !crate::icons::cutout(s, c.app, C, PAGE_Y, 5) {
        disc(s, C, PAGE_Y, 38, BLACK, BLACK);
    }
    app_icon(s, c.app, C, PAGE_Y, 32, BLACK);
    centred(s, if c.name().is_ascii() { &T } else { &B }, C, PAGE_Y + 76, c.name(), 300, WHITE, body);
    let mut clock = [0u8; 10];
    centred(s, &B, C, PAGE_Y + 108, c.clock(&mut clock), 300, GREY, body);
    // the message, centred, as many lines as fit; the last one ends in "…" if there is more
    let mut rest = c.msg();
    for line in 0..BODY_LINES {
        let base = PAGE_Y + 146 + line as i32 * 29;
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
}

fn page_dots(s: &mut impl Surface, i: usize, n: usize) {
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

/// One step of the conversation's scrolling, in pixels.
/// The part of the screen the bubbles live in.
const VIEW_TOP: i32 = 78;
const VIEW_BOTTOM: i32 = 354;
const TEXT_W: i32 = 284;
const BUBBLE_LEFT: i32 = 22;
const BUBBLE_RIGHT: i32 = 390;
/// Room between a bubble's edge and its text, sideways and above and below.
const PADX: i32 = 17;
const PADY: i32 = 11;
const NAME_H: i32 = 25;
/// How far the tail of a bubble reaches out past its side, from the bottom row up.
const TAIL: [i32; 10] = [14, 11, 8, 6, 4, 3, 2, 1, 1, 0];

/// A bubble with a pointed tail at its bottom corner (the left one for `left`, else the right one), like a messages app.
fn bubble(s: &mut impl Surface, x: i32, y: i32, w: i32, h: i32, c: Rgb, left: bool, has_tail: bool) {
    let c = c.rgb565();
    let n = ROUND.len() as i32;
    let row = |s: &mut dyn FnMut(i32, i32, i32, i32), r: i32| {
        let from_bottom = h - 1 - r;
        let (mut l, mut rt) = if r < n { (ROUND[r as usize], ROUND[r as usize]) } else if from_bottom < n { (ROUND[from_bottom as usize], ROUND[from_bottom as usize]) } else { (0, 0) };
        if has_tail && from_bottom < n {
            let out = TAIL.get(from_bottom as usize).copied().unwrap_or(0);
            if left { l = -out } else { rt = -out }
        }
        s(x + l, y + r, w - l - rt, 1);
    };
    let mut fill = |x: i32, y: i32, w: i32, h: i32| s.rect(x, y, w, h, c);
    for r in 0..n {
        row(&mut fill, r);
    }
    fill(x, y + n, w, h - 2 * n);
    for r in h - n..h {
        row(&mut fill, r);
    }
}

/// The corner of a bubble: how far in each row from the edge starts.
const ROUND: [i32; 15] = [15, 12, 10, 8, 7, 6, 5, 4, 3, 3, 2, 2, 1, 1, 1];

/// The font and the line height of a bubble's text: the big English font when the text is plain English, else the one that
/// also has the Arabic letters.
fn text_font(_t: &str) -> (&'static Font, i32) {
    // B contains both Latin and Arabic at 27 px. Scale it to 32 px so messages containing
    // Arabic/Unicode are not visibly smaller than ASCII-only messages.
    (&B, 35)
}

const BODY_SCALE_NUM: i32 = 32;
const BODY_SCALE_DEN: i32 = 27;

fn first_line_body<'a>(f: &Font, t: &'a str, max: i32) -> (&'a str, &'a str) {
    if f.width_scaled(t, BODY_SCALE_NUM, BODY_SCALE_DEN) <= max { return (t, ""); }
    let (mut end, mut space) = (0, None);
    for (i, c) in t.char_indices() {
        let next = i + c.len_utf8();
        if f.width_scaled(&t[..next], BODY_SCALE_NUM, BODY_SCALE_DEN) > max { break; }
        end = next;
        if c == ' ' { space = Some(i); }
    }
    let at = space.filter(|&i| i > 0).unwrap_or(end);
    (t[..at].trim_end(), t[at..].trim_start())
}

/// The font of a name: the small one, unless the name has letters only the big one has.
fn name_font(n: &str) -> &'static Font {
    if n.is_ascii() { &N } else { &B }
}
const REPLY_X: i32 = 108;
const REPLY_Y: i32 = 364;
const REPLY_W: i32 = 250;
const REPLY_H: i32 = 58;

/// A touch is on the message box.
pub fn reply_at(x: i32, y: i32) -> bool {
    x >= REPLY_X - 10 && x < REPLY_X + REPLY_W + 10 && y >= REPLY_Y - 10 && y < REPLY_Y + REPLY_H + 14
}

/// How far a thing at height `y` is pulled in from the side to follow the round screen (0 in the middle).
pub fn sag(y: i32) -> i32 {
    (C - circle_half_width(y.clamp(0, SIZE - 1))).max(0)
}

/// Draws only inside the rows `from..to` (a bubble can be half scrolled out of the conversation).
struct Clip<'a, S: Surface> {
    s: &'a mut S,
    from: i32,
    to: i32,
}

// The watch has a round 480 px display. Clip bubble and glyph rectangles to both
// the conversation viewport and the circular panel, not just the rectangular framebuffer.
const fn half_table() -> [i16; SIZE as usize] {
    let mut t = [-1i16; SIZE as usize];
    let radius = SIZE / 2;
    let mut y = 0;
    while y < SIZE {
        let dy = y - C;
        let remaining = radius * radius - dy * dy;
        if remaining >= 0 {
            let (mut low, mut high) = (0, radius);
            while low < high {
                let mid = (low + high + 1) / 2;
                if mid * mid <= remaining { low = mid } else { high = mid - 1 }
            }
            t[y as usize] = low as i16;
        }
        y += 1;
    }
    t
}
/// Worked out when the app is built, so drawing does not search for square roots row by row.
static HALF: [i16; SIZE as usize] = half_table();

fn circle_half_width(y: i32) -> i32 {
    if (0..SIZE).contains(&y) { HALF[y as usize] as i32 } else { -1 }
}

impl<S: Surface> Surface for Clip<'_, S> {
    fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: u16) {
        if w <= 0 || h <= 0 {
            return;
        }
        let y0 = y.max(self.from).max(0);
        let y1 = (y + h).min(self.to).min(SIZE);
        let requested_left = x.max(0);
        let requested_right = (x + w).min(SIZE);
        if y1 <= y0 || requested_right <= requested_left {
            return;
        }
        for row in y0..y1 {
            let half = circle_half_width(row);
            if half < 0 {
                continue;
            }
            let left = requested_left.max(C - half);
            let right = requested_right.min(C + half + 1);
            if right > left {
                self.s.rect(left, row, right - left, 1, c);
            }
        }
    }
    fn text(&mut self, _: i32, _: i32, _: &str, _: u16) {}
}

/// Lines and widest line of a bubble's text.
fn measure(t: &str) -> (i32, i32) {
    let (mut rest, mut lines, mut w) = (t, 0, 0);
    let font = text_font(t).0;
    while !rest.is_empty() && lines < 12 {
        let (l, more) = first_line_body(font, rest, TEXT_W);
        if l.is_empty() {
            break;
        }
        w = w.max(font.width_scaled(l, BODY_SCALE_NUM, BODY_SCALE_DEN));
        lines += 1;
        rest = more;
    }
    (lines.max(1), w)
}

/// The bubbles, top to bottom: `f(top, mine, with_name, width, height, text)` for each; returns the height of them all.
/// A reply of mine is on the right; the first bubble of a run from the other side carries the name.
fn bubbles(c: &Chat, mut f: impl FnMut(i32, bool, bool, bool, usize, i32, i32, &str)) -> i32 {
    let (mut y, mut prev, mut incoming_index) = (0, None, 0usize);
    for (mine, msg) in c.entries() {
        let group_start = prev != Some(mine);
        let named = !mine && group_start;
        let (lines, mut w) = measure(msg);
        if named {
            w = w.max(name_font(c.name()).width(c.name()).min(TEXT_W));
        }
        let h = 2 * PADY + lines * text_font(msg).1 + if named { NAME_H } else { 0 };
        y += match prev {
            None => 0,
            Some(p) if p == mine => 5,
            _ => 11,
        };
        f(y, mine, named, group_start, incoming_index, w + 2 * PADX, h, msg);
        if !mine { incoming_index += 1; }
        y += h;
        prev = Some(mine);
    }
    y
}

/// How far the conversation can scroll (in steps).
pub fn detail_max(c: &Chat) -> usize {
    let over = bubbles(c, |_, _, _, _, _, _, _, _| {}) + 8 - (VIEW_BOTTOM - VIEW_TOP);
    over.max(0) as usize
}

fn lighter(c: Rgb) -> Rgb {
    Rgb((c.0 as u32 * 2 / 5 + 153) as u8, (c.1 as u32 * 2 / 5 + 153) as u8, (c.2 as u32 * 2 / 5 + 153) as u8)
}

/// Is the app one of the three that have a colour of their own.
fn known(app: u8) -> bool {
    matches!(app, proto::APP_WHATSAPP | proto::APP_MESSENGER | proto::APP_TELEGRAM)
}

/// The colour of the other side's bubbles: the app's own.
fn bubble_colour(app: u8) -> Rgb {
    match app {
        proto::APP_WHATSAPP => Rgb(24, 140, 76),
        proto::APP_MESSENGER => Rgb(20, 98, 232),
        proto::APP_TELEGRAM => Rgb(52, 160, 222),
        _ => Rgb(206, 208, 214),
    }
}

/// The conversation of one chat like a messages app on a watch: their bubbles (green, blue, light blue or dark grey by
/// app) on the left with the name, my replies in light grey on the right, and a box that says "Message" under them; the
/// bubbles scroll by `scroll` steps. A thin bar on the right shows where you are when there is more than fits.
pub fn detail(s: &mut impl Surface, c: &Chat, scroll_px: usize, reply_down: bool) {
    s.rect(0, 0, SIZE, SIZE, 0);
    let theirs = bubble_colour(c.app);
    let mine = Rgb(80, 82, 90);
    let off = VIEW_TOP + 4 - scroll_px as i32;
    // the bubbles that lie under the sign at the top, to blend the edge of its black rim into
    let mut under = [(0, 0, 0, 0, 0u16); 4];
    let mut under_n = 0;
    let total = {
        let mut clip = Clip { s: &mut *s, from: 0, to: SIZE };
        bubbles(c, |top, is_mine, named, has_tail, incoming_index, w, h, msg| {
            let y = off + top;
            if y + h <= 0 || y >= SIZE {
                return;
            }
            // Alternate incoming bubbles between a centered lane and a much farther-left lane.
            // curved list: bubbles stick to the round wall — theirs to the left one, mine to the right one — and follow
            // it as they scroll (the wall is as far in as its tightest row inside the conversation)
            let wall = circle_half_width(y.clamp(VIEW_TOP, VIEW_BOTTOM - 1)).min(circle_half_width((y + h).clamp(VIEW_TOP, VIEW_BOTTOM - 1)));
            let _ = incoming_index;
            let x = if is_mine { C + wall - w - 18 } else { C - wall + 20 };
            let (face, ink) = if is_mine { (mine, WHITE) } else if known(c.app) { (theirs, WHITE) } else { (theirs, Rgb(24, 24, 28)) };
            if y < 70 && y + h > 14 && x < C + 32 && x + w > C - 32 && under_n < under.len() {
                under[under_n] = (x, y, w, h, face.rgb565());
                under_n += 1;
            }
            bubble(&mut clip, x, y, w, h, face, !is_mine, has_tail);
            let (font, pitch) = text_font(msg);
            let scaled_cap = (font.cap * BODY_SCALE_NUM + BODY_SCALE_DEN / 2) / BODY_SCALE_DEN;
            let mut base = y + PADY + scaled_cap + 1;
            if named {
                let nf = name_font(c.name());
                let (n, _) = nf.fit(c.name(), TEXT_W);
                let tint = if known(c.app) { lighter(face) } else { Rgb(96, 98, 108) };
                nf.draw(&mut clip, x + PADX, y + PADY + nf.cap + 1, n, tint.rgb565(), face.rgb565());
                base += NAME_H;
            }
            let mut rest = msg;
            for _ in 0..12 {
                let (l, more) = first_line_body(font, rest, TEXT_W);
                if l.is_empty() {
                    break;
                }
                font.draw_scaled(&mut clip, x + PADX, base, l, ink.rgb565(), face.rgb565(), BODY_SCALE_NUM, BODY_SCALE_DEN);
                base += pitch;
                rest = more;
            }
        })
    };
    let view = VIEW_BOTTOM - VIEW_TOP;
    if total + 8 > view {
        let (y0, h) = (170, 150);
        s.rect(436, y0, 4, h, BAR_TRACK.rgb565());
        let range = (total + 8 - view).max(1);
        let thumb = (h * view / (total + 8)).max(14);
        let at = y0 + (h - thumb) * (scroll_px as i32).min(range) / range;
        s.rect(436, at, 4, thumb, WHITE.rgb565());
    }
    // the sign and the message box float over the bubbles, each with a black rim round it
    // no circle: a smooth black rim follows the sign's own outline (tail and all)
    let behind = |px: i32, py: i32| {
        under[..under_n].iter().find(|b| px >= b.0 && px < b.0 + b.2 && py >= b.1 && py < b.1 + b.3).map_or(0, |b| b.4)
    };
    if crate::icons::rim_small(s, c.app, C, 42, 5, behind) {
        crate::icons::draw_small(s, c.app, C, 42);
    } else {
        // any other app: its round sign, with the edge of the black circle blended into what lies under it
        disc(s, C, 42, 27, BLACK, Rgb::from565(behind(C, 42)));
        app_icon(s, c.app, C, 42, 22, BLACK);
    }
    let face = if reply_down { KEY_DOWN } else { CHIP };
    pill(s, REPLY_X - 7, REPLY_Y - 7, REPLY_W + 14, REPLY_H + 14, BLACK);
    pill(s, REPLY_X, REPLY_Y, REPLY_W, REPLY_H, face);
    B.draw(s, REPLY_X + 26, REPLY_Y + (REPLY_H + B.cap) / 2, "Message", GREY.rgb565(), face.rgb565());
}

// ---- the keyboard ----

pub const SEND_X: i32 = 332;
pub const BAR_Y: i32 = 87;
pub const BAR_H: i32 = 46;
const CHIPS_FROM: i32 = 78;
const CHIPS_TO: i32 = 326;

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
    let (x, base) = (98, 72);
    let t = core::str::from_utf8(text).unwrap_or("");
    if t.is_empty() {
        s.rect(x - 5, 48, 3, 28, BLUE_LIGHT.rgb565());
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
        s.rect((left - 5).max(x - 5), 48, 3, 28, BLUE_LIGHT.rgb565());
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
        s.rect(end + 2, 48, 3, 28, BLUE_LIGHT.rgb565());
    }
}

/// The quick replies (while there is no text yet) and the send button.
pub fn bar(s: &mut impl Surface, m: &Model, text: &[u8], listening: bool, voice_error: bool) {
    s.rect(60, BAR_Y - 2, SIZE - 120, BAR_H + 4, 0);
    // The unused arrow tile is removed; suggestions use the reclaimed horizontal space.
    {
        let (items, k) = m.items(text);
        let (w, n) = chips(&items[..k]);
        for (i, &(x, cw)) in w[..n].iter().enumerate() {
            tile(s, x, BAR_Y, cw, BAR_H, CHIP);
            centred(s, &B, x + cw / 2, BAR_Y + 31, items[i], cw - 16, WHITE, CHIP);
        }
    }
    // One action slot inside the keyboard: microphone while empty, blue send arrow once text exists.
    let on = !text.is_empty();
    let face = if on { BLUE_LIGHT } else if listening { RED } else { CHIP };
    pill(s, SEND_X, BAR_Y - 5, 84, BAR_H + 10, face);
    let (cx, cy) = (SEND_X + 42, BAR_Y + BAR_H / 2);
    if on {
        let ink = Rgb(28, 40, 76);
        // outlined paper plane
        let (a, b, c, d) = ((cx - 17, cy - 18), (cx + 20, cy), (cx - 17, cy + 18), (cx - 8, cy));
        for (p, q) in [(a, b), (b, c), (c, d), (d, a)] {
            line(s, p.0, p.1, q.0, q.1, 3, ink, face);
        }
    } else {
        let ink = if listening { WHITE } else if voice_error { RED } else { Rgb(218, 222, 230) };
        // Draw the cradle first, then a solid capsule head over its upper ends.
        // This reads as a microphone at the watch's small scale instead of a hollow rounded box.
        line(s, cx - 12, cy - 1, cx - 12, cy + 9, 3, ink, face);
        line(s, cx + 12, cy - 1, cx + 12, cy + 9, 3, ink, face);
        line(s, cx - 12, cy + 9, cx + 12, cy + 9, 3, ink, face);
        line(s, cx, cy + 9, cx, cy + 18, 3, ink, face);
        line(s, cx - 8, cy + 18, cx + 8, cy + 18, 3, ink, face);
        // Capsule: two end discs and a filled centre. Small dark grille marks give it detail.
        disc(s, cx, cy - 9, 7, ink, face);
        s.rect(cx - 7, cy - 9, 14, 14, ink.rgb565());
        disc(s, cx, cy + 5, 7, ink, face);
        line(s, cx - 3, cy - 11, cx + 3, cy - 11, 2, face, ink);
        line(s, cx - 3, cy - 5, cx + 3, cy - 5, 2, face, ink);
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
            // Latin letters in the bigger font; the wide Arabic shapes keep the smaller one so they stay inside their keys
            let f = if ch.is_ascii() { &T } else { &B };
            put(s, f, cx - f.width(t) / 2, cy + f.cap / 2, t, Rgb(232, 234, 237), under);
        }
        Key::Shift => {
            // an outlined up arrow
            let ink = if shift { BLUE_LIGHT } else { Rgb(232, 234, 237) };
            let mut p = [(0, 0); 8];
            let n = scaled(cx, cy, 42, &[(0, -34), (34, 2), (15, 2), (15, 34), (-15, 34), (-15, 2), (-34, 2)], &mut p);
            outline(s, &p[..n], ink, under);
        }
        Key::Back => {
            // an outlined delete sign (a tag pointing left) with a cross in it
            let ink = Rgb(232, 234, 237);
            let mut p = [(0, 0); 8];
            let n = scaled(cx, cy, 56, &[(-40, 0), (-14, -32), (40, -32), (40, 32), (-14, 32)], &mut p);
            outline(s, &p[..n], ink, under);
            let x = cx + 5;
            line(s, x - 6, cy - 6, x + 6, cy + 6, 3, ink, under);
            line(s, x + 6, cy - 6, x - 6, cy + 6, 3, ink, under);
        }
        Key::Space => {
            // Show the active language in that language: English or العربية.
            let t = if page.is_arabic() { "العربية" } else { "English" };
            // Dark gray, rather than white, keeps the language hint understated.
            put(s, &B, cx - B.width(t) / 2, cy + B.cap / 2, t, Rgb(105, 109, 114), under);
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
    keyboard_voice(s, m, to, text, page, shift, false, false);
}

/// Keyboard renderer with the dictation button state. The six-argument wrapper above
/// remains for the existing framebuffer tests and older internal callers.
pub fn keyboard_voice(s: &mut impl Surface, m: &Model, to: &str, text: &[u8], page: Page, shift: bool, listening: bool, voice_error: bool) {
    s.rect(0, 0, SIZE, SIZE, 0);
    field(s, to, text);
    bar(s, m, text, listening, voice_error);
    all_keys(s, page, shift);
}

/// Only the top part (the text changed).
pub fn text_changed(s: &mut impl Surface, m: &Model, to: &str, text: &[u8]) {
    text_changed_voice(s, m, to, text, false, false);
}

/// Redraw the text field and dictation/send control while keeping the legacy test API.
pub fn text_changed_voice(s: &mut impl Surface, m: &Model, to: &str, text: &[u8], listening: bool, voice_error: bool) {
    s.rect(40, 30, SIZE - 80, keys::TOP - 34, 0);
    field(s, to, text);
    bar(s, m, text, listening, voice_error);
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
        assert!(B.covers("مرحبا"));
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
        // a conversation: their messages, my replies, long and short
        let mut conv = Model::new();
        for (i, t) in ["Hey, are you coming tonight?", "Bring the charger please", "Also milk"].iter().enumerate() {
            let mut v = vec![b'r', b'1', b'L', 4, 1, 9, 5 + i as u8, 14];
            v.extend_from_slice(b"Jane Appleseed");
            v.extend_from_slice(t.as_bytes());
            conv.apply(&v);
            if i == 1 {
                conv.sent(4, b"Yes, I'll be there soon.");
            }
        }
        conv.sent(4, b"ok");
        let max = detail_max(&conv.chats[0]);
        for sc in 0..=max {
            detail(&mut fb, &conv.chats[0], sc, false);
            assert_eq!(lit_outside(&fb), None, "conv {sc}");
            if sc == max { shot("conv_end", &fb); }
            if sc == 0 { shot("conv_top", &fb); }
        }
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
