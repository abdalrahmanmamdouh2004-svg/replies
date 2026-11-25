#![cfg_attr(any(target_os = "none", wp2_sim), no_main, cfg_attr(target_os = "none", no_std))]

//! Watch Remote: a pad of 4 arrows, a centre button and a small mode button. Every press is sent to the phone
//! (the Watch Menu app), which turns it into Bluetooth keyboard/mouse input for a PC. The phone decides what a
//! press does, so shortcuts are changed on the phone.
//!
//! Watch -> phone messages (all start with `c1`):
//!   `c1h` ver            hello: the app is open
//!   `c1f`                heartbeat while open (the phone treats 9 s of silence as closed)
//!   `c1z`                the app is closing
//!   `c1k` button gesture a press. button: 1 up 2 down 3 left 4 right 5 centre 6 mode.
//!                        gesture: 0 tap, 1 long press, 2 double tap, 3 triple tap (centre and mode only)

#[cfg(not(any(target_os = "none", wp2_sim)))]
fn main() {}

#[cfg(any(target_os = "none", wp2_sim))]
mod device {
    use cmf_wp2_app::{Api, BUTTON, PAUSE, RESUME, TOUCH_DOWN, TOUCH_UP};
    use cmf_wp2_ui::color::{BLACK, WHITE};
    use cmf_wp2_ui::draw::disc;
    use cmf_wp2_ui::{C, Rgb, SIZE, Surface};

    cmf_wp2_app::entry!(main);
    cmf_wp2_app::app_icon!();

    const HEARTBEAT_MS: u32 = 2500;
    const LONG_MS: u32 = 500;
    const MULTI_MS: u32 = 300;

    const FACE: Rgb = Rgb(52, 56, 61);
    const DOWN: Rgb = Rgb(120, 140, 200);
    const BLUE: Rgb = Rgb(138, 180, 248);

    /// (button id, centre x, centre y, radius, label)
    const PADS: [(u8, i32, i32, i32, &str); 6] = [
        (1, C, C - 140, 46, "^"),
        (2, C, C + 140, 46, "v"),
        (3, C - 140, C, 46, "<"),
        (4, C + 140, C, 46, ">"),
        (5, C, C, 66, "OK"),
        (6, C, SIZE - 36, 24, "M"),
    ];

    fn hit(x: i32, y: i32) -> Option<u8> {
        PADS.iter()
            .find(|p| {
                let (dx, dy) = (x - p.1, y - p.2);
                dx * dx + dy * dy <= (p.3 + 8) * (p.3 + 8)
            })
            .map(|p| p.0)
    }

    fn draw(api: &Api, pressed: Option<u8>) {
        let mut s = api;
        Surface::rect(&mut s, 0, 0, SIZE, SIZE, BLACK.rgb565());
        for p in PADS {
            let on = pressed == Some(p.0);
            disc(&mut s, p.1, p.2, p.3, if on { DOWN } else { FACE }, BLACK);
            let w = p.4.len() as i32 * 16;
            Surface::text(&mut s, p.1 - w / 2, p.2 - 8, p.4, if p.0 == 5 { BLUE.rgb565() } else { WHITE.rgb565() });
        }
        api.show();
    }

    fn key(api: &Api, button: u8, gesture: u8) {
        let _ = api.send(&[b'c', b'1', b'k', button, gesture]);
    }

    fn main(api: &'static Api) -> i32 {
        api.own_back(true);
        api.keep_screen_on(true);
        let _ = api.send(&[b'c', b'1', b'h', 1]);
        let mut beat = api.now();
        let mut pressed: Option<u8> = None;
        let mut down_at = 0u32;
        // pending multi-tap on centre / mode: (button, taps so far, deadline)
        let mut multi: Option<(u8, u8, u32)> = None;
        draw(api, None);
        loop {
            let now = api.now();
            if now.wrapping_sub(beat) >= HEARTBEAT_MS {
                let _ = api.send(&[b'c', b'1', b'f']);
                beat = now;
            }
            if let Some((b, n, t)) = multi {
                if now.wrapping_sub(t) as i32 >= 0 {
                    key(api, b, match n { 1 => 0, 2 => 2, _ => 3 });
                    multi = None;
                }
            }
            let mut wait = HEARTBEAT_MS.saturating_sub(now.wrapping_sub(beat)).max(1);
            if let Some((_, _, t)) = multi {
                wait = wait.min((t.wrapping_sub(now) as i32).max(1) as u32);
            }
            let Some(e) = api.event(wait) else { continue };
            match e.kind {
                k if k == BUTTON => {
                    let _ = api.send(&[b'c', b'1', b'z']);
                    return 0;
                }
                k if k == RESUME => draw(api, None),
                k if k == PAUSE => {}
                k if k == TOUCH_DOWN => {
                    pressed = hit(e.x as i32, e.y as i32);
                    down_at = api.now();
                    if pressed.is_some() {
                        draw(api, pressed);
                    }
                }
                k if k == TOUCH_UP => {
                    if let Some(b) = pressed.take() {
                        let long = api.now().wrapping_sub(down_at) >= LONG_MS;
                        if b <= 4 {
                            key(api, b, long as u8);
                        } else if long {
                            multi = None;
                            key(api, b, 1);
                        } else {
                            // a tap on another button first sends the earlier pending taps
                            let n = match multi {
                                Some((pb, n, _)) if pb == b => n + 1,
                                Some((pb, n, _)) => {
                                    key(api, pb, match n { 1 => 0, 2 => 2, _ => 3 });
                                    1
                                }
                                None => 1,
                            };
                            multi = Some((b, n, api.now().wrapping_add(MULTI_MS)));
                        }
                        draw(api, None);
                    }
                }
                _ => {}
            }
        }
    }
}
