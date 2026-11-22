#![cfg_attr(any(target_os = "none", wp2_sim), no_main, cfg_attr(target_os = "none", no_std))]

//! Watch Replies: the messages that come to the phone (WhatsApp, Messenger, Telegram), as cards on the watch, and a
//! keyboard to answer them. The phone side is the Watch Menu app. Protocol: `proto.rs`.

mod font;
mod gesture;
#[rustfmt::skip]
mod fonts;
mod icons;
mod keys;
mod proto;
mod view;
mod words;

pub use cmf_wp2_ui::{C, SIZE};

#[cfg(not(any(target_os = "none", wp2_sim)))]
fn main() {}

#[cfg(any(target_os = "none", wp2_sim))]
mod device {
    use crate::gesture::{Gesture, classify};
    use crate::keys::{self, Cell, Key, Page};
    use crate::proto::{self, Effect, Model, REPLY_MAX};
    use crate::view;
    use cmf_wp2_app::{Api, BUTTON, CROWN, MSG_MAX, PAUSE, RESUME, TOUCH_DOWN, TOUCH_UP};

    cmf_wp2_app::entry!(main);
    cmf_wp2_app::app_icon!();

    /// Say hello to the phone again this often while it has not answered (doubling up to a minute).
    const HELLO_MS: u32 = 3000;
    /// A reply that the phone does not answer in this long has failed.
    const SEND_MS: u32 = 15000;
    /// "Sent" stays this long, "Not sent" a little longer.
    const SENT_MS: u32 = 1400;
    const FAILED_MS: u32 = 2600;
    const GAP_MS: u32 = 20;

    static mut MODEL: Model = Model::new();

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Screen {
        Inbox,
        Detail,
        Keyboard,
        Sending,
        Sent,
        Failed,
    }

    /// What a finger came down on in the keyboard.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Target {
        Key(Cell),
        Chip(usize),
        Send,
    }

    // true on RESUME, false on BUTTON
    fn paused(api: &Api) -> bool {
        loop {
            match api.event(u32::MAX).map(|e| e.kind) {
                Some(RESUME) => return true,
                Some(BUTTON) => return false,
                _ => {}
            }
        }
    }

    struct Draft {
        text: [u8; REPLY_MAX],
        len: usize,
        to: [u8; 40],
        to_len: usize,
        chat: u8,
        page: Page,
        shift: bool,
    }

    impl Draft {
        fn bytes(&self) -> &[u8] {
            &self.text[..self.len]
        }

        fn to(&self) -> &str {
            core::str::from_utf8(&self.to[..self.to_len]).unwrap_or("")
        }

        /// Starts a reply to `c`.
        fn open(&mut self, c: &proto::Chat) {
            let b = c.name();
            let mut n = 0;
            let take = b.len().min(self.to.len());
            // not in the middle of a letter
            let take = (0..=take).rev().find(|&i| b.is_char_boundary(i)).unwrap_or(0);
            self.to[n..n + take].copy_from_slice(&b.as_bytes()[..take]);
            n += take;
            (self.to_len, self.chat, self.len, self.page, self.shift) = (n, c.id, 0, Page::Letters, true);
        }

        fn push(&mut self, ch: u8) -> bool {
            if self.len >= REPLY_MAX {
                return false;
            }
            self.text[self.len] = ch;
            self.len += 1;
            true
        }
    }

    fn main(api: &'static Api) -> i32 {
        // SAFETY: the app has one thread and this is the only reference.
        let m = unsafe { &mut *core::ptr::addr_of_mut!(MODEL) };
        m.set_defaults();
        api.own_back(true); // a right swipe or the crown press is "back" here: keyboard -> messages -> leave
        let mut d = Draft { text: [0; REPLY_MAX], len: 0, to: [0; 40], to_len: 0, chat: 0, page: Page::Letters, shift: true };
        let mut screen = Screen::Inbox;
        let mut top = 0usize;
        let mut dirty = true;
        let mut drawn_end = 0u32;
        let mut hello_at: Option<u32> = None;
        let mut tries = 0u32;
        let mut touch0: Option<(u32, i32, i32)> = None;
        let mut card_down: Option<usize> = None;
        let mut det_chat = 0u8; // the chat on the full-message screen
        let mut dscroll = 0usize;
        let mut rep_down = false;
        let mut target: Option<Target> = None;
        let mut until: Option<u32> = None; // when a status page ends (Sent, Failed) or a reply times out (Sending)
        let mut keep_on = false;
        loop {
            let now = api.now();
            if !m.heard && hello_at.is_none_or(|t| now.wrapping_sub(t) >= (HELLO_MS << tries.min(4)).min(60_000)) {
                api.send(&proto::hello()); // -107 while the phone is not listening: asked again
                hello_at = Some(now);
                tries += 1;
            }
            let on = matches!(screen, Screen::Keyboard | Screen::Sending);
            if on != keep_on {
                keep_on = on;
                api.keep_screen_on(on);
            }
            if let Some(t) = until.filter(|&t| now.wrapping_sub(t) as i32 >= 0) {
                until = None;
                let _ = t;
                match screen {
                    Screen::Sending => {
                        screen = Screen::Failed;
                        until = Some(now.wrapping_add(FAILED_MS));
                    }
                    Screen::Sent => screen = Screen::Inbox,
                    Screen::Failed => screen = Screen::Keyboard,
                    _ => {}
                }
                dirty = true;
            }
            if dirty && now.wrapping_sub(drawn_end) >= GAP_MS {
                let mut s = api;
                match screen {
                    Screen::Inbox => {
                        top = top.min(view::max_top(m.n));
                        if m.n == 0 {
                            view::idle(&mut s, m.heard);
                        } else {
                            view::page(&mut s, &m.chats[top], top, m.n, card_down.is_some());
                        }
                    }
                    Screen::Detail => match m.find(det_chat) {
                        Some(i) => {
                            dscroll = dscroll.min(view::detail_max(&m.chats[i]));
                            view::detail(&mut s, &m.chats[i], dscroll, rep_down);
                        }
                        None => screen = Screen::Inbox, // the chat is gone: drawn as the inbox on the next round
                    },
                    Screen::Keyboard => view::keyboard(&mut s, m, d.to(), d.bytes(), d.page, d.shift),
                    Screen::Sending => view::status(&mut s, 0, "Sending…", ""),
                    Screen::Sent => view::status(&mut s, 1, "Sent", d.to()),
                    Screen::Failed => view::status(&mut s, 2, "Not sent", "Is Watch Menu open?"),
                }
                api.show();
                (dirty, drawn_end) = (false, api.now());
                continue;
            }
            let mut wait = if dirty { GAP_MS.saturating_sub(now.wrapping_sub(drawn_end)).max(1) } else { 1000 };
            if let Some(t) = until {
                wait = wait.min((t.wrapping_sub(now) as i32).max(1) as u32);
            }
            if !m.heard {
                wait = wait.min(500);
            }
            let mut ev = api.event(wait);
            while let Some(e) = ev {
                match e {
                    e if e.kind == BUTTON => match screen {
                        Screen::Inbox => return 0,
                        Screen::Keyboard => {
                            (screen, until, dirty) = (if m.find(det_chat).is_some() { Screen::Detail } else { Screen::Inbox }, None, true);
                            target = None;
                        }
                        Screen::Sending => {
                            (screen, until, dirty) = (Screen::Inbox, None, true);
                            target = None;
                        }
                        _ => (screen, until, dirty) = (Screen::Inbox, None, true),
                    },
                    e if e.kind == CROWN && screen == Screen::Detail => {
                        if let Some(i) = m.find(det_chat) {
                            let max = view::detail_max(&m.chats[i]);
                            let next = if e.delta > 0 { (dscroll + 1).min(max) } else { dscroll.saturating_sub(1) };
                            if next != dscroll {
                                (dscroll, dirty) = (next, true);
                            }
                        }
                    }
                    e if e.kind == CROWN && screen == Screen::Inbox => {
                        let next = if e.delta > 0 { top + 1 } else { top.saturating_sub(1) };
                        let next = next.min(view::max_top(m.n));
                        if next != top {
                            (top, dirty) = (next, true);
                        }
                    }
                    e if e.kind == TOUCH_DOWN => {
                        touch0 = Some((api.now(), e.x as i32, e.y as i32));
                        let (x, y) = (e.x as i32, e.y as i32);
                        match screen {
                            Screen::Inbox => {
                                card_down = (m.n > 0 && view::page_at(x, y)).then_some(top);
                                dirty |= card_down.is_some();
                            }
                            Screen::Detail => {
                                rep_down = view::reply_at(x, y);
                                dirty |= rep_down;
                            }
                            Screen::Keyboard => {
                                target = if view::send_at(x, y) {
                                    Some(Target::Send)
                                } else if view::chip_at(&m.items(d.bytes()).0[..m.items(d.bytes()).1], x, y).is_some() {
                                    view::chip_at(&m.items(d.bytes()).0[..m.items(d.bytes()).1], x, y).map(Target::Chip)
                                } else {
                                    keys::key_at(d.page, x, y).map(Target::Key)
                                };
                                if let Some(Target::Key(c)) = target {
                                    let mut s = api;
                                    view::key(&mut s, &c, d.page, d.shift, true);
                                    api.show();
                                    drawn_end = api.now();
                                }
                            }
                            _ => {}
                        }
                    }
                    e if e.kind == TOUCH_UP => {
                        let Some((t, x0, y0)) = touch0.take() else {
                            ev = api.event(0);
                            continue;
                        };
                        let (x, y) = (e.x as i32, e.y as i32);
                        let held = api.now().wrapping_sub(t);
                        match screen {
                            Screen::Inbox => {
                                let down = card_down.take();
                                dirty |= down.is_some();
                                match classify(x - x0, y - y0, held) {
                                    Gesture::SwipeUp => {
                                        top = (top + 1).min(view::max_top(m.n));
                                        dirty = true;
                                    }
                                    Gesture::SwipeDown => {
                                        top = top.saturating_sub(1);
                                        dirty = true;
                                    }
                                    Gesture::Tap | Gesture::Hold => {
                                        if let Some(i) = down.filter(|&i| i < m.n && view::page_at(x, y)) {
                                            // the whole message first; reply is on that screen
                                            (det_chat, dscroll) = (m.chats[i].id, view::detail_max(&m.chats[i]));
                                            (screen, dirty) = (Screen::Detail, true);
                                        }
                                    }
                                    Gesture::None => {}
                                }
                            }
                            Screen::Detail => {
                                let down = core::mem::take(&mut rep_down);
                                dirty |= down;
                                let max = m.find(det_chat).map_or(0, |i| view::detail_max(&m.chats[i]));
                                match classify(x - x0, y - y0, held) {
                                    Gesture::SwipeUp => {
                                        dscroll = (dscroll + 3).min(max);
                                        dirty = true;
                                    }
                                    Gesture::SwipeDown => {
                                        dscroll = dscroll.saturating_sub(3);
                                        dirty = true;
                                    }
                                    Gesture::Tap | Gesture::Hold => {
                                        if let (true, true, Some(i)) = (down, view::reply_at(x, y), m.find(det_chat)) {
                                            d.open(&m.chats[i]);
                                            (screen, dirty) = (Screen::Keyboard, true);
                                        }
                                    }
                                    Gesture::None => {}
                                }
                            }
                            Screen::Keyboard => {
                                let down = target.take();
                                // the key is drawn as it was, whatever the touch did
                                if let Some(Target::Key(c)) = down {
                                    let mut s = api;
                                    view::key(&mut s, &c, d.page, d.shift, false);
                                    api.show();
                                    drawn_end = api.now();
                                }
                                let same = match down {
                                    Some(Target::Key(c)) => keys::key_at(d.page, x, y).is_some_and(|u| u.key == c.key),
                                    Some(Target::Chip(i)) => view::chip_at(&m.items(d.bytes()).0[..m.items(d.bytes()).1], x, y) == Some(i),
                                    Some(Target::Send) => view::send_at(x, y),
                                    None => false,
                                };
                                if !same {
                                    ev = api.event(0);
                                    continue;
                                }
                                let mut changed = false;
                                match down {
                                    Some(Target::Key(c)) => match c.key {
                                        Key::Ch(_) | Key::Space => {
                                            if let Some(ch) = keys::typed(c.key, d.shift && d.page == Page::Letters) {
                                                changed = d.push(ch);
                                            }
                                            let shift = d.page == Page::Letters && keys::sentence_start(d.bytes());
                                            if shift != d.shift {
                                                d.shift = shift;
                                                let mut s = api;
                                                view::all_keys(&mut s, d.page, d.shift);
                                            }
                                        }
                                        Key::Back => {
                                            if held >= crate::gesture::HOLD_MS {
                                                d.len = 0; // holding backspace clears the lot
                                            } else {
                                                d.len = d.len.saturating_sub(1);
                                            }
                                            changed = true;
                                            let shift = d.page == Page::Letters && keys::sentence_start(d.bytes());
                                            if shift != d.shift {
                                                d.shift = shift;
                                                let mut s = api;
                                                view::all_keys(&mut s, d.page, d.shift);
                                            }
                                        }
                                        Key::Shift => {
                                            d.shift = !d.shift;
                                            let mut s = api;
                                            view::all_keys(&mut s, d.page, d.shift);
                                            api.show();
                                        }
                                        Key::Page => {
                                            d.page = if d.page == Page::Letters { Page::Numbers } else { Page::Letters };
                                            d.shift = d.page == Page::Letters && keys::sentence_start(d.bytes());
                                            let mut s = api;
                                            view::keyboard(&mut s, m, d.to(), d.bytes(), d.page, d.shift);
                                            api.show();
                                        }
                                    },
                                    Some(Target::Chip(i)) => {
                                        if d.len == 0 {
                                            let q = m.quick(i).as_bytes();
                                            let n = q.len().min(REPLY_MAX);
                                            d.text[..n].copy_from_slice(&q[..n]);
                                            d.len = n;
                                        } else {
                                            // finish the word being typed, and a space after it
                                            let (items, _) = m.items(d.bytes());
                                            let w = items[i].as_bytes();
                                            let fl = crate::words::fragment(d.bytes()).len();
                                            let cap = d.bytes()[d.len - fl..].first().is_some_and(|b| b.is_ascii_uppercase());
                                            if d.len - fl + w.len() + 1 <= REPLY_MAX {
                                                d.len -= fl;
                                                for (k, &b) in w.iter().enumerate() {
                                                    d.text[d.len] = if k == 0 && cap { b.to_ascii_uppercase() } else { b };
                                                    d.len += 1;
                                                }
                                                d.text[d.len] = b' ';
                                                d.len += 1;
                                            }
                                        }
                                        changed = true;
                                        d.shift = false;
                                        dirty = true; // the keys change case too
                                    }
                                    Some(Target::Send) if d.len > 0 => {
                                        let mut buf = [0u8; MSG_MAX];
                                        let mut ok = true;
                                        for i in 0..proto::pieces(d.len) {
                                            let n = proto::reply_piece(d.chat, d.bytes(), i, &mut buf);
                                            ok &= api.send(&buf[..n]) >= 0;
                                        }
                                        if ok {
                                            (screen, until) = (Screen::Sending, Some(api.now().wrapping_add(SEND_MS)));
                                        } else {
                                            (screen, until) = (Screen::Failed, Some(api.now().wrapping_add(FAILED_MS)));
                                        }
                                        dirty = true;
                                    }
                                    _ => {}
                                }
                                if changed && !dirty {
                                    let mut s = api;
                                    view::text_changed(&mut s, m, d.to(), d.bytes());
                                    api.show();
                                    drawn_end = api.now();
                                }
                            }
                            _ => {}
                        }
                    }
                    e if e.kind == PAUSE => {
                        (touch0, card_down, target, rep_down) = (None, None, None, false);
                        if !paused(api) {
                            return 0;
                        }
                        // messages were refused meanwhile: ask for everything again
                        (m.heard, hello_at, tries, dirty) = (false, None, 0, true);
                    }
                    _ => {}
                }
                ev = api.event(0);
            }
            let mut buf = [0u8; MSG_MAX];
            while let Some(msg) = api.recv(&mut buf) {
                match m.apply(msg) {
                    Effect::Ignored => {}
                    Effect::Changed => {
                        // the keyboard is not redrawn under the person's fingers, except for its quick replies
                        if screen == Screen::Inbox || screen == Screen::Detail {
                            dirty = true;
                        }
                        // answering a chat usually makes its notification go away: the chat vanishing means it worked
                        if screen == Screen::Sending && m.find(d.chat).is_none() {
                            (screen, until, dirty) = (Screen::Sent, Some(api.now().wrapping_add(SENT_MS)), true);
                        }
                    }
                    Effect::Result(id, status) => {
                        if screen == Screen::Sending && id == d.chat {
                            if status == 0 {
                                (screen, until) = (Screen::Sent, Some(api.now().wrapping_add(SENT_MS)));
                            } else {
                                (screen, until) = (Screen::Failed, Some(api.now().wrapping_add(FAILED_MS)));
                            }
                            dirty = true;
                        }
                    }
                }
                if m.heard && screen == Screen::Inbox && m.n == 0 {
                    dirty = true;
                }
                tries = 0;
            }
        }
    }
}
