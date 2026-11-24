//! The messages between Watch Replies and the phone (the Watch Menu app). Every message starts with `r1` and a kind
//! byte, so the phone and the other watch apps tell them apart; 1..=128 bytes each.
//!
//! Phone -> watch:
//!   `r1C`                         forget every chat (a fresh list follows)
//!   `r1L` id app hh mm nlen name msg   a chat has a new message: `app` 1 WhatsApp, 2 Messenger, 3 Telegram, anything else;
//!                                 `hh mm` the time it came; the name is `nlen` bytes, the message is the rest; plain
//!                                 ASCII (the phone replaces what the watch cannot draw). The newest chat is first.
//!   `r1D` id                      that chat is gone (answered or dismissed on the phone)
//!   `r1Q` n (len text)*           up to 3 quick replies
//!   `r1R` id status               the answer to a reply: 0 sent, 1 could not be sent
//!   `r1U` id app hh mm nlen name msg  quiet update for the conversation currently open; does not move its inbox card
//!   `r1S` state                   dictation state: 0 ready, 1 listening, 2 error
//!   `r1T` flags text              recognized UTF-8 text; flags bit 7 = last piece, bits 0..3 = piece number
//! Watch -> phone:
//!   `r1h` version                 hello: the app is open (repeated until the phone answers)
//!   `r1x` id flags text           a reply for chat `id`; `flags` bit 7 = the last piece, bits 0..3 = the piece number
//!   `r1m` language                toggle phone-microphone dictation; `a` Arabic (Egypt), `e` English
//!   `r1f` id                      heartbeat: conversation `id` is open (0 = inbox / no focused chat)
//!   `r1z`                         app is closing; clear the focused conversation immediately
//!                                 (0 first); the pieces are `CHUNK` bytes of text each

pub const PREFIX: [u8; 2] = *b"r1";
pub const VERSION: u8 = 1;
pub const MAX_CHATS: usize = 8;
pub const NAME_MAX: usize = 24;
pub const TEXT_MAX: usize = 96;
/// Bytes of earlier messages kept per chat (messages separated by a newline, oldest first).
pub const LOG_MAX: usize = 400;
/// First byte of a log entry that is a reply sent from the watch.
const MINE: u8 = 1;
/// The most of a sent reply that is kept in the log.
const MINE_MAX: usize = 120;
pub const QUICK_MAX: usize = 3;
pub const QUICK_LEN: usize = 24;
/// Bytes of reply text in one message.
pub const CHUNK: usize = 100;
/// The longest reply (two messages: the watch's outbox holds four).
pub const REPLY_MAX: usize = 2 * CHUNK;

pub const APP_WHATSAPP: u8 = 1;
pub const APP_MESSENGER: u8 = 2;
pub const APP_TELEGRAM: u8 = 3;

pub fn hello() -> [u8; 4] {
    [b'r', b'1', b'h', VERSION]
}

/// Toggle phone-microphone dictation. Phone replies with r1S state and r1T transcript chunks.
pub fn dictation_toggle(arabic: bool) -> [u8; 4] {
    [b'r', b'1', b'm', if arabic { b'a' } else { b'e' }]
}

/// Heartbeat sent while the watch app is open. `0` means the inbox, not a conversation.
pub fn focus(chat_id: u8) -> [u8; 4] {
    [b'r', b'1', b'f', chat_id]
}

/// Explicit close; the phone also expires focus if this message is lost.
pub fn close_focus() -> [u8; 3] {
    *b"r1z"
}

#[derive(Clone, Copy)]
pub struct Chat {
    pub id: u8,
    pub app: u8,
    pub hh: u8,
    pub mm: u8,
    nlen: u8,
    mlen: u8,
    name: [u8; NAME_MAX],
    msg: [u8; TEXT_MAX],
    llen: u16,
    log: [u8; LOG_MAX],
}

fn text(b: &[u8]) -> &str {
    core::str::from_utf8(b).unwrap_or("?")
}

impl Chat {
    pub const EMPTY: Chat = Chat { id: 0, app: 0, hh: 0, mm: 0, nlen: 0, mlen: 0, name: [0; NAME_MAX], msg: [0; TEXT_MAX], llen: 0, log: [0; LOG_MAX] };

    pub fn name(&self) -> &str {
        text(&self.name[..self.nlen as usize])
    }

    pub fn msg(&self) -> &str {
        text(&self.msg[..self.mlen as usize])
    }

    /// Every message kept for this chat, oldest first (the replies sent from the watch too).
    pub fn log(&self) -> impl Iterator<Item = &str> {
        self.entries().map(|(_, t)| t)
    }

    /// Every kept message, oldest first: `(true, text)` for a reply sent from the watch, `(false, text)` for one that came in.
    pub fn entries(&self) -> impl Iterator<Item = (bool, &str)> {
        let l = &self.log[..self.llen as usize];
        l.split(|&b| b == b'\n').filter(|e| !e.is_empty()).map(|e| match e.split_first() {
            Some((&MINE, rest)) => (true, text(rest)),
            _ => (false, text(e)),
        })
    }

    /// Adds the newest message to the log (not again if it is the same as the last one); the oldest fall off when full.
    fn remember(&mut self, body: &[u8]) {
        if body.is_empty() || self.log[..self.llen as usize].rsplit(|&b| b == b'\n').next() == Some(body) {
            return;
        }
        self.push_entry(body);
    }

    /// Keeps a reply sent from the watch (shown as a grey bubble of its own).
    pub fn remember_mine(&mut self, body: &[u8]) {
        let body = cut(&body[..body.len().min(MINE_MAX)]);
        if body.is_empty() {
            return;
        }
        let mut e = [0u8; MINE_MAX + 1];
        e[0] = MINE;
        e[1..=body.len()].copy_from_slice(body);
        self.push_entry(&e[..=body.len()]);
    }

    fn push_entry(&mut self, body: &[u8]) {
        while self.llen > 0 && self.llen as usize + 1 + body.len() > LOG_MAX {
            let l = self.llen as usize;
            let cut_at = self.log[..l].iter().position(|&b| b == b'\n').map_or(l, |i| i + 1);
            self.log.copy_within(cut_at..l, 0);
            self.llen = (l - cut_at) as u16;
        }
        let l = self.llen as usize;
        let at = if l > 0 { self.log[l] = b'\n'; l + 1 } else { 0 };
        self.log[at..at + body.len()].copy_from_slice(body);
        self.llen = (at + body.len()) as u16;
    }

    /// "9:05 a.m." for the time the message came.
    pub fn clock<'a>(&self, buf: &'a mut [u8; 10]) -> &'a str {
        let (h, m) = (self.hh.min(23), self.mm.min(59));
        let h12 = if h % 12 == 0 { 12 } else { h % 12 };
        let mut n = 0;
        if h12 >= 10 {
            buf[n] = b'0' + h12 / 10;
            n += 1;
        }
        buf[n] = b'0' + h12 % 10;
        buf[n + 1] = b':';
        buf[n + 2] = b'0' + m / 10;
        buf[n + 3] = b'0' + m % 10;
        n += 4;
        let suffix: &[u8] = if h < 12 { b" a.m." } else { b" p.m." };
        buf[n..n + 5].copy_from_slice(suffix);
        text(&buf[..n + 5])
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Effect {
    Ignored,
    /// The chats or the quick replies changed.
    Changed,
    /// The phone answered a reply: (chat, status).
    Result(u8, u8),
}

pub struct Model {
    pub chats: [Chat; MAX_CHATS],
    pub n: usize,
    quick: [[u8; QUICK_LEN]; QUICK_MAX],
    qlen: [u8; QUICK_MAX],
    pub nq: usize,
    /// Something from the phone has arrived since the app opened.
    pub heard: bool,
    /// The reply just sent from the watch, for a little while: the chat app puts it in the notification, so the phone
    /// sends it back as if it were a new message from the other side. That echo must not become a bubble of theirs.
    echo: [u8; MINE_MAX],
    echo_len: u8,
    echo_id: u8,
    echo_on: bool,
}

impl Model {
    pub const fn new() -> Model {
        Model { chats: [Chat::EMPTY; MAX_CHATS], n: 0, quick: [[0; QUICK_LEN]; QUICK_MAX], qlen: [0; QUICK_MAX], nq: 0, heard: false, echo: [0; MINE_MAX], echo_len: 0, echo_id: 0, echo_on: false }
    }

    /// A reply to chat `id` has just been sent: the phone may send it straight back as a message of that chat.
    pub fn arm_echo(&mut self, id: u8, body: &[u8]) {
        let body = cut(&body[..body.len().min(MINE_MAX)]);
        self.echo[..body.len()].copy_from_slice(body);
        (self.echo_len, self.echo_id, self.echo_on) = (body.len() as u8, id, !body.is_empty());
    }

    /// The reply was not sent, or long enough has passed: a message like it is a new one.
    pub fn disarm_echo(&mut self) {
        self.echo_on = false;
    }

    /// `body`, a message that came in for chat `id`, is my own reply coming back (the same text, or with the sender's
    /// name in front: "You: ok").
    fn is_echo(&self, id: u8, body: &[u8]) -> bool {
        let e = &self.echo[..self.echo_len as usize];
        self.echo_on && self.echo_id == id && !e.is_empty() && (body == e || (body.len() > e.len() + 2 && body.ends_with(e) && body[body.len() - e.len() - 2..][..2] == *b": "))
    }

    /// The quick replies the app has before the phone sends its own.
    pub fn set_defaults(&mut self) {
        for (i, t) in ["Ok", "On my way", "Thanks!"].iter().enumerate() {
            self.quick[i][..t.len()].copy_from_slice(t.as_bytes());
            self.qlen[i] = t.len() as u8;
        }
        self.nq = 3;
    }

    /// What the row under the text shows: the quick replies while nothing is typed, else words that continue the word being typed.
    pub fn items<'a>(&'a self, text: &[u8]) -> ([&'a str; 3], usize) {
        let mut out = [""; 3];
        if text.is_empty() {
            for i in 0..self.nq.min(3) {
                out[i] = self.quick(i);
            }
            return (out, self.nq.min(3));
        }
        let mut sug = [""; 3];
        let n = if text.last() == Some(&b' ') {
            crate::words::predict_next(text, &mut sug)
        } else {
            crate::words::suggest(crate::words::fragment(text), &mut sug)
        };
        out[..n].copy_from_slice(&sug[..n]);
        (out, n)
    }

    pub fn quick(&self, i: usize) -> &str {
        text(&self.quick[i][..self.qlen[i] as usize])
    }

    /// A reply to chat `id` went out: it joins that chat's messages as a bubble of mine.
    pub fn sent(&mut self, id: u8, body: &[u8]) {
        if let Some(i) = self.find(id) {
            self.chats[i].remember_mine(body);
        }
    }

    pub fn find(&self, id: u8) -> Option<usize> {
        self.chats[..self.n].iter().position(|c| c.id == id)
    }

    /// Replace a chat in place for a message arriving from the conversation currently on screen.
    /// It updates the visible thread without moving that thread to the front of the inbox.
    fn update_in_place(&mut self, mut c: Chat) {
        if let Some(i) = self.find(c.id) {
            let old = self.chats[i];
            if c.llen == 0 {
                (c.llen, c.log) = (old.llen, old.log);
            }
            let last = c.msg;
            let ml = c.mlen as usize;
            c.remember(&last[..ml]);
            self.chats[i] = c;
        } else {
            self.put_first(c);
        }
    }

    fn remove(&mut self, i: usize) {
        self.chats.copy_within(i + 1..self.n, i);
        self.n -= 1;
    }

    /// The chat comes first in the list; the oldest falls off when it is full.
    fn put_first(&mut self, c: Chat) {
        // one page per chat: the phone gives every chat its own id (names can look alike when letters are replaced by dots)
        let mut c = c;
        while let Some(i) = self.chats[..self.n].iter().position(|o| o.id == c.id) {
            // the messages that came before stay, so the second screen shows everything that was missed
            let old = self.chats[i];
            if c.llen == 0 {
                (c.llen, c.log) = (old.llen, old.log);
            }
            self.remove(i);
        }
        let last = c.msg;
        let ml = c.mlen as usize;
        c.remember(&last[..ml]);
        if self.n == MAX_CHATS {
            self.n -= 1;
        }
        self.chats.copy_within(0..self.n, 1);
        self.chats[0] = c;
        self.n += 1;
    }

    pub fn apply(&mut self, msg: &[u8]) -> Effect {
        if msg.len() < 3 || msg[..2] != PREFIX {
            return Effect::Ignored;
        }
        self.heard = true;
        let d = &msg[3..];
        match msg[2] {
            b'C' => {
                self.n = 0;
                Effect::Changed
            }
            b'L' | b'U' if d.len() >= 6 => {
                let nlen = (d[4] as usize).min(NAME_MAX).min(d.len() - 5);
                let rest = &d[5 + nlen..];
                let mlen = rest.len().min(TEXT_MAX);
                let mut c = Chat { id: d[0], app: d[1], hh: d[2], mm: d[3], nlen: nlen as u8, mlen: mlen as u8, ..Chat::EMPTY };
                // a cut that fell inside a letter is dropped, not shown as a question mark
                let name = cut(&d[5..5 + nlen]);
                let body = cut(&rest[..mlen]);
                if self.is_echo(d[0], body) {
                    self.echo_on = false; // once: a later message like it is theirs
                    return Effect::Ignored;
                }
                c.nlen = name.len() as u8;
                c.mlen = body.len() as u8;
                c.name[..name.len()].copy_from_slice(name);
                c.msg[..body.len()].copy_from_slice(body);
                if msg[2] == b'U' { self.update_in_place(c); } else { self.put_first(c); }
                Effect::Changed
            }
            b'D' if !d.is_empty() => match self.find(d[0]) {
                Some(i) => {
                    self.remove(i);
                    Effect::Changed
                }
                None => Effect::Ignored,
            },
            b'Q' if !d.is_empty() => {
                self.nq = 0;
                let mut at = 1;
                for _ in 0..(d[0] as usize).min(QUICK_MAX) {
                    let Some(&l) = d.get(at) else { break };
                    let l = (l as usize).min(QUICK_LEN);
                    let Some(t) = d.get(at + 1..at + 1 + l) else { break };
                    let t = cut(t);
                    self.quick[self.nq][..t.len()].copy_from_slice(t);
                    self.qlen[self.nq] = t.len() as u8;
                    self.nq += 1;
                    at += 1 + l;
                }
                Effect::Changed
            }
            b'R' if d.len() >= 2 => Effect::Result(d[0], d[1]),
            _ => Effect::Ignored,
        }
    }
}

impl Default for Model {
    fn default() -> Self {
        Model::new()
    }
}

/// `b` without a letter that was cut in half at its end.
fn cut(b: &[u8]) -> &[u8] {
    match core::str::from_utf8(b) {
        Ok(_) => b,
        Err(e) => &b[..e.valid_up_to()],
    }
}

/// How many messages a reply of `len` bytes takes.
pub fn pieces(len: usize) -> usize {
    len.div_ceil(CHUNK).max(1)
}

/// Piece `i` of the reply `text` for chat `id`, written to `out`; returns its length.
pub fn reply_piece(id: u8, text: &[u8], i: usize, out: &mut [u8; 128]) -> usize {
    let part = &text[(i * CHUNK).min(text.len())..((i + 1) * CHUNK).min(text.len())];
    let last = (i + 1) * CHUNK >= text.len();
    out[..3].copy_from_slice(b"r1x");
    out[3] = id;
    out[4] = (i as u8 & 15) | if last { 0x80 } else { 0 };
    out[5..5 + part.len()].copy_from_slice(part);
    5 + part.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_chat_is_one_page_and_the_ninth_drops_the_oldest() {
        let mut m = Model::new();
        m.apply(&chat(1, 1, "Group", "first"));
        m.apply(&chat(1, 1, "Group", "second")); // same chat (same id)
        m.apply(&chat(3, 2, "Group", "other app"));
        m.apply(&chat(4, 1, "Group", "same name, other chat")); // names can look alike (Arabic names become dots)
        assert_eq!(m.n, 3);
        assert_eq!(m.chats[2].msg(), "second");
        let mut m = Model::new();
        for i in 0..9u8 {
            m.apply(&chat(i, 1, &std::format!("P{i}"), "x"));
        }
        assert_eq!(m.n, MAX_CHATS);
        assert_eq!(m.chats[0].name(), "P8");
        assert_eq!(m.chats[MAX_CHATS - 1].name(), "P1");
    }

    fn chat(id: u8, app: u8, name: &str, msg: &str) -> std::vec::Vec<u8> {
        let mut v = std::vec![b'r', b'1', b'L', id, app, 10, 20, name.len() as u8];
        v.extend_from_slice(name.as_bytes());
        v.extend_from_slice(msg.as_bytes());
        v
    }

    #[test]
    fn a_focused_chat_update_does_not_move_its_inbox_card() {
        let mut m = Model::new();
        m.apply(&chat(1, 1, "Jane", "first"));
        m.apply(&chat(2, 1, "Bob", "other"));
        let mut update = chat(1, 1, "Jane", "new message");
        update[2] = b'U';
        assert_eq!(m.apply(&update), Effect::Changed);
        assert_eq!(m.chats[0].name(), "Bob");
        assert_eq!(m.chats[1].name(), "Jane");
        assert_eq!(m.chats[1].msg(), "new message");
        assert_eq!(m.chats[1].log().last(), Some("new message"));
    }

    #[test]
    fn a_chat_comes_in_first_and_an_update_moves_it_up() {
        let mut m = Model::new();
        assert_eq!(m.apply(&chat(1, 1, "Jane", "hi")), Effect::Changed);
        m.apply(&chat(2, 3, "Bob", "yo"));
        assert_eq!((m.n, m.chats[0].name(), m.chats[1].name()), (2, "Bob", "Jane"));
        m.apply(&chat(1, 1, "Jane", "are you there?"));
        assert_eq!((m.n, m.chats[0].name(), m.chats[0].msg(), m.chats[1].name()), (2, "Jane", "are you there?", "Bob"));
        let mut b = [0; 10];
        assert_eq!(m.chats[0].clock(&mut b), "10:20 a.m.");
        let mut c = Chat::EMPTY;
        (c.hh, c.mm) = (0, 5);
        assert_eq!(c.clock(&mut b), "12:05 a.m.");
        (c.hh, c.mm) = (13, 40);
        assert_eq!(c.clock(&mut b), "1:40 p.m.");
        (c.hh, c.mm) = (12, 0);
        assert_eq!(c.clock(&mut b), "12:00 p.m.");
        assert!(m.heard);
    }

    #[test]
    fn the_list_holds_eight_and_chats_can_go() {
        let mut m = Model::new();
        for i in 0..10u8 {
            m.apply(&chat(i, 1, &std::format!("n{i}"), "y"));
        }
        assert_eq!((m.n, m.chats[0].id, m.chats[7].id), (8, 9, 2));
        assert_eq!(m.apply(&[b'r', b'1', b'D', 9]), Effect::Changed);
        assert_eq!((m.n, m.chats[0].id), (7, 8));
        assert_eq!(m.apply(&[b'r', b'1', b'D', 77]), Effect::Ignored);
        m.apply(b"r1C");
        assert_eq!(m.n, 0);
    }

    #[test]
    fn other_apps_messages_and_short_ones_are_ignored() {
        let mut m = Model::new();
        assert_eq!(m.apply(b"n1S"), Effect::Ignored);
        assert_eq!(m.apply(b"r1"), Effect::Ignored);
        assert!(!m.heard);
        assert_eq!(m.apply(b"r1L\x01"), Effect::Ignored); // a short one from the phone still shows it is there
        assert!(m.heard);
    }

    #[test]
    fn a_letter_cut_in_half_is_dropped() {
        let mut m = Model::new();
        let mut v = chat(1, 1, "Jane", "ab");
        v.extend_from_slice(&"é".as_bytes()[..1]);
        m.apply(&v);
        assert_eq!(m.chats[0].msg(), "ab");
    }

    #[test]
    fn quick_replies_and_results() {
        let mut m = Model::new();
        m.apply(b"r1Q\x02\x02Ok\x0bOn my way!!");
        assert_eq!((m.nq, m.quick(0), m.quick(1)), (2, "Ok", "On my way!!"));
        assert_eq!(m.apply(&[b'r', b'1', b'R', 4, 1]), Effect::Result(4, 1));
    }

    #[test]
    fn a_reply_is_cut_into_pieces() {
        let text = [b'a'; 150];
        assert_eq!((pieces(0), pieces(100), pieces(101), pieces(200)), (1, 1, 2, 2));
        let mut out = [0u8; 128];
        let n = reply_piece(7, &text, 0, &mut out);
        assert_eq!((n, &out[..3], out[3], out[4]), (105, &b"r1x"[..], 7, 0));
        let n = reply_piece(7, &text, 1, &mut out);
        assert_eq!((n, out[4]), (55, 0x81));
        let n = reply_piece(7, b"hello", 0, &mut out);
        assert_eq!((n, out[4], &out[5..10]), (10, 0x80, &b"hello"[..]));
        // exactly one full piece is a single, last piece
        let n = reply_piece(7, &text[..100], 0, &mut out);
        assert_eq!((n, out[4]), (105, 0x80));
    }
}

#[cfg(test)]
mod history_tests {
    use super::*;
    fn l(id: u8, name: &str, msg: &str) -> Vec<u8> {
        let mut v = vec![b'r', b'1', b'L', id, 1, 9, 5, name.len() as u8];
        v.extend(name.as_bytes());
        v.extend(msg.as_bytes());
        v
    }
    #[test]
    fn new_messages_pile_up_in_their_chat() {
        let mut m = Model::new();
        for t in ["one", "two", "two", "three"] {
            m.apply(&l(7, "Sam", t));
        }
        assert_eq!(m.n, 1);
        assert_eq!(m.chats[0].msg(), "three");
        assert_eq!(m.chats[0].log().collect::<Vec<_>>(), ["one", "two", "three"]);
        let long = "x".repeat(90);
        for _ in 0..8 {
            m.apply(&l(7, "Sam", &long));
            m.apply(&l(7, "Sam", "y"));
        }
        assert!(m.chats[0].llen as usize <= LOG_MAX);
        assert_eq!(m.chats[0].log().last(), Some("y"));
        m.apply(b"r1C");
        m.apply(&l(7, "Sam", "fresh"));
        assert_eq!(m.chats[0].log().collect::<Vec<_>>(), ["fresh"]);
    }

    #[test]
    fn sent_replies_are_kept_as_mine() {
        let mut m = Model::new();
        m.apply(&l(7, "Sam", "hi"));
        m.sent(7, b"hello");
        m.sent(7, b"hello");
        m.apply(&l(7, "Sam", "ok"));
        let e: Vec<_> = m.chats[0].entries().collect();
        assert_eq!(e, [(false, "hi"), (true, "hello"), (true, "hello"), (false, "ok")]);
        assert_eq!(m.chats[0].log().collect::<Vec<_>>(), ["hi", "hello", "hello", "ok"]);
    }

    #[test]
    fn my_reply_coming_back_from_the_phone_is_not_a_message_from_them() {
        let mut m = Model::new();
        m.apply(&l(7, "Sam", "hi"));
        m.arm_echo(7, b"yo");
        // the chat app shows my reply in the notification; the phone sends it as a new message: not theirs
        assert_eq!(m.apply(&l(7, "Sam", "yo")), Effect::Ignored);
        m.sent(7, b"yo");
        assert_eq!(m.chats[0].entries().collect::<Vec<_>>(), [(false, "hi"), (true, "yo")]);
        // it only counts once: the same words from them later are theirs
        m.apply(&l(7, "Sam", "yo"));
        assert_eq!(m.chats[0].entries().last(), Some((false, "yo")));
        // with the name in front, and when the echo comes after the phone said it was sent
        m.arm_echo(7, b"ok");
        m.sent(7, b"ok");
        assert_eq!(m.apply(&l(7, "Sam", "You: ok")), Effect::Ignored);
        assert_eq!(m.chats[0].entries().last(), Some((true, "ok")));
        // another chat, a different text, and a reply that was disarmed are all theirs
        m.arm_echo(7, b"hey");
        assert_eq!(m.apply(&l(8, "Al", "hey")), Effect::Changed);
        assert_eq!(m.apply(&l(7, "Sam", "hey there")), Effect::Changed);
        m.arm_echo(7, b"late");
        m.disarm_echo();
        assert_eq!(m.apply(&l(7, "Sam", "late")), Effect::Changed);
    }
}
