//! Modal window system for socktop TUI application

use std::time::{Duration, Instant};

use super::theme::{
    BTN_EXIT_BG_ACTIVE, BTN_EXIT_FG_ACTIVE, BTN_EXIT_FG_INACTIVE, BTN_EXIT_TEXT,
    BTN_RETRY_BG_ACTIVE, BTN_RETRY_FG_ACTIVE, BTN_RETRY_FG_INACTIVE, BTN_RETRY_TEXT, ICON_CLUSTER,
    ICON_COUNTDOWN_LABEL, ICON_MESSAGE, ICON_OFFLINE_LABEL, ICON_RETRY_LABEL, ICON_WARNING_TITLE,
    LARGE_ERROR_ICON, MODAL_AGENT_FG, MODAL_BG, MODAL_BORDER_FG, MODAL_COUNTDOWN_LABEL_FG,
    MODAL_DIM_BG, MODAL_FG, MODAL_HINT_FG, MODAL_ICON_PINK, MODAL_OFFLINE_LABEL_FG,
    MODAL_RETRY_LABEL_FG, MODAL_TITLE_FG,
};
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

#[derive(Debug, Clone)]
pub enum ModalType {
    ConnectionError {
        message: String,
        disconnected_at: Instant,
        retry_count: u32,
        auto_retry_countdown: Option<u64>,
    },
    #[allow(dead_code)]
    Confirmation {
        title: String,
        message: String,
        confirm_text: String,
        cancel_text: String,
    },
    #[allow(dead_code)]
    Info { title: String, message: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModalAction {
    None,
    RetryConnection,
    ExitApp,
    Confirm,
    Cancel,
    Dismiss,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModalButton {
    Retry,
    Exit,
    Confirm,
    Cancel,
    Ok,
}

#[derive(Debug)]
pub struct ModalManager {
    stack: Vec<ModalType>,
    active_button: ModalButton,
}

impl ModalManager {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            active_button: ModalButton::Retry,
        }
    }
    pub fn is_active(&self) -> bool {
        !self.stack.is_empty()
    }

    pub fn push_modal(&mut self, modal: ModalType) {
        self.stack.push(modal);
        self.active_button = match self.stack.last() {
            Some(ModalType::ConnectionError { .. }) => ModalButton::Retry,
            Some(ModalType::Confirmation { .. }) => ModalButton::Confirm,
            Some(ModalType::Info { .. }) => ModalButton::Ok,
            None => ModalButton::Ok,
        };
    }
    pub fn pop_modal(&mut self) -> Option<ModalType> {
        let m = self.stack.pop();
        if let Some(next) = self.stack.last() {
            self.active_button = match next {
                ModalType::ConnectionError { .. } => ModalButton::Retry,
                ModalType::Confirmation { .. } => ModalButton::Confirm,
                ModalType::Info { .. } => ModalButton::Ok,
            };
        }
        m
    }
    pub fn update_connection_error_countdown(&mut self, new_countdown: Option<u64>) {
        if let Some(ModalType::ConnectionError {
            auto_retry_countdown,
            ..
        }) = self.stack.last_mut()
        {
            *auto_retry_countdown = new_countdown;
        }
    }
    pub fn handle_key(&mut self, key: KeyCode) -> ModalAction {
        if !self.is_active() {
            return ModalAction::None;
        }
        match key {
            KeyCode::Esc => {
                self.pop_modal();
                ModalAction::Cancel
            }
            KeyCode::Enter => self.handle_enter(),
            KeyCode::Tab | KeyCode::Right => {
                self.next_button();
                ModalAction::None
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.prev_button();
                ModalAction::None
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                if matches!(self.stack.last(), Some(ModalType::ConnectionError { .. })) {
                    ModalAction::RetryConnection
                } else {
                    ModalAction::None
                }
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                if matches!(self.stack.last(), Some(ModalType::ConnectionError { .. })) {
                    ModalAction::ExitApp
                } else {
                    ModalAction::None
                }
            }
            _ => ModalAction::None,
        }
    }
    fn handle_enter(&mut self) -> ModalAction {
        match (&self.stack.last(), &self.active_button) {
            (Some(ModalType::ConnectionError { .. }), ModalButton::Retry) => {
                ModalAction::RetryConnection
            }
            (Some(ModalType::ConnectionError { .. }), ModalButton::Exit) => ModalAction::ExitApp,
            (Some(ModalType::Confirmation { .. }), ModalButton::Confirm) => ModalAction::Confirm,
            (Some(ModalType::Confirmation { .. }), ModalButton::Cancel) => ModalAction::Cancel,
            (Some(ModalType::Info { .. }), ModalButton::Ok) => {
                self.pop_modal();
                ModalAction::Dismiss
            }
            _ => ModalAction::None,
        }
    }
    fn next_button(&mut self) {
        self.active_button = match (&self.stack.last(), &self.active_button) {
            (Some(ModalType::ConnectionError { .. }), ModalButton::Retry) => ModalButton::Exit,
            (Some(ModalType::ConnectionError { .. }), ModalButton::Exit) => ModalButton::Retry,
            (Some(ModalType::Confirmation { .. }), ModalButton::Confirm) => ModalButton::Cancel,
            (Some(ModalType::Confirmation { .. }), ModalButton::Cancel) => ModalButton::Confirm,
            _ => self.active_button.clone(),
        };
    }
    fn prev_button(&mut self) {
        self.next_button();
    }

    pub fn render(&self, f: &mut Frame) {
        if let Some(m) = self.stack.last() {
            self.render_background_dim(f);
            self.render_modal_content(f, m);
        }
    }

    fn render_background_dim(&self, f: &mut Frame) {
        let area = f.area();
        f.render_widget(Clear, area);
        f.render_widget(
            Block::default()
                .style(Style::default().bg(MODAL_DIM_BG).fg(MODAL_DIM_BG))
                .borders(Borders::NONE),
            area,
        );
    }

    fn render_modal_content(&self, f: &mut Frame, modal: &ModalType) {
        let area = f.area();
        let modal_area = self.centered_rect(70, 50, area);
        f.render_widget(Clear, modal_area);
        match modal {
            ModalType::ConnectionError {
                message,
                disconnected_at,
                retry_count,
                auto_retry_countdown,
            } => self.render_connection_error(
                f,
                modal_area,
                message,
                *disconnected_at,
                *retry_count,
                *auto_retry_countdown,
            ),
            ModalType::Confirmation {
                title,
                message,
                confirm_text,
                cancel_text,
            } => self.render_confirmation(f, modal_area, title, message, confirm_text, cancel_text),
            ModalType::Info { title, message } => self.render_info(f, modal_area, title, message),
        }
    }

    fn render_connection_error(
        &self,
        f: &mut Frame,
        area: Rect,
        message: &str,
        disconnected_at: Instant,
        retry_count: u32,
        auto_retry_countdown: Option<u64>,
    ) {
        let duration_text = format_duration(disconnected_at.elapsed());
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(4),
                Constraint::Length(4),
            ])
            .split(area);
        let block = Block::default()
            .title(ICON_WARNING_TITLE)
            .title_style(
                Style::default()
                    .fg(MODAL_TITLE_FG)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(MODAL_BORDER_FG))
            .style(Style::default().bg(MODAL_BG).fg(MODAL_FG));
        f.render_widget(block, area);

        let content_area = chunks[1];
        let max_w = content_area.width.saturating_sub(15) as usize;
        let clean_message = if message.to_lowercase().contains("hostname verification")
            || message.contains("socktop_connector")
        {
            "Connection failed - hostname verification disabled".to_string()
        } else if message.contains("Failed to fetch metrics:") {
            if let Some(p) = message.find(':') {
                let ess = message[p + 1..].trim();
                if ess.len() > max_w {
                    format!("{}...", &ess[..max_w.saturating_sub(3)])
                } else {
                    ess.to_string()
                }
            } else {
                "Connection error".to_string()
            }
        } else if message.starts_with("Retry failed:") {
            if let Some(p) = message.find(':') {
                let ess = message[p + 1..].trim();
                if ess.len() > max_w {
                    format!("{}...", &ess[..max_w.saturating_sub(3)])
                } else {
                    ess.to_string()
                }
            } else {
                "Retry failed".to_string()
            }
        } else if message.len() > max_w {
            format!("{}...", &message[..max_w.saturating_sub(3)])
        } else {
            message.to_string()
        };
        let truncate = |s: &str| {
            if s.len() > max_w {
                format!("{}...", &s[..max_w.saturating_sub(3)])
            } else {
                s.to_string()
            }
        };
        let agent_text = truncate("📡 Cannot connect to socktop agent");
        let message_text = truncate(&clean_message);
        let duration_display = truncate(&duration_text);
        let retry_display = truncate(&retry_count.to_string());
        let countdown_text = auto_retry_countdown.map(|c| {
            if c == 0 {
                "Auto retry now...".to_string()
            } else {
                format!("{c}s")
            }
        });

        // Determine if we have enough space (height + width) to show large centered icon
        let icon_max_width = LARGE_ERROR_ICON
            .iter()
            .map(|l| l.trim().chars().count())
            .max()
            .unwrap_or(0) as u16;
        let large_allowed = content_area.height >= (LARGE_ERROR_ICON.len() as u16 + 8)
            && content_area.width >= icon_max_width + 6; // small margin for borders/padding
        let mut icon_lines: Vec<Line> = Vec::new();
        if large_allowed {
            for &raw in LARGE_ERROR_ICON.iter() {
                let trimmed = raw.trim();
                icon_lines.push(Line::from(
                    trimmed
                        .chars()
                        .map(|ch| {
                            if ch == '!' {
                                Span::styled(
                                    ch.to_string(),
                                    Style::default()
                                        .fg(Color::White)
                                        .add_modifier(Modifier::BOLD),
                                )
                            } else if ch == '/' || ch == '\\' || ch == '_' {
                                // keep outline in pink
                                Span::styled(
                                    ch.to_string(),
                                    Style::default()
                                        .fg(MODAL_ICON_PINK)
                                        .add_modifier(Modifier::BOLD),
                                )
                            } else if ch == ' ' {
                                Span::raw(" ")
                            } else {
                                Span::styled(ch.to_string(), Style::default().fg(MODAL_ICON_PINK))
                            }
                        })
                        .collect::<Vec<_>>(),
                ));
            }
            icon_lines.push(Line::from("")); // blank spacer line below icon
        }

        let mut info_lines: Vec<Line> = Vec::new();
        if !large_allowed {
            info_lines.push(Line::from(vec![Span::styled(
                ICON_CLUSTER,
                Style::default().fg(MODAL_ICON_PINK),
            )]));
            info_lines.push(Line::from(""));
        }
        info_lines.push(Line::from(vec![Span::styled(
            &agent_text,
            Style::default().fg(MODAL_AGENT_FG),
        )]));
        info_lines.push(Line::from(""));
        info_lines.push(Line::from(vec![
            Span::styled(ICON_MESSAGE, Style::default().fg(MODAL_HINT_FG)),
            Span::styled(&message_text, Style::default().fg(MODAL_AGENT_FG)),
        ]));
        info_lines.push(Line::from(""));
        info_lines.push(Line::from(vec![
            Span::styled(
                ICON_OFFLINE_LABEL,
                Style::default().fg(MODAL_OFFLINE_LABEL_FG),
            ),
            Span::styled(
                &duration_display,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        info_lines.push(Line::from(vec![
            Span::styled(ICON_RETRY_LABEL, Style::default().fg(MODAL_RETRY_LABEL_FG)),
            Span::styled(
                &retry_display,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));
        if let Some(cd) = &countdown_text {
            info_lines.push(Line::from(vec![
                Span::styled(
                    ICON_COUNTDOWN_LABEL,
                    Style::default().fg(MODAL_COUNTDOWN_LABEL_FG),
                ),
                Span::styled(
                    cd,
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
        }

        let constrained = Rect {
            x: content_area.x + 2,
            y: content_area.y,
            width: content_area.width.saturating_sub(4),
            height: content_area.height,
        };
        if large_allowed {
            let split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(icon_lines.len() as u16),
                    Constraint::Min(0),
                ])
                .split(constrained);
            // Center the icon block; each line already trimmed so per-line centering keeps shape
            f.render_widget(
                Paragraph::new(Text::from(icon_lines))
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: false }),
                split[0],
            );
            f.render_widget(
                Paragraph::new(Text::from(info_lines))
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true }),
                split[1],
            );
        } else {
            f.render_widget(
                Paragraph::new(Text::from(info_lines))
                    .alignment(Alignment::Center)
                    .wrap(Wrap { trim: true }),
                constrained,
            );
        }

        let button_area = Rect {
            x: chunks[2].x,
            y: chunks[2].y,
            width: chunks[2].width,
            height: chunks[2].height.saturating_sub(1),
        };
        self.render_connection_error_buttons(f, button_area);
    }

    fn render_connection_error_buttons(&self, f: &mut Frame, area: Rect) {
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(15),
                Constraint::Percentage(10),
                Constraint::Percentage(15),
                Constraint::Percentage(30),
            ])
            .split(area);
        let retry_style = if self.active_button == ModalButton::Retry {
            Style::default()
                .bg(BTN_RETRY_BG_ACTIVE)
                .fg(BTN_RETRY_FG_ACTIVE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(BTN_RETRY_FG_INACTIVE)
                .add_modifier(Modifier::DIM)
        };
        let exit_style = if self.active_button == ModalButton::Exit {
            Style::default()
                .bg(BTN_EXIT_BG_ACTIVE)
                .fg(BTN_EXIT_FG_ACTIVE)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(BTN_EXIT_FG_INACTIVE)
                .add_modifier(Modifier::DIM)
        };
        f.render_widget(
            Paragraph::new(Text::from(Line::from(vec![Span::styled(
                BTN_RETRY_TEXT,
                retry_style,
            )])))
            .alignment(Alignment::Center),
            button_chunks[1],
        );
        f.render_widget(
            Paragraph::new(Text::from(Line::from(vec![Span::styled(
                BTN_EXIT_TEXT,
                exit_style,
            )])))
            .alignment(Alignment::Center),
            button_chunks[3],
        );
    }

    fn render_confirmation(
        &self,
        f: &mut Frame,
        area: Rect,
        title: &str,
        message: &str,
        confirm_text: &str,
        cancel_text: &str,
    ) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(area);
        let block = Block::default()
            .title(format!(" {title} "))
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black));
        f.render_widget(block, area);
        f.render_widget(
            Paragraph::new(message)
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true }),
            chunks[0],
        );
        let buttons = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(chunks[1]);
        let confirm_style = if self.active_button == ModalButton::Confirm {
            Style::default()
                .bg(Color::Green)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let cancel_style = if self.active_button == ModalButton::Cancel {
            Style::default()
                .bg(Color::Red)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Red)
        };
        f.render_widget(
            Paragraph::new(confirm_text)
                .style(confirm_style)
                .alignment(Alignment::Center),
            buttons[0],
        );
        f.render_widget(
            Paragraph::new(cancel_text)
                .style(cancel_style)
                .alignment(Alignment::Center),
            buttons[1],
        );
    }

    fn render_info(&self, f: &mut Frame, area: Rect, title: &str, message: &str) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(3)])
            .split(area);
        let block = Block::default()
            .title(format!(" {title} "))
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black));
        f.render_widget(block, area);
        f.render_widget(
            Paragraph::new(message)
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true }),
            chunks[0],
        );
        let ok_style = if self.active_button == ModalButton::Ok {
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Blue)
        };
        f.render_widget(
            Paragraph::new("[ Enter ] OK")
                .style(ok_style)
                .alignment(Alignment::Center),
            chunks[1],
        );
    }

    fn centered_rect(&self, percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let vert = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(vert[1])[1]
    }
}

fn format_duration(duration: Duration) -> String {
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
