//! # Rate limiting module.
//!
//! This module contains implementation
//! of [exponential rate limiting](https://dotat.at/@/2024-09-02-ewma.html).
//! Implementation is simplified to only use one variable (`next_time`) to store the state.
//! Its primary use is preventing Delta Chat from sending too many messages, especially automatic,
//! such as read receipts.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct Ratelimit {
    /// Next time we are allowed to send, i.e. when measured rate (number of messages sent within
    /// the window) drops down to the limit `quota`.
    ///
    /// Measured in seconds since unix epoch.
    next_time: SystemTime,

    /// Time window size.
    window: f64,

    /// Number of messages allowed to send within the time window.
    limit: f64,
}

impl Ratelimit {
    /// Returns a new rate limiter with the given constraints.
    ///
    /// Rate limiter will allow to send no more than `limit` messages within duration `window`.
    pub fn new(window: Duration, limit: f64) -> Self {
        Self {
            next_time: UNIX_EPOCH,
            window: window.as_secs_f64(),
            limit,
        }
    }

    /// Returns true if it is allowed to send a message.
    fn can_send_at(&self, now: SystemTime) -> bool {
        now >= self.next_time
    }

    /// Returns true if can send another message now.
    ///
    /// This method takes mutable reference
    pub fn can_send(&self) -> bool {
        self.can_send_at(SystemTime::now())
    }

    fn send_at(&mut self, now: SystemTime) {
        let now = now
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs_f64();
        let next_time = self
            .next_time
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs_f64();
        self.next_time = UNIX_EPOCH
            + Duration::from_secs_f64(
                now + self.window
                    * (((next_time - now) / self.window).exp() + 1.0 / self.limit).ln(),
            );
    }

    /// Increases current usage value.
    ///
    /// It is possible to send message even if over quota, e.g. if the message sending is initiated
    /// by the user and should not be rate limited. However, sending messages when over quota
    /// further postpones the time when it will be allowed to send low priority messages.
    pub fn send(&mut self) {
        self.send_at(SystemTime::now())
    }

    fn until_can_send_at(&self, now: SystemTime) -> Duration {
        self.next_time.duration_since(now).unwrap_or(Duration::ZERO)
    }

    /// Calculates the time until `can_send` will return `true`.
    pub fn until_can_send(&self) -> Duration {
        self.until_can_send_at(SystemTime::now())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ratelimit() {
        let now = SystemTime::now();

        let mut ratelimit = Ratelimit::new(Duration::new(60, 0), 3.0);
        assert!(ratelimit.can_send_at(now));

        // Send burst of 3 messages.
        ratelimit.send_at(now);
        assert!(ratelimit.can_send_at(now));
        ratelimit.send_at(now);
        assert!(ratelimit.can_send_at(now));
        ratelimit.send_at(now);
        assert!(ratelimit.can_send_at(now));
        ratelimit.send_at(now);

        // Can't send more messages now.
        assert!(!ratelimit.can_send_at(now));

        // Can send one more message 20 seconds later.
        assert_eq!(ratelimit.until_can_send_at(now), Duration::from_secs(20));
        let now = now + Duration::from_secs(20);
        assert!(ratelimit.can_send_at(now));
        ratelimit.send_at(now);
        assert!(!ratelimit.can_send_at(now));

        // Send one more message anyway, over quota.
        ratelimit.send_at(now);

        // Always can send another message after 20 seconds,
        // leaky bucket never overflows.
        let now = now + Duration::from_secs(20);
        assert!(ratelimit.can_send_at(now));

        // Test that we don't panic if time appears to move backwards
        assert!(!ratelimit.can_send_at(now - Duration::from_secs(20)));
    }
}
