//! Pure retry timing logic (decoupled from App state / UI) for testability.
use std::time::{Duration, Instant};

/// Result of computing retry timing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetryTiming {
    pub should_retry_now: bool,
    /// Seconds until next retry (Some(0) means ready now); None means inactive/no countdown.
    pub seconds_until_retry: Option<u64>,
}

/// Compute retry timing given connection state inputs.
///
/// Inputs:
/// - `disconnected`: true when connection_state == Disconnected.
/// - `modal_active`: requires the connection error modal be visible to show countdown / trigger auto retry.
/// - `original_disconnect_time`: time we first noticed disconnect.
/// - `last_auto_retry`: time we last performed an automatic retry.
/// - `now`: current time (injected for determinism / tests).
/// - `interval`: retry interval duration.
pub(crate) fn compute_retry_timing(
    disconnected: bool,
    modal_active: bool,
    original_disconnect_time: Option<Instant>,
    last_auto_retry: Option<Instant>,
    now: Instant,
    interval: Duration,
) -> RetryTiming {
    if !disconnected || !modal_active {
        return RetryTiming {
            should_retry_now: false,
            seconds_until_retry: None,
        };
    }

    let baseline = match last_auto_retry.or(original_disconnect_time) {
        Some(b) => b,
        None => {
            return RetryTiming {
                should_retry_now: false,
                seconds_until_retry: None,
            };
        }
    };

    let elapsed = now.saturating_duration_since(baseline);
    if elapsed >= interval {
        RetryTiming {
            should_retry_now: true,
            seconds_until_retry: Some(0),
        }
    } else {
        let remaining = interval - elapsed;
        RetryTiming {
            should_retry_now: false,
            seconds_until_retry: Some(remaining.as_secs()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inactive_when_not_disconnected() {
        let now = Instant::now();
        let rt = compute_retry_timing(false, true, Some(now), None, now, Duration::from_secs(30));
        assert!(!rt.should_retry_now);
        assert_eq!(rt.seconds_until_retry, None);
    }

    #[test]
    fn countdown_progress_and_ready() {
        let base = Instant::now();
        let rt1 = compute_retry_timing(
            true,
            true,
            Some(base),
            None,
            base + Duration::from_secs(10),
            Duration::from_secs(30),
        );
        assert!(!rt1.should_retry_now);
        assert_eq!(rt1.seconds_until_retry, Some(20));
        let rt2 = compute_retry_timing(
            true,
            true,
            Some(base),
            None,
            base + Duration::from_secs(30),
            Duration::from_secs(30),
        );
        assert!(rt2.should_retry_now);
        assert_eq!(rt2.seconds_until_retry, Some(0));
    }

    #[test]
    fn uses_last_auto_retry_as_baseline() {
        let base: Instant = Instant::now();
        let last = base + Duration::from_secs(30); // one prior retry
        // 10s after last retry => 20s remaining
        let rt = compute_retry_timing(
            true,
            true,
            Some(base),
            Some(last),
            last + Duration::from_secs(10),
            Duration::from_secs(30),
        );
        assert!(!rt.should_retry_now);
        assert_eq!(rt.seconds_until_retry, Some(20));
    }
}
