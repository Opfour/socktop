//! Top header with hostname and CPU temperature indicator.

use crate::types::Metrics;
use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
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
        format!("socktop — host: {}", mm.hostname)
    } else {
        "socktop — connecting...".into()
    };
    // TLS indicator: lock vs lock with cross (using ✗). Keep explicit label for clarity.
    let tls_txt = if is_tls { "🔒 TLS" } else { "🔒✗ TLS" };
    // Token indicator
    let tok_txt = if has_token { "🔑 token" } else { "" };
    let mut parts = vec![base, tls_txt.into()];
    if !tok_txt.is_empty() {
        parts.push(tok_txt.into());
    }
    parts.push("(a: about, h: help, q: quit)".into());
    let title = parts.join(" | ");

    // Render the block with left-aligned title
    f.render_widget(Block::default().title(title).borders(Borders::BOTTOM), area);

    // Render polling intervals on the right side
    let mi = metrics_interval.as_millis();
    let pi = procs_interval.as_millis();
    let intervals = format!("⏱ {mi}ms metrics | {pi}ms procs");
    let intervals_width = intervals.len() as u16;

    if area.width > intervals_width + 2 {
        let right_area = Rect {
            x: area.x + area.width.saturating_sub(intervals_width + 1),
            y: area.y,
            width: intervals_width,
            height: 1,
        };
        let intervals_line = Line::from(Span::raw(intervals));
        f.render_widget(Paragraph::new(intervals_line), right_area);
    }
}
