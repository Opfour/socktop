//! Top header with hostname and CPU temperature indicator.

use crate::types::Metrics;
use ratatui::{
    layout::Rect,
    widgets::{Block, Borders},
};
use std::time::Duration;

pub fn draw_header(
    f: &mut ratatui::Frame<'_>,
    area: Rect,
    m: Option<&Metrics>,
    is_tls: bool,
    has_token: bool,
    metrics_interval: Duration,
    procs_interval: Duration,
) {
    let base = if let Some(mm) = m {
        let temp = mm
            .cpu_temp_c
            .map(|t| {
                let icon = if t < 50.0 {
                    "😎"
                } else if t < 85.0 {
                    "⚠️"
                } else {
                    "🔥"
                };
                format!("CPU Temp: {t:.1}°C {icon}")
            })
            .unwrap_or_else(|| "CPU Temp: N/A".into());
        format!("socktop — host: {} | {}", mm.hostname, temp)
    } else {
        "socktop — connecting...".into()
    };
    // TLS indicator: lock vs lock with cross (using ✗). Keep explicit label for clarity.
    let tls_txt = if is_tls { "🔒 TLS" } else { "🔒✗ TLS" };
    // Token indicator
    let tok_txt = if has_token { "🔑 token" } else { "" };
    let mi = metrics_interval.as_millis();
    let pi = procs_interval.as_millis();
    let intervals = format!("⏱  {mi}ms metrics | {pi}ms procs");
    let mut parts = vec![base, tls_txt.into()];
    if !tok_txt.is_empty() {
        parts.push(tok_txt.into());
    }
    parts.push(intervals);
    parts.push("(a: about, h: help, q: quit)".into());
    let title = parts.join(" | ");
    f.render_widget(Block::default().title(title).borders(Borders::BOTTOM), area);
}
