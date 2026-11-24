//! Suggested words: the first few words from this list that start like the word being typed (the list is roughly
//! ordered by how common the words are, so the likeliest come first).

const WORDS: &str = concat!(
    "the\n",
    "be\n",
    "to\n",
    "of\n",
    "and\n",
    "in\n",
    "that\n",
    "have\n",
    "it\n",
    "for\n",
    "not\n",
    "on\n",
    "with\n",
    "he\n",
    "as\n",
    "you\n",
    "do\n",
    "at\n",
    "this\n",
    "but\n",
    "his\n",
    "by\n",
    "from\n",
    "they\n",
    "we\n",
    "say\n",
    "her\n",
    "she\n",
    "or\n",
    "an\n",
    "will\n",
    "my\n",
    "one\n",
    "all\n",
    "would\n",
    "there\n",
    "their\n",
    "what\n",
    "so\n",
    "up\n",
    "out\n",
    "if\n",
    "about\n",
    "who\n",
    "get\n",
    "which\n",
    "go\n",
    "me\n",
    "when\n",
    "make\n",
    "can\n",
    "like\n",
    "time\n",
    "no\n",
    "just\n",
    "him\n",
    "know\n",
    "take\n",
    "people\n",
    "into\n",
    "year\n",
    "your\n",
    "good\n",
    "some\n",
    "could\n",
    "them\n",
    "see\n",
    "other\n",
    "than\n",
    "then\n",
    "now\n",
    "look\n",
    "only\n",
    "come\n",
    "its\n",
    "over\n",
    "think\n",
    "also\n",
    "back\n",
    "after\n",
    "use\n",
    "two\n",
    "how\n",
    "our\n",
    "work\n",
    "first\n",
    "well\n",
    "way\n",
    "even\n",
    "new\n",
    "want\n",
    "because\n",
    "any\n",
    "these\n",
    "give\n",
    "day\n",
    "most\n",
    "us\n",
    "is\n",
    "are\n",
    "was\n",
    "were\n",
    "been\n",
    "has\n",
    "had\n",
    "did\n",
    "said\n",
    "went\n",
    "got\n",
    "made\n",
    "here\n",
    "home\n",
    "hello\n",
    "hi\n",
    "hey\n",
    "okay\n",
    "ok\n",
    "yes\n",
    "yeah\n",
    "yep\n",
    "nope\n",
    "thanks\n",
    "thank\n",
    "please\n",
    "sorry\n",
    "welcome\n",
    "sure\n",
    "maybe\n",
    "soon\n",
    "later\n",
    "tonight\n",
    "tomorrow\n",
    "today\n",
    "yesterday\n",
    "morning\n",
    "evening\n",
    "night\n",
    "afternoon\n",
    "week\n",
    "weekend\n",
    "monday\n",
    "tuesday\n",
    "wednesday\n",
    "thursday\n",
    "friday\n",
    "saturday\n",
    "sunday\n",
    "where\n",
    "why\n",
    "while\n",
);

const ARABIC_WORDS: &str = concat!(
    "عايز\n",
    "عايزة\n",
    "عايزين\n",
    "دلوقتي\n",
    "معلش\n",
    "انا\n",
    "أنا\n",
    "انت\n",
    "أنت\n",
    "انتي\n",
    "إنت\n",
    "إنتي\n",
    "احنا\n",
    "إحنا\n",
    "انتوا\n",
    "انتو\n",
    "ازيك\n",
    "إزيك\n",
    "اخبارك\n",
    "أخبارك\n",
    "عامل\n",
    "عاملة\n",
    "ايه\n",
    "إيه\n",
    "ازاي\n",
    "إزاي\n",
    "عاوز\n",
    "عاوزة\n",
    "عاوزين\n",
    "مش\n",
    "ماشي\n",
    "تمام\n",
    "طيب\n",
    "طب\n",
    "كده\n",
    "كدا\n",
    "دلوقت\n",
    "لسه\n",
    "لسة\n",
    "برضه\n",
    "برده\n",
    "عشان\n",
    "علشان\n",
    "مفيش\n",
    "مافيش\n",
    "فيه\n",
    "فاضي\n",
    "فاضية\n",
    "ممكن\n",
    "ينفع\n",
    "ينفعش\n",
    "ليه\n",
    "فين\n",
    "امتى\n",
    "إمتى\n",
    "مين\n",
    "كام\n",
    "ازايك\n",
    "إزايك\n",
    "بجد\n",
    "فعلا\n",
    "فعلاً\n",
    "اوي\n",
    "أوي\n",
    "قوي\n",
    "جدا\n",
    "جدًا\n",
    "شوية\n",
    "شويه\n",
    "حاجة\n",
    "حاجه\n",
    "حاجات\n",
    "حلو\n",
    "حلوة\n",
    "حلوين\n",
    "جميل\n",
    "جميلة\n",
    "كويس\n",
    "كويسة\n",
    "تماما\n",
    "خلاص\n",
    "حاضر\n",
    "اه\n",
    "آه\n",
    "ايوه\n",
    "أيوه\n",
    "ايوا\n",
    "أيوة\n",
    "لأ\n",
    "لا\n",
    "لازم\n",
    "أكيد\n",
    "اكيد\n",
    "يمكن\n",
    "غالبا\n",
    "غالبًا\n",
    "تقريبا\n",
    "تقريبًا\n",
    "بالظبط\n",
    "بالضبط\n",
    "طبعا\n",
    "طبعاً\n",
    "يعني\n",
    "بص\n",
    "بصي\n",
    "بصوا\n",
    "شوف\n",
    "شوفي\n",
    "شوفوا\n",
    "استنى\n",
    "استني\n",
    "استنوا\n",
    "ثانية\n",
    "ثانيه\n",
    "دقيقة\n",
    "دقيقه\n",
    "دقايق\n",
    "ساعة\n",
    "ساعه\n",
    "حالاً\n",
    "حالا\n",
    "النهارده\n",
    "النهاردة\n",
    "النهاردا\n",
    "امبارح\n",
    "أمبارح\n",
    "بكرة\n",
    "بكره\n",
    "بكرا\n",
    "بعده\n",
    "الصبح\n",
    "بالليل\n",
    "الليل\n",
    "النهار\n",
    "الجمعة\n",
    "السبت\n",
    "الاحد\n",
    "الأحد\n",
    "الاتنين\n",
    "التلات\n",
    "التلاتة\n",
    "الاربع\n",
    "الأربع\n",
    "الخميس\n",
    "الاسبوع\n",
    "الأسبوع\n",
    "الشهر\n",
    "السنة\n",
    "بعدين\n",
    "قبل\n",
    "مرحبا\n",
    "شكرا\n",
);

/// Up to 3 words that continue `frag` (the letters typed since the last space), not the word itself.
/// True for Arabic letters in the basic and commonly used extended Arabic blocks.
fn is_arabic_letter(c: char) -> bool {
    matches!(c as u32, 0x0621..=0x064A | 0x066E..=0x06D3 | 0x0750..=0x077F | 0x08A0..=0x08FF)
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '\'' || is_arabic_letter(c)
}

fn contains_arabic(text: &str) -> bool {
    text.chars().any(is_arabic_letter)
}

/// Up to 3 words that continue the current English or Arabic word fragment.
pub fn suggest(frag: &[u8], out: &mut [&'static str; 3]) -> usize {
    if frag.is_empty() || frag.len() > 40 {
        return 0;
    }
    let Ok(fragment) = core::str::from_utf8(frag) else { return 0; };
    let dictionary = if contains_arabic(fragment) { ARABIC_WORDS } else { WORDS };
    let mut n = 0;
    for w in dictionary.split('\n') {
        let wb = w.as_bytes();
        if wb.len() > frag.len() && wb[..frag.len()].eq_ignore_ascii_case(frag) && !out[..n].contains(&w) {
            out[n] = w;
            n += 1;
            if n == 3 {
                break;
            }
        }
    }
    n
}

/// The letters being typed at the end of `text` (empty after a space or a symbol).
/// Walk Unicode characters so an Arabic word is never cut in the middle of a UTF-8 sequence.
pub fn fragment(text: &[u8]) -> &[u8] {
    let Ok(s) = core::str::from_utf8(text) else { return &[]; };
    let mut start = 0;
    for (i, c) in s.char_indices() {
        if !is_word_char(c) {
            start = i + c.len_utf8();
        }
    }
    &text[start..]
}

/// Small offline next-word maps. The same suggestion chips are used for both languages.
const ENGLISH_NEXT: &[(&str, &[&str])] = &[
    ("are", &["you", "we", "they"]),
    ("how", &["are", "do", "is"]),
    ("i", &["am", "will", "can"]),
    ("you", &["can", "are", "have"]),
    ("thank", &["you"]),
    ("see", &["you", "soon"]),
    ("good", &["morning", "night", "luck"]),
    ("on", &["my", "the", "it"]),
    ("let", &["me", "us"]),
    ("what", &["do", "is", "about"]),
    ("can", &["you", "i", "we"]),
    ("do", &["you", "we", "i"]),
    ("have", &["a", "to", "you"]),
    ("i'm", &["fine", "good", "sorry"]),
    ("no", &["problem", "worries", "thanks"]),
    ("my", &["friend", "name", "love"]),
    ("looking", &["forward", "for", "at"]),
];

const ARABIC_NEXT: &[(&str, &[&str])] = &[
    ("ازيك", &["عامل", "عاملة"]),
    ("إزيك", &["عامل", "عاملة"]),
    ("عامل", &["ايه", "إيه"]),
    ("عاملة", &["ايه", "إيه"]),
    ("انا", &["تمام", "بخير"]),
    ("أنا", &["تمام", "بخير"]),
    ("انت", &["عامل", "رايح"]),
    ("أنت", &["عامل", "رايح"]),
    ("انتي", &["عاملة", "رايحة"]),
    ("إنتي", &["عاملة", "رايحة"]),
    ("صباح", &["الخير"]),
    ("مساء", &["الخير"]),
    ("شكرا", &["جدا", "ليك"]),
    ("شكراً", &["جدا", "ليك"]),
    ("الحمد", &["لله"]),
    ("ربنا", &["يخليك", "يكرمك"]),
    ("مع", &["السلام", "اكيد"]),
    ("ممكن", &["اسألك", "تساعدني"]),
    ("ازاي", &["كده", "يعني"]),
    ("إزاي", &["كده", "يعني"]),
    ("ايه", &["رأيك", "الموضوع"]),
    ("إيه", &["رأيك", "الموضوع"]),
    ("فين", &["انت", "انتي"]),
    ("بعد", &["كده", "شوية"]),
    ("كل", &["حاجة", "يوم"]),
    ("ماشي", &["تمام", "خلاص"]),
    ("تمام", &["الحمدلله", "كده"]),
    ("معلش", &["ولا", "مفيش"]),
    ("مش", &["عارف", "فاهم"]),
    ("عشان", &["كده", "انا"]),
    ("كده", &["يعني", "تمام"]),
    ("برضه", &["مش", "كده"]),
    ("دلوقتي", &["لا", "اه"]),
    ("بكرة", &["إن", "الصبح"]),
    ("يا", &["حبيبي", "باشا"]),
    ("وحشت", &["ني"]),
];

/// Up to three next-word suggestions after a trailing space, based on the previous word.
pub fn predict_next(text: &[u8], out: &mut [&'static str; 3]) -> usize {
    if text.last() != Some(&b' ') || text.len() < 2 {
        return 0;
    }
    let previous = fragment(&text[..text.len() - 1]);
    if previous.is_empty() {
        return 0;
    }
    let Ok(previous_text) = core::str::from_utf8(previous) else { return 0; };
    let is_arabic = contains_arabic(previous_text);
    let dictionary = if is_arabic { ARABIC_NEXT } else { ENGLISH_NEXT };
    for (word, candidates) in dictionary {
        let matched = if is_arabic {
            word.as_bytes() == previous
        } else {
            word.as_bytes().eq_ignore_ascii_case(previous)
        };
        if matched {
            let n = candidates.len().min(3);
            for (i, candidate) in candidates[..n].iter().enumerate() {
                out[i] = candidate;
            }
            return n;
        }
    }
    0
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ho_offers_home() {
        let mut o = [""; 3];
        let n = suggest(b"Ho", &mut o);
        assert!(n >= 1 && o[..n].contains(&"home"), "{:?}", &o[..n]);
        assert_eq!(suggest(b"", &mut o), 0);
        assert_eq!(fragment(b"see you at ho"), b"ho");
        assert_eq!(fragment(b"hi "), b"");
    }

    #[test]
    fn arabic_prefixes_offer_arabic_words() {
        let mut out = [""; 3];
        let n = suggest("مر".as_bytes(), &mut out);
        assert!(n > 0 && out[..n].contains(&"مرحبا"), "{:?}", &out[..n]);

        let n = suggest("شك".as_bytes(), &mut out);
        assert!(n > 0 && out[..n].contains(&"شكرا"), "{:?}", &out[..n]);
    }

    #[test]
    fn fragment_finds_arabic_utf8_words_after_spaces_and_punctuation() {
        assert_eq!(fragment("انا في البيت".as_bytes()), "البيت".as_bytes());
        assert_eq!(fragment("Hello، يا صديقي".as_bytes()), "صديقي".as_bytes());
        assert_eq!(fragment("مرحبا ".as_bytes()), b"");
        assert_eq!(fragment("hello! مر".as_bytes()), "مر".as_bytes());
    }

    #[test]
    fn egyptian_dictionary_prioritizes_everyday_colloquial_words() {
        let mut out = [""; 3];
        let n = suggest("عا".as_bytes(), &mut out);
        assert!(n > 0 && out[..n].contains(&"عايز"), "{:?}", &out[..n]);
        let n = suggest("دل".as_bytes(), &mut out);
        assert!(n > 0 && out[..n].contains(&"دلوقتي"), "{:?}", &out[..n]);
        let n = suggest("معل".as_bytes(), &mut out);
        assert!(n > 0 && out[..n].contains(&"معلش"), "{:?}", &out[..n]);
    }

    #[test]
    fn next_word_predictions_work_in_both_languages_without_hiding_quick_replies() {
        let mut model = crate::proto::Model::new();
        model.set_defaults();
        let (quick, quick_n) = model.items(b"");
        assert_eq!(quick_n, 3);
        assert!(quick[..quick_n].contains(&"Ok"));

        let (english, english_n) = model.items(b"How are ");
        assert!(english_n > 0 && english[..english_n].contains(&"you"));

        let (arabic, arabic_n) = model.items("ازيك ".as_bytes());
        assert!(arabic_n > 0 && arabic[..arabic_n].contains(&"عامل"));
    }
}
