//! What a touch was, from where the finger went down and came up.

/// A touch that moved this far (and mostly in one direction) is a swipe.
pub const SWIPE_PX: i32 = 60;
/// A touch that stayed within this far is a tap or a hold.
pub const TAP_PX: i32 = 28;
/// A touch held this long (and still) is a hold.
pub const HOLD_MS: u32 = 600;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gesture {
    None,
    Tap,
    Hold,
    SwipeUp,
    SwipeDown,
}

/// `dx`, `dy`: how far the finger went from where it came down (right and down are positive); `held_ms`: for how long.
/// Sideways swipes are not read here: a right swipe is the watch's own "back" (`BUTTON`).
pub fn classify(dx: i32, dy: i32, held_ms: u32) -> Gesture {
    let (ax, ay) = (dx.abs(), dy.abs());
    if ay >= SWIPE_PX && ay >= 2 * ax {
        return if dy > 0 { Gesture::SwipeDown } else { Gesture::SwipeUp };
    }
    if ax.max(ay) <= TAP_PX {
        return if held_ms >= HOLD_MS { Gesture::Hold } else { Gesture::Tap };
    }
    Gesture::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn taps_holds_and_vertical_swipes() {
        assert_eq!(classify(3, -4, 120), Gesture::Tap);
        assert_eq!(classify(0, 0, 700), Gesture::Hold);
        assert_eq!(classify(10, -120, 200), Gesture::SwipeUp);
        assert_eq!(classify(-20, 90, 300), Gesture::SwipeDown);
        assert_eq!(classify(150, 20, 200), Gesture::None); // sideways: the watch's back
        assert_eq!(classify(40, 45, 100), Gesture::None);
    }
}
