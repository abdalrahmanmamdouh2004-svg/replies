//! The keyboard: English and Arabic letters plus a numbers page.
//! Holding Space changes language; tapping Space inserts a normal space.

pub const MAX_KEYS: usize = 40;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Page {
    Letters,
    Numbers,
    ArabicLetters,
    ArabicNumbers,
}

impl Page {
    pub const fn is_arabic(self) -> bool {
        matches!(self, Self::ArabicLetters | Self::ArabicNumbers)
    }

    pub const fn is_letters(self) -> bool {
        matches!(self, Self::Letters | Self::ArabicLetters)
    }

    pub const fn toggle_language(self) -> Self {
        match self {
            Self::Letters => Self::ArabicLetters,
            Self::Numbers => Self::ArabicNumbers,
            Self::ArabicLetters => Self::Letters,
            Self::ArabicNumbers => Self::Numbers,
        }
    }

    pub const fn toggle_numbers(self) -> Self {
        match self {
            Self::Letters => Self::Numbers,
            Self::Numbers => Self::Letters,
            Self::ArabicLetters => Self::ArabicNumbers,
            Self::ArabicNumbers => Self::ArabicLetters,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Key {
    /// ASCII input uses its byte value. Arabic letters and Arabic-Indic digits use private key codes
    /// which `character()` converts to Unicode before a character is appended to the reply.
    Ch(u8),
    Shift,
    Back,
    Space,
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

const ARABIC_LETTERS: [char; 28] = [
    'ض', 'ص', 'ث', 'ق', 'ف', 'غ', 'ع', 'ه', 'خ', 'ح',
    'ش', 'س', 'ي', 'ب', 'ل', 'ا', 'ت', 'ن', 'م',
    'ج', 'د', 'ذ', 'ر', 'ز', 'ط', 'ظ', 'ك', 'و',
];

const ARABIC_KEY_BASE: u8 = 0x80;
const ARABIC_VARIANT_BASE: u8 = 0xC0;
const ARABIC_PUNCT_BASE: u8 = 0xD0;
const ARABIC_DIGIT_BASE: u8 = 0xE0;
const ARABIC_VARIANTS: [char; 8] = ['ة', 'ى', 'ء', 'أ', 'إ', 'آ', 'ؤ', 'ئ'];
const ARABIC_PUNCTUATION: [char; 3] = ['،', '؛', '؟'];

/// Converts an internal keyboard code to the Unicode character it represents.
pub fn character(code: u8) -> char {
    if (ARABIC_KEY_BASE..ARABIC_KEY_BASE + ARABIC_LETTERS.len() as u8).contains(&code) {
        return ARABIC_LETTERS[(code - ARABIC_KEY_BASE) as usize];
    }
    if (ARABIC_VARIANT_BASE..ARABIC_VARIANT_BASE + ARABIC_VARIANTS.len() as u8).contains(&code) {
        return ARABIC_VARIANTS[(code - ARABIC_VARIANT_BASE) as usize];
    }
    if (ARABIC_PUNCT_BASE..ARABIC_PUNCT_BASE + ARABIC_PUNCTUATION.len() as u8).contains(&code) {
        return ARABIC_PUNCTUATION[(code - ARABIC_PUNCT_BASE) as usize];
    }
    if (ARABIC_DIGIT_BASE..ARABIC_DIGIT_BASE + 10).contains(&code) {
        return char::from_u32(0x0660 + (code - ARABIC_DIGIT_BASE) as u32).unwrap_or('?');
    }
    code as char
}

fn arabic_key(c: char) -> Key {
    let i = ARABIC_LETTERS.iter().position(|&v| v == c).unwrap_or(0);
    Key::Ch(ARABIC_KEY_BASE + i as u8)
}

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
        (Page::ArabicLetters, 0) => "ضصثقفغعهخح".chars().for_each(|c| put(arabic_key(c), 44)),
        (Page::ArabicLetters, 1) => "شسيبلاتنم".chars().for_each(|c| put(arabic_key(c), 49)),
        (Page::ArabicLetters, 2) => {
            put(arabic_key('ج'), 52);
            "دذرزطظك".chars().for_each(|c| put(arabic_key(c), 42));
            put(arabic_key('و'), 52);
        }
        (Page::Numbers, 0) => b"1234567890".iter().for_each(|&c| put(Key::Ch(c), 44)),
        (Page::ArabicNumbers, 0) => (0..10).for_each(|i| put(Key::Ch(ARABIC_DIGIT_BASE + i), 44)),
        (Page::Numbers, 1) => b"-/:;()$&@".iter().for_each(|&c| put(Key::Ch(c), 49)),
        (Page::ArabicNumbers, 1) => (0..ARABIC_VARIANTS.len()).for_each(|i| put(Key::Ch(ARABIC_VARIANT_BASE + i as u8), 49)),
        (Page::Numbers, 2) => {
            put(Key::Ch(b'#'), 52);
            b".,?!'\"+".iter().for_each(|&c| put(Key::Ch(c), 42));
            put(Key::Ch(b'%'), 52);
        }
        (Page::ArabicNumbers, 2) => {
            put(Key::Ch(b'#'), 52);
            for i in 0..3 { put(Key::Ch(ARABIC_PUNCT_BASE + i), 42); }
            b".!'\"".iter().for_each(|&c| put(Key::Ch(c), 42));
            put(Key::Ch(b'%'), 52);
        }
        (_, 3) => {
            put(Key::Page, 70);
            put(Key::Space, 178);
            put(Key::Back, 70);
        }
        _ => {}
    }
    (out, n)
}

pub fn layout(page: Page) -> ( [Cell; MAX_KEYS], usize ) {
    let mut out = [Cell { key: Key::Space, x: 0, y: 0, w: 0, h: 0 }; MAX_KEYS];
    let mut n = 0;
    for r in 0..4 {
        let (row, len) = row_of(page, r);
        let total: i32 = row[..len].iter().map(|k| k.1).sum();
        let mut x = crate::C - total / 2;
        for &(key, w) in &row[..len] {
            let h = if key == Key::Space { KEY_H + 36 } else { KEY_H };
            out[n] = Cell { key, x, y: TOP + r as i32 * KEY_H, w, h };
            n += 1;
            x += w;
        }
    }
    (out, n)
}

pub fn key_at(page: Page, x: i32, y: i32) -> Option<Cell> {
    let row = (y - TOP).div_euclid(KEY_H);
    if row < 0 || y >= TOP + 4 * KEY_H + 36 { return None; }
    let row = row.min(3);
    let (cells, n) = layout(page);
    if y >= TOP + 4 * KEY_H {
        return cells[..n].iter().find(|c| c.key == Key::Space && x >= c.x && x < c.x + c.w).copied();
    }
    let mut in_row = cells[..n].iter().filter(|c| c.y == TOP + row * KEY_H);
    let first = *in_row.clone().next()?;
    let last = *in_row.clone().last()?;
    if x < first.x - 20 || x >= last.x + last.w + 20 { return None; }
    if x < first.x { return Some(first); }
    in_row.find(|c| x >= c.x && x < c.x + c.w).copied().or(Some(last))
}

pub fn sentence_start(text: &[u8]) -> bool {
    let Ok(s) = core::str::from_utf8(text) else { return false; };
    let t = s.trim_end_matches(' ');
    if t.is_empty() { return true; }
    if t.len() == s.len() { return false; }
    matches!(t.chars().last(), Some('.' | '?' | '!' | '؟' | '۔'))
}

/// The byte code for the character a key types. Arabic codes are expanded by Draft::push.
pub fn typed(key: Key, shift: bool) -> Option<u8> {
    match key {
        Key::Ch(c) if c < 0x80 => Some(if shift { c.to_ascii_uppercase() } else { c }),
        Key::Ch(c) => Some(c),
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
    fn all_layouts_tile_the_rows_and_stay_on_the_round_screen() {
        for page in [Page::Letters, Page::Numbers, Page::ArabicLetters, Page::ArabicNumbers] {
            let (cells, n) = layout(page);
            for (i, c) in cells[..n].iter().enumerate() {
                assert!(inside(c.x + c.w / 2, c.y + c.h / 2), "{page:?} {:?}", c.key);
                assert!(inside(c.x + c.w / 2, c.y + 12) && inside(c.x + c.w / 2, c.y + c.h - 12), "{:?}", c.key);
                for d in &cells[i + 1..n] {
                    let apart = c.x + c.w <= d.x || d.x + d.w <= c.x || c.y + c.h <= d.y || d.y + d.h <= c.y;
                    assert!(apart, "{page:?}: {:?} overlaps {:?}", c.key, d.key);
                }
            }
            for r in 0..4 {
                let row: std::vec::Vec<_> = cells[..n].iter().filter(|c| c.y == TOP + r * KEY_H).collect();
                for w in row.windows(2) { assert_eq!(w[0].x + w[0].w, w[1].x, "{page:?} row {r}"); }
            }
        }
    }

    #[test]
    fn all_english_and_arabic_letters_are_represented_once() {
        let (cells, n) = layout(Page::Letters);
        for ch in b'a'..=b'z' { assert_eq!(cells[..n].iter().filter(|c| c.key == Key::Ch(ch)).count(), 1, "{}", ch as char); }
        let (cells, n) = layout(Page::ArabicLetters);
        assert_eq!(cells[..n].iter().filter(|c| matches!(c.key, Key::Ch(v) if v >= ARABIC_KEY_BASE && v < ARABIC_KEY_BASE + 28)).count(), 28);
        for ch in ARABIC_LETTERS {
            let key = arabic_key(ch);
            assert_eq!(character(match key { Key::Ch(code) => code, _ => 0 }), ch);
        }
    }

    #[test]
    fn arabic_digits_and_language_switch_are_correct() {
        let (cells, n) = layout(Page::ArabicNumbers);
        for i in 0..10 { assert!(cells[..n].iter().any(|c| c.key == Key::Ch(ARABIC_DIGIT_BASE + i))); }
        for i in 0..ARABIC_VARIANTS.len() { assert!(cells[..n].iter().any(|c| c.key == Key::Ch(ARABIC_VARIANT_BASE + i as u8))); }
        for i in 0..ARABIC_PUNCTUATION.len() { assert!(cells[..n].iter().any(|c| c.key == Key::Ch(ARABIC_PUNCT_BASE + i as u8))); }
        assert_eq!(character(ARABIC_DIGIT_BASE + 4), '٤');
        assert_eq!(character(ARABIC_VARIANT_BASE), 'ة');
        assert_eq!(character(ARABIC_VARIANT_BASE + 7), 'ئ');
        assert_eq!(character(ARABIC_PUNCT_BASE + 2), '؟');
        assert_eq!(Page::Letters.toggle_language(), Page::ArabicLetters);
        assert_eq!(Page::ArabicNumbers.toggle_language(), Page::Numbers);
        assert_eq!(Page::ArabicLetters.toggle_numbers(), Page::ArabicNumbers);
    }

    #[test]
    fn touches_find_keys_in_every_layout() {
        for page in [Page::Letters, Page::Numbers, Page::ArabicLetters, Page::ArabicNumbers] {
            let (cells, n) = layout(page);
            for c in &cells[..n] {
                let mid = key_at(page, c.x + c.w / 2, c.y + c.h / 2).unwrap();
                assert_eq!(mid.key, c.key);
                let low = key_at(page, c.x + 1, c.y + c.h - 1).unwrap();
                assert_eq!(low.key, c.key);
            }
        }
    }

    #[test]
    fn shift_only_changes_english_letters() {
        assert_eq!(typed(Key::Ch(b'a'), true), Some(b'A'));
        assert_eq!(character(typed(arabic_key('ض'), true).unwrap()), 'ض');
        assert_eq!(typed(Key::Space, false), Some(b' '));
        assert!(sentence_start("انتهى؟ ".as_bytes()));
        assert!(!sentence_start("انتهى؟".as_bytes()));
        assert_eq!(typed(Key::Back, false), None);
    }
}
