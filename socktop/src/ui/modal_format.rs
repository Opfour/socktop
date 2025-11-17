//! Formatting utilities for process details modal

use std::time::Duration;

/// Format uptime in human-readable form
pub fn format_uptime(secs: u64) -> String {
    let days = secs / 86400;
    let hours = (secs % 86400) / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    if days > 0 {
        format!("{days}d {hours}h {minutes}m")
    } else if hours > 0 {
        format!("{hours}h {minutes}m {seconds}s")
    } else if minutes > 0 {
        format!("{minutes}m {seconds}s")
    } else {
        format!("{seconds}s")
    }
}

/// Format duration in human-readable form
pub fn format_duration(duration: Duration) -> String {
    let total = duration.as_secs();
    let h = total / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if h > 0 {
        format!("{h}h {m}m {s}s")
    } else if m > 0 {
        format!("{m}m {s}s")
    } else {
        format!("{s}s")
    }
}

/// Normalize CPU usage to 0-100% by dividing by thread count
pub fn normalize_cpu_usage(cpu_usage: f32, thread_count: u32) -> f32 {
    let threads = thread_count.max(1) as f32;
    (cpu_usage / threads).min(100.0)
}

/// Calculate dynamic Y-axis maximum in 10% increments
pub fn calculate_dynamic_y_max(max_value: f64) -> f64 {
    ((max_value / 10.0).ceil() * 10.0).clamp(10.0, 100.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_uptime_seconds() {
        assert_eq!(format_uptime(45), "45s");
    }

    #[test]
    fn test_format_uptime_minutes() {
        assert_eq!(format_uptime(125), "2m 5s");
    }

    #[test]
    fn test_format_uptime_hours() {
        assert_eq!(format_uptime(3665), "1h 1m 5s");
    }

    #[test]
    fn test_format_uptime_days() {
        assert_eq!(format_uptime(90061), "1d 1h 1m");
    }

    #[test]
    fn test_normalize_cpu_single_thread() {
        assert_eq!(normalize_cpu_usage(50.0, 1), 50.0);
    }

    #[test]
    fn test_normalize_cpu_multi_thread() {
        assert_eq!(normalize_cpu_usage(400.0, 4), 100.0);
    }

    #[test]
    fn test_normalize_cpu_zero_threads() {
        // Should default to 1 thread to avoid division by zero
        assert_eq!(normalize_cpu_usage(100.0, 0), 100.0);
    }

    #[test]
    fn test_normalize_cpu_caps_at_100() {
        assert_eq!(normalize_cpu_usage(150.0, 1), 100.0);
    }

    #[test]
    fn test_dynamic_y_max_rounds_up() {
        assert_eq!(calculate_dynamic_y_max(15.0), 20.0);
        assert_eq!(calculate_dynamic_y_max(25.0), 30.0);
        assert_eq!(calculate_dynamic_y_max(5.0), 10.0);
    }

    #[test]
    fn test_dynamic_y_max_minimum() {
        assert_eq!(calculate_dynamic_y_max(0.0), 10.0);
        assert_eq!(calculate_dynamic_y_max(3.0), 10.0);
    }

    #[test]
    fn test_dynamic_y_max_caps_at_100() {
        assert_eq!(calculate_dynamic_y_max(95.0), 100.0);
        assert_eq!(calculate_dynamic_y_max(100.0), 100.0);
    }
}
