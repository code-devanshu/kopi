//! The hybrid logical clock that stamps every synced change. See `docs/SYNC_DESIGN.md`.
//!
//! An HLC is Unix milliseconds in the high 48 bits and a counter in the low 16. It follows wall
//! time but never goes backwards, and it moves past every HLC it receives, so a change made
//! after seeing another always sorts after it, even when the computers' clocks disagree.

use std::sync::{Mutex, PoisonError};

/// How far ahead of local time a received HLC may be. Anything later is clamped to this, so
/// one computer with a wrong clock can't push what it touches to the top for days.
pub const MAX_DRIFT_MS: i64 = 5 * 60 * 1000;

/// The HLC at the start of Unix millisecond `ms`.
pub const fn from_ms(ms: i64) -> i64 {
    ms << 16
}

/// The Unix milliseconds of an HLC.
pub const fn to_ms(hlc: i64) -> i64 {
    hlc >> 16
}

#[derive(Debug)]
pub(crate) struct Clock {
    last: Mutex<i64>,
}

impl Clock {
    /// A clock that continues after `last`, the highest HLC already stored.
    pub(crate) fn new(last: i64) -> Self {
        Self {
            last: Mutex::new(last),
        }
    }

    /// The HLC for a change made now: wall time, or one past the last HLC if that's later.
    pub(crate) fn tick(&self, now_ms: i64) -> i64 {
        let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
        *last = (*last + 1).max(from_ms(now_ms));
        *last
    }

    /// Takes in a received HLC and returns it, clamped to [`MAX_DRIFT_MS`] ahead of `now_ms`.
    /// The clock moves past it, so the next local change sorts after it.
    pub(crate) fn observe(&self, hlc: i64, now_ms: i64) -> i64 {
        let hlc = clamp(hlc, now_ms);
        let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
        *last = (*last).max(hlc);
        hlc
    }
}

/// `hlc`, or the latest HLC [`MAX_DRIFT_MS`] ahead of `now_ms` if it's later than that.
pub(crate) fn clamp(hlc: i64, now_ms: i64) -> i64 {
    hlc.clamp(0, from_ms(now_ms + MAX_DRIFT_MS) | 0xFFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn follows_wall_time_and_never_goes_back() {
        let clock = Clock::new(0);
        let a = clock.tick(1_000);
        assert_eq!(a, from_ms(1_000));
        let b = clock.tick(1_000);
        assert_eq!(b, a + 1);
        // The wall clock went back a second.
        let c = clock.tick(0);
        assert_eq!(c, b + 1);
        let d = clock.tick(2_000);
        assert_eq!(d, from_ms(2_000));
        assert_eq!(to_ms(d), 2_000);
    }

    #[test]
    fn moves_past_what_it_receives() {
        let clock = Clock::new(0);
        let remote = from_ms(5_000) + 7;
        assert_eq!(clock.observe(remote, 4_000), remote);
        assert_eq!(clock.tick(4_000), remote + 1);
    }

    #[test]
    fn clamps_far_future_clocks() {
        let clock = Clock::new(0);
        let now = 1_000_000;
        let far = from_ms(now + 24 * 60 * 60 * 1000);
        let clamped = clock.observe(far, now);
        assert_eq!(to_ms(clamped), now + MAX_DRIFT_MS);
        assert!(clock.tick(now) > clamped);
        assert_eq!(clamp(-5, now), 0);
    }
}
