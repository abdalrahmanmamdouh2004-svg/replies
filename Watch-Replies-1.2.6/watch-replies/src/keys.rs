//! The keyboard: where the keys are and what they type. Two pages, letters and numbers. A key is a plain rectangle;
//! the picture of it is in `view.rs`.

pub const MAX_KEYS: usize = 40;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Letters,
    Numbers,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    /// A character (an ASCII byte); a letter is capitalised by the shift state.
    Ch(u8),
    Shift,
    Back,
    Space,
    /// Switches between the letters and the numbers.
    Page,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Cell {
    pub key: Key,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

pub const KEY_H: i32 = 58;
pub const TOP: i32 = 172;

/// A row: its keys with their widths. Rows are as wide as the round screen allows at their height; the keys touch (the gaps you see are drawn inside the
/// keys' own areas by `view.rs`), so every touch lands on some key.
fn row_of(page: Page, r: usize) -> ([(Key, i32); 10], usize) {
    let mut out = [(Key::Space, 0); 10];
    let mut n = 0;
    let mut put = |k: Key, w: i32| {
        out[n] = (k, w);
        n += 1;
    };
    match (page, r) {
        (Page::Letters, 0) => b"qwertyuiop".iter().for_each(|&c| put(Key::Ch(c), 44)),
        (Page::Letters, 1) => b"asdfghjkl".iter().for_each(|&c| put(Key::Ch(c), 49)),
        (Page::Letters, 2) => {
            put(Key::Shift, 52);
            b"zxcvbnm".iter().for_each(|&c| put(Key::Ch(c), 42));
            put(Key::Ch(b'.'), 52);
        }
        (Page::Numbers, 0) => b"1234567890".iter().for_each(|&c| put(Key::Ch(c), 44)),
        (Page::Numbers, 1) => b"-/:;()$&@".iter().for_each(|&c| put(Key::Ch(c), 49)),
        (Page::Numbers, 2) => {
            put(Key::Ch(b'#'), 52);
            b".,?!'\"+".iter().for_each(|&c| put(Key::Ch(c), 42));
            put(Key::Ch(b'%'), 52);
        }
        _ => {
            // number key, space bar and delete side by side, as on the Wear OS keyboard
            put(Key::Page, 70);
            put(Key::Space, 178);
            put(Key::Back, 70);
        }
    }
    (out, n)
}

/// The keys of a page, like a phone keyboard shrunk onto the watch: three rows of letters and under them the
/// number key, the space bar and delete.
pub fn layout(page: Page) -> ([Cell; MAX_KEYS], usize) {
    let mut out = [Cell { key: Key::Space, x: 0, y: 0, w: 0, h: 0 }; MAX_KEYS];
    let mut n = 0;
    for r in 0..4 {
        let (row, len) = row_of(page, r);
        let total: i32 = row[..len].iter().map(|k| k.1).sum();
        let mut x = crate::C - total / 2;
        for &(key, w) in &row[..len] {
            // the space bar is taller and reaches down towards the round edge of the screen
            let h = if key == Key::Space { KEY_H + 36 } else { KEY_H };
            out[n] = Cell { key, x, y: TOP + r as i32 * KEY_H, w, h };
            n += 1;
            x += w;
        }
    }
    (out, n)
}

/// The key under a touch. A touch a little beyond the end of a row still belongs to the key at that end.
pub fn key_at(page: Page, x: i32, y: i32) -> Option<Cell> {
    let row = (y - TOP).div_euclid(KEY_H);
    // the space bar reaches below the bottom row
    if row < 0 || y >= TOP + 4 * KEY_H + 36 {
        return None;
    }
    let row = row.min(3);
    if y >= TOP + 4 * KEY_H {
        // below the keys' own row only the space bar is there
        let (cells, n) = layout(page);
        return cells[..n].iter().find(|c| c.key == Key::Space && x >= c.x && x < c.x + c.w).copied();
    }
    let (cells, n) = layout(page);
    let mut in_row = cells[..n].iter().filter(|c| c.y == TOP + row * KEY_H);
    let first = *in_row.clone().next()?;
    let last = *in_row.clone().last()?;
    if x < first.x - 20 || x >= last.x + last.w + 20 {
        return None;
    }
    if x < first.x {
        return Some(first);
    }
    in_row.find(|c| x >= c.x && x < c.x + c.w).copied().or(Some(last))
}

/// The text so far ends a sentence (or is empty), so the next letter is a capital.
pub fn sentence_start(text: &[u8]) -> bool {
    let t = match text.iter().rposition(|&b| b != b' ') {
        None => return true,
        Some(i) => &text[..=i],
    };
    text.len() > t.len() && matches!(t[t.len() - 1], b'.' | b'?' | b'!')
}

/// The character a key types, with the shift state applied; `None` for keys that are not characters.
pub fn typed(key: Key, shift: bool) -> Option<u8> {
    match key {
        Key::Ch(c) => Some(if shift { c.to_ascii_uppercase() } else { c }),
        Key::Space => Some(b' '),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{C, SIZE};

    fn inside(x: i32, y: i32) -> bool {
        let (dx, dy) = (x - C, y - C);
        dx * dx + dy * dy <= (SIZE / 2 - 4) * (SIZE / 2 - 4)
    }

    #[test]
    fn the_keys_tile_their_rows_and_their_centres_are_on_the_screen() {
        for page in [Page::Letters, Page::Numbers] {
            let (cells, n) = layout(page);
            for (i, c) in cells[..n].iter().enumerate() {
                assert!(inside(c.x + c.w / 2, c.y + c.h / 2), "{page:?} {:?}", c.key);
                assert!(inside(c.x + c.w / 2, c.y + 12) && inside(c.x + c.w / 2, c.y + c.h - 12), "{:?}", c.key);
                for d in &cells[i + 1..n] {
                    let apart = c.x + c.w <= d.x || d.x + d.w <= c.x || c.y + c.h <= d.y || d.y + d.h <= c.y;
                    assert!(apart, "{page:?}: {:?} overlaps {:?}", c.key, d.key);
                }
            }
            // each row is one run of touching keys
            for r in 0..4 {
                let row: std::vec::Vec<_> = cells[..n].iter().filter(|c| c.y == TOP + r * KEY_H).collect();
                for w in row.windows(2) {
                    assert_eq!(w[0].x + w[0].w, w[1].x, "{page:?} row {r}");
                }
            }
        }
    }

    #[test]
    fn the_keyboard_fills_the_screen() {
        let (cells, n) = layout(Page::Letters);
        let left = cells[..n].iter().map(|c| c.x).min().unwrap();
        let right = cells[..n].iter().map(|c| c.x + c.w).max().unwrap();
        let bottom = cells[..n].iter().map(|c| c.y + c.h).max().unwrap();
        assert!(right - left >= 380 && bottom >= 380, "{left}..{right} {bottom}");
    }

    #[test]
    fn the_letters_page_has_every_letter_once() {
        let (cells, n) = layout(Page::Letters);
        for ch in b'a'..=b'z' {
            assert_eq!(cells[..n].iter().filter(|c| c.key == Key::Ch(ch)).count(), 1, "{}", ch as char);
        }
        for k in [Key::Shift, Key::Space, Key::Page] {
            assert_eq!(cells[..n].iter().filter(|c| c.key == k).count(), 1, "{k:?}");
        }
        assert_eq!(cells[..n].iter().filter(|c| c.key == Key::Back).count(), 1);
        let (cells, n) = layout(Page::Numbers);
        for ch in b"0123456789.,?!'\"-/:;()$&@#+" {
            assert!(cells[..n].iter().any(|c| c.key == Key::Ch(*ch)), "{}", *ch as char);
        }
        assert!(cells[..n].iter().all(|c| c.key != Key::Shift));
    }

    #[test]
    fn a_touch_finds_its_key() {
        let (cells, n) = layout(Page::Letters);
        for c in &cells[..n] {
            let mid = key_at(Page::Letters, c.x + c.w / 2, c.y + c.h / 2).unwrap();
            assert_eq!(mid.key, c.key);
            let low = key_at(Page::Letters, c.x + 1, c.y + c.h - 1).unwrap();
            assert_eq!(low.key, c.key);
        }
        let q = cells[0];
        // just outside the row's end still counts for its end key; far outside, above or below the keys, for none
        assert_eq!(key_at(Page::Letters, q.x - 8, q.y + 10).map(|c| c.key), Some(Key::Ch(b'q')));
        assert_eq!(key_at(Page::Letters, q.x - 60, q.y + 10), None);
        assert_eq!(key_at(Page::Letters, C, TOP - 4), None);
        assert_eq!(key_at(Page::Letters, C, TOP + 4 * KEY_H + 40), None);
        assert_eq!(key_at(Page::Letters, C, TOP + 4 * KEY_H + 20).map(|c| c.key), Some(Key::Space));
        assert_eq!(key_at(Page::Letters, 100, TOP + 4 * KEY_H + 20).map(|c| c.key), None, "under ?123 is nothing");
    }

    #[test]
    fn capitals_start_sentences() {
        assert!(sentence_start(b""));
        assert!(sentence_start(b"Hi. "));
        assert!(!sentence_start(b"Hi."));
        assert!(!sentence_start(b"Hi there"));
        assert!(sentence_start(b"ok? "));
        assert_eq!(typed(Key::Ch(b'a'), true), Some(b'A'));
        assert_eq!(typed(Key::Ch(b'1'), true), Some(b'1'));
        assert_eq!(typed(Key::Space, false), Some(b' '));
        assert_eq!(typed(Key::Back, false), None);
    }
}
