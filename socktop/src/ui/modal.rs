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
    widgets::{
        Axis, Block, Borders, Chart, Clear, Dataset, GraphType, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, Wrap,
    },
};

/// History data for process metrics rendering
pub struct ProcessHistoryData<'a> {
    pub cpu: &'a std::collections::VecDeque<f32>,
    pub mem: &'a std::collections::VecDeque<u64>,
    pub io_read: &'a std::collections::VecDeque<u64>,
    pub io_write: &'a std::collections::VecDeque<u64>,
}

/// Process data for modal rendering
pub struct ProcessModalData<'a> {
    pub details: Option<&'a socktop_connector::ProcessMetricsResponse>,
    pub journal: Option<&'a socktop_connector::JournalResponse>,
    pub history: ProcessHistoryData<'a>,
    pub unsupported: bool,
}

/// Parameters for rendering scatter plot
struct ScatterPlotParams<'a> {
    process: &'a socktop_connector::DetailedProcessInfo,
    main_user_ms: f64,
    main_system_ms: f64,
    max_user: f64,
    max_system: f64,
}

#[derive(Debug, Clone)]
pub enum ModalType {
    ConnectionError {
        message: String,
        disconnected_at: Instant,
        retry_count: u32,
        auto_retry_countdown: Option<u64>,
    },
    ProcessDetails {
        pid: u32,
    },
    #[allow(dead_code)]
    Confirmation {
        title: String,
        message: String,
        confirm_text: String,
        cancel_text: String,
    },
    #[allow(dead_code)]
    Info {
        title: String,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModalAction {
    None,    // Modal didn't handle the key, pass to main window
    Handled, // Modal handled the key, don't pass to main window
    RetryConnection,
    ExitApp,
    Confirm,
    Cancel,
    Dismiss,
    SwitchToParentProcess(u32), // Switch to viewing parent process details
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
    pub thread_scroll_offset: usize,
    pub journal_scroll_offset: usize,
    thread_scroll_max: usize,
    journal_scroll_max: usize,
}

impl ModalManager {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            active_button: ModalButton::Retry,
            thread_scroll_offset: 0,
            journal_scroll_offset: 0,
            thread_scroll_max: 0,
            journal_scroll_max: 0,
        }
    }
    pub fn is_active(&self) -> bool {
        !self.stack.is_empty()
    }

    pub fn current_modal(&self) -> Option<&ModalType> {
        self.stack.last()
    }

    pub fn push_modal(&mut self, modal: ModalType) {
        self.stack.push(modal);
        self.active_button = match self.stack.last() {
            Some(ModalType::ConnectionError { .. }) => ModalButton::Retry,
            Some(ModalType::ProcessDetails { .. }) => {
                // Reset scroll state for new process details
                self.thread_scroll_offset = 0;
                self.journal_scroll_offset = 0;
                self.thread_scroll_max = 0;
                self.journal_scroll_max = 0;
                ModalButton::Ok
            }
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
                ModalType::ProcessDetails { .. } => ModalButton::Ok,
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
            KeyCode::Char('x') | KeyCode::Char('X') => {
                if matches!(self.stack.last(), Some(ModalType::ProcessDetails { .. })) {
                    // Close all ProcessDetails modals at once (handles parent navigation chain)
                    while matches!(self.stack.last(), Some(ModalType::ProcessDetails { .. })) {
                        self.pop_modal();
                    }
                    ModalAction::Dismiss
                } else {
                    ModalAction::None
                }
            }
            KeyCode::Char('j') | KeyCode::Char('J') => {
                if matches!(self.stack.last(), Some(ModalType::ProcessDetails { .. })) {
                    self.thread_scroll_offset = self
                        .thread_scroll_offset
                        .saturating_add(1)
                        .min(self.thread_scroll_max);
                    ModalAction::Handled
                } else {
                    ModalAction::None
                }
            }
            KeyCode::Char('k') | KeyCode::Char('K') => {
                if matches!(self.stack.last(), Some(ModalType::ProcessDetails { .. })) {
                    self.thread_scroll_offset = self.thread_scroll_offset.saturating_sub(1);
                    ModalAction::Handled
                } else {
                    ModalAction::None
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if matches!(self.stack.last(), Some(ModalType::ProcessDetails { .. })) {
                    self.thread_scroll_offset = self
                        .thread_scroll_offset
                        .saturating_add(10)
                        .min(self.thread_scroll_max);
                    ModalAction::Handled
                } else {
                    ModalAction::None
                }
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                if matches!(self.stack.last(), Some(ModalType::ProcessDetails { .. })) {
                    self.thread_scroll_offset = self.thread_scroll_offset.saturating_sub(10);
                    ModalAction::Handled
                } else {
                    ModalAction::None
                }
            }
            KeyCode::Char('[') => {
                if matches!(self.stack.last(), Some(ModalType::ProcessDetails { .. })) {
                    self.journal_scroll_offset = self.journal_scroll_offset.saturating_sub(1);
                    ModalAction::Handled
                } else {
                    ModalAction::None
                }
            }
            KeyCode::Char(']') => {
                if matches!(self.stack.last(), Some(ModalType::ProcessDetails { .. })) {
                    self.journal_scroll_offset = self
                        .journal_scroll_offset
                        .saturating_add(1)
                        .min(self.journal_scroll_max);
                    ModalAction::Handled
                } else {
                    ModalAction::None
                }
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                // Switch to parent process if it exists
                if let Some(ModalType::ProcessDetails { pid }) = self.stack.last() {
                    // We need to get the parent PID from the process details
                    // For now, return a special action that the app can handle
                    // The app has access to the process details and can extract parent_pid
                    ModalAction::SwitchToParentProcess(*pid)
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
            (Some(ModalType::ProcessDetails { .. }), ModalButton::Ok) => {
                self.pop_modal();
                ModalAction::Dismiss
            }
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

    pub fn render(&mut self, f: &mut Frame, data: ProcessModalData) {
        if let Some(m) = self.stack.last().cloned() {
            self.render_background_dim(f);
            self.render_modal_content(f, &m, data);
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

    fn render_modal_content(&mut self, f: &mut Frame, modal: &ModalType, data: ProcessModalData) {
        let area = f.area();
        // Different sizes for different modal types
        let modal_area = match modal {
            ModalType::ProcessDetails { .. } => {
                // Process details modal uses almost full screen (95% width, 90% height)
                self.centered_rect(95, 90, area)
            }
            _ => {
                // Other modals use smaller size
                self.centered_rect(70, 50, area)
            }
        };
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
            ModalType::ProcessDetails { pid } => {
                self.render_process_details(f, modal_area, *pid, data)
            }
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

    fn render_process_details(
        &mut self,
        f: &mut Frame,
        area: Rect,
        pid: u32,
        data: ProcessModalData,
    ) {
        let title = format!("Process Details - PID {pid}");

        // Use neutral colors to match main UI aesthetic
        let block = Block::default().title(title).borders(Borders::ALL);

        // Split the modal into the 3-row layout as designed
        let inner = block.inner(area);
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(18), // Top row: CPU sparkline | Thread scatter plot
                Constraint::Length(25), // Middle row: Memory/IO graphs | Thread table | Command details (fixed height for consistent scrolling)
                Constraint::Min(6),     // Bottom row: Journal events (gets remaining space)
                Constraint::Length(1),  // Help line
            ])
            .split(inner);

        // Render the border
        f.render_widget(block, area);

        if let Some(details) = data.details {
            // Top Row: CPU sparkline (left) | Thread scatter plot (right)
            self.render_top_row_with_sparkline(
                f,
                main_chunks[0],
                &details.process,
                data.history.cpu,
            );

            // Middle Row: Memory/IO + Thread Table + Command Details (with process metadata)
            self.render_middle_row_with_metadata(
                f,
                main_chunks[1],
                &details.process,
                data.history.mem,
                data.history.io_read,
                data.history.io_write,
            );

            // Bottom Row: Journal Events
            if let Some(journal) = data.journal {
                self.render_journal_events(f, main_chunks[2], journal);
            } else {
                self.render_loading_journal_events(f, main_chunks[2]);
            }
        } else if data.unsupported {
            // Agent doesn't support this feature
            self.render_unsupported_message(f, main_chunks[0]);
            self.render_loading_middle_row(f, main_chunks[1]);
            self.render_loading_journal_events(f, main_chunks[2]);
        } else {
            // Loading states for all sections
            self.render_loading_top_row(f, main_chunks[0]);
            self.render_loading_middle_row(f, main_chunks[1]);
            self.render_loading_journal_events(f, main_chunks[2]);
        }

        // Help line
        let help_text = vec![Line::from(vec![
            Span::styled(
                "X ",
                Style::default()
                    .fg(super::theme::PROCESS_DETAILS_ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("close  ", Style::default().add_modifier(Modifier::DIM)),
            Span::styled(
                "P ",
                Style::default()
                    .fg(super::theme::PROCESS_DETAILS_ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "goto parent  ",
                Style::default().add_modifier(Modifier::DIM),
            ),
            Span::styled(
                "j/k ",
                Style::default()
                    .fg(super::theme::PROCESS_DETAILS_ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("threads  ", Style::default().add_modifier(Modifier::DIM)),
            Span::styled(
                "[ ] ",
                Style::default()
                    .fg(super::theme::PROCESS_DETAILS_ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("journal", Style::default().add_modifier(Modifier::DIM)),
        ])];
        let help = Paragraph::new(help_text).alignment(Alignment::Center);
        f.render_widget(help, main_chunks[3]);
    }

    fn render_thread_scatter_plot(
        &self,
        f: &mut Frame,
        area: Rect,
        process: &socktop_connector::DetailedProcessInfo,
    ) {
        let plot_block = Block::default()
            .title("Thread & Process CPU Time")
            .borders(Borders::ALL);

        let inner = plot_block.inner(area);

        // Convert CPU times from microseconds to milliseconds for better readability
        let main_user_ms = process.cpu_time_user as f64 / 1000.0;
        let main_system_ms = process.cpu_time_system as f64 / 1000.0;

        // Calculate max values for scaling
        let mut max_user = main_user_ms;
        let mut max_system = main_system_ms;

        for child in &process.child_processes {
            let child_user_ms = child.cpu_time_user as f64 / 1000.0;
            let child_system_ms = child.cpu_time_system as f64 / 1000.0;
            max_user = max_user.max(child_user_ms);
            max_system = max_system.max(child_system_ms);
        }

        // Add some padding to the scale
        max_user = (max_user * 1.1).max(1.0);
        max_system = (max_system * 1.1).max(1.0);

        // Render the existing scatter plot but in the smaller space
        self.render_scatter_plot_content(
            f,
            inner,
            ScatterPlotParams {
                process,
                main_user_ms,
                main_system_ms,
                max_user,
                max_system,
            },
        );

        // Render the border
        f.render_widget(plot_block, area);
    }

    fn render_memory_io_graphs(
        &self,
        f: &mut Frame,
        area: Rect,
        process: &socktop_connector::DetailedProcessInfo,
        mem_history: &std::collections::VecDeque<u64>,
        io_read_history: &std::collections::VecDeque<u64>,
        io_write_history: &std::collections::VecDeque<u64>,
    ) {
        let graphs_block = Block::default().title("Memory & I/O").borders(Borders::ALL);

        let mem_mb = process.mem_bytes as f64 / 1_048_576.0;
        let virtual_mb = process.virtual_mem_bytes as f64 / 1_048_576.0;

        let mut content_lines = vec![
            Line::from(vec![Span::styled(
                "🧠 Memory",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("  📊 RSS: ", Style::default().add_modifier(Modifier::DIM)),
                Span::raw(format!("{mem_mb:.1} MB")),
            ]),
        ];

        // Add memory sparkline if we have history
        if mem_history.len() >= 2 {
            let mem_data: Vec<u64> = mem_history.iter().map(|&bytes| bytes / 1_048_576).collect(); // Convert to MB
            let max_mem = mem_data.iter().copied().max().unwrap_or(1).max(1);

            // Create mini sparkline using Unicode blocks
            let blocks = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
            let sparkline_str: String = mem_data
                .iter()
                .map(|&val| {
                    let level = ((val as f64 / max_mem as f64) * 7.0).round() as usize;
                    blocks[level.min(7)]
                })
                .collect();

            content_lines.push(Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(sparkline_str, Style::default().fg(Color::Blue)),
            ]));
        } else {
            content_lines.push(Line::from(vec![Span::styled(
                "  Collecting...",
                Style::default().add_modifier(Modifier::DIM),
            )]));
        }

        content_lines.push(Line::from(vec![
            Span::styled("  Virtual: ", Style::default().add_modifier(Modifier::DIM)),
            Span::raw(format!("{virtual_mb:.1} MB")),
        ]));

        // Add shared memory if available
        if let Some(shared_bytes) = process.shared_mem_bytes {
            let shared_mb = shared_bytes as f64 / 1_048_576.0;
            content_lines.push(Line::from(vec![
                Span::styled("  Shared: ", Style::default().add_modifier(Modifier::DIM)),
                Span::raw(format!("{shared_mb:.1} MB")),
            ]));
        }

        content_lines.push(Line::from(""));
        content_lines.push(Line::from(vec![Span::styled(
            "💾 Disk I/O",
            Style::default().add_modifier(Modifier::BOLD),
        )]));

        // Add I/O stats if available
        match (process.read_bytes, process.write_bytes) {
            (Some(read), Some(write)) => {
                let read_mb = read as f64 / 1_048_576.0;
                let write_mb = write as f64 / 1_048_576.0;
                content_lines.push(Line::from(vec![
                    Span::styled("  📖 Read: ", Style::default().add_modifier(Modifier::DIM)),
                    Span::raw(format!("{read_mb:.1} MB")),
                ]));

                // Add read I/O sparkline if we have history
                if io_read_history.len() >= 2 {
                    let read_data: Vec<u64> = io_read_history
                        .iter()
                        .map(|&bytes| bytes / 1_048_576)
                        .collect(); // Convert to MB
                    let max_read = read_data.iter().copied().max().unwrap_or(1).max(1);

                    let blocks = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
                    let sparkline_str: String = read_data
                        .iter()
                        .map(|&val| {
                            let level = ((val as f64 / max_read as f64) * 7.0).round() as usize;
                            blocks[level.min(7)]
                        })
                        .collect();

                    content_lines.push(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(sparkline_str, Style::default().fg(Color::Green)),
                    ]));
                }

                content_lines.push(Line::from(vec![
                    Span::styled(
                        "  ✍️  Write: ",
                        Style::default().add_modifier(Modifier::DIM),
                    ),
                    Span::raw(format!("{write_mb:.1} MB")),
                ]));

                // Add write I/O sparkline if we have history
                if io_write_history.len() >= 2 {
                    let write_data: Vec<u64> = io_write_history
                        .iter()
                        .map(|&bytes| bytes / 1_048_576)
                        .collect(); // Convert to MB
                    let max_write = write_data.iter().copied().max().unwrap_or(1).max(1);

                    let blocks = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
                    let sparkline_str: String = write_data
                        .iter()
                        .map(|&val| {
                            let level = ((val as f64 / max_write as f64) * 7.0).round() as usize;
                            blocks[level.min(7)]
                        })
                        .collect();

                    content_lines.push(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(sparkline_str, Style::default().fg(Color::Yellow)),
                    ]));
                }
            }
            _ => {
                content_lines.push(Line::from(vec![Span::styled(
                    "  Not available",
                    Style::default().add_modifier(Modifier::DIM),
                )]));
            }
        }

        let content = Paragraph::new(content_lines).block(graphs_block);

        f.render_widget(content, area);
    }

    fn render_thread_table(
        &mut self,
        f: &mut Frame,
        area: Rect,
        process: &socktop_connector::DetailedProcessInfo,
    ) {
        let total_items = process.threads.len() + process.child_processes.len();

        // Manually calculate inner area (like processes.rs does)
        let inner_area = Rect {
            x: area.x + 1,
            y: area.y + 1,
            width: area.width.saturating_sub(2),
            height: area.height.saturating_sub(2),
        };

        // Calculate visible rows: inner height minus header (1 line) and header bottom margin (1 line)
        let visible_rows = inner_area.height.saturating_sub(2).max(1) as usize;

        // Calculate and store max scroll for key handler bounds checking
        self.thread_scroll_max = if total_items > visible_rows {
            total_items.saturating_sub(visible_rows)
        } else {
            0
        };

        // Clamp scroll offset to valid range
        let scroll_offset = self.thread_scroll_offset.min(self.thread_scroll_max);

        // Combine threads and processes into rows
        let mut rows = Vec::new();

        // Add threads first
        for thread in &process.threads {
            rows.push(Row::new(vec![
                Line::from(Span::styled("[T]", Style::default().fg(Color::Cyan))),
                Line::from(format!("{}", thread.tid)),
                Line::from(thread.name.clone()),
                Line::from(thread.status.clone()),
            ]));
        }

        // Add child processes
        for child in &process.child_processes {
            rows.push(Row::new(vec![
                Line::from(Span::styled("[P]", Style::default().fg(Color::Green))),
                Line::from(format!("{}", child.pid)),
                Line::from(child.name.clone()),
                Line::from(format!("{:.1}%", child.cpu_usage)),
            ]));
        }

        // Create table header
        let header = Row::new(vec!["Type", "TID/PID", "Name", "Status/CPU"])
            .style(Style::default().add_modifier(Modifier::BOLD))
            .bottom_margin(1);

        let block = Block::default()
            .title(format!(
                "Threads ({}) & Children ({}) - j/k to scroll, u/d for 10x",
                process.threads.len(),
                process.child_processes.len()
            ))
            .borders(Borders::ALL);

        let table = Table::new(
            rows.iter().skip(scroll_offset).take(visible_rows).cloned(),
            [
                Constraint::Length(6),
                Constraint::Length(10),
                Constraint::Min(15),
                Constraint::Length(12),
            ],
        )
        .header(header)
        .block(block)
        .highlight_style(Style::default());

        f.render_widget(table, area);

        // Render scrollbar if there are more items than visible
        if total_items > visible_rows {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            // Use the same max_scroll value we use for clamping
            // This ensures the scrollbar position matches our actual scroll range
            let mut scrollbar_state =
                ScrollbarState::new(self.thread_scroll_max).position(scroll_offset);

            let scrollbar_area = Rect {
                x: area.x + area.width.saturating_sub(1),
                y: area.y + 1,
                width: 1,
                height: area.height.saturating_sub(2),
            };

            f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
        }
    }

    fn render_journal_events(
        &mut self,
        f: &mut Frame,
        area: Rect,
        journal: &socktop_connector::JournalResponse,
    ) {
        let total_entries = journal.entries.len();
        let visible_lines = area.height.saturating_sub(2) as usize; // Account for borders

        // Calculate and store max scroll for key handler bounds checking
        self.journal_scroll_max = if total_entries > visible_lines {
            total_entries.saturating_sub(visible_lines)
        } else {
            0
        };

        // Clamp scroll offset to valid range
        let scroll_offset = self.journal_scroll_offset.min(self.journal_scroll_max);

        let journal_block = Block::default()
            .title(format!(
                "Journal Events ({total_entries} entries) - Use [ ] to scroll"
            ))
            .borders(Borders::ALL);

        let content_lines: Vec<Line> = if journal.entries.is_empty() {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "No journal entries found for this process",
                    Style::default().add_modifier(Modifier::DIM),
                )),
            ]
        } else {
            journal
                .entries
                .iter()
                .skip(scroll_offset)
                .take(visible_lines)
                .map(|entry| {
                    let priority_style = match entry.priority {
                        socktop_connector::LogLevel::Error
                        | socktop_connector::LogLevel::Critical => Style::default().fg(Color::Red),
                        socktop_connector::LogLevel::Warning => Style::default().fg(Color::Yellow),
                        socktop_connector::LogLevel::Info | socktop_connector::LogLevel::Notice => {
                            Style::default().fg(Color::Blue)
                        }
                        _ => Style::default(),
                    };

                    let timestamp = &entry.timestamp[..entry.timestamp.len().min(16)]; // Show just time
                    let message_max_len = area.width.saturating_sub(30) as usize; // Leave space for timestamp + priority
                    let message = &entry.message[..entry.message.len().min(message_max_len)];

                    Line::from(vec![
                        Span::styled(timestamp, Style::default().add_modifier(Modifier::DIM)),
                        Span::raw(" "),
                        Span::styled(
                            format!("{:>7}", format!("{:?}", entry.priority)),
                            priority_style,
                        ),
                        Span::raw(" "),
                        Span::raw(message),
                        if entry.message.len() > message_max_len {
                            Span::styled("...", Style::default().add_modifier(Modifier::DIM))
                        } else {
                            Span::raw("")
                        },
                    ])
                })
                .collect()
        };

        let content = Paragraph::new(content_lines).block(journal_block);

        f.render_widget(content, area);

        // Render scrollbar if there are more entries than visible
        if total_entries > visible_lines {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"));

            // Use the same max_scroll value we use for clamping
            let mut scrollbar_state =
                ScrollbarState::new(self.journal_scroll_max).position(scroll_offset);

            let scrollbar_area = Rect {
                x: area.x + area.width.saturating_sub(1),
                y: area.y + 1,
                width: 1,
                height: area.height.saturating_sub(2),
            };

            f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
        }
    }

    fn render_scatter_plot_content(&self, f: &mut Frame, area: Rect, params: ScatterPlotParams) {
        if area.width < 20 || area.height < 10 {
            // Area too small for meaningful plot
            let content = Paragraph::new(vec![Line::from(Span::styled(
                "Area too small for plot",
                Style::default().fg(MODAL_HINT_FG),
            ))])
            .alignment(Alignment::Center)
            .style(Style::default().bg(MODAL_BG));
            f.render_widget(content, area);
            return;
        }

        // Calculate plot dimensions (leave space for axes labels)
        let plot_width = area.width.saturating_sub(8) as usize; // Leave space for Y-axis labels
        let plot_height = area.height.saturating_sub(3) as usize; // Leave space for X-axis labels

        if plot_width == 0 || plot_height == 0 {
            return;
        }

        // Create a 2D grid to represent the plot
        let mut plot_grid = vec![vec![' '; plot_width]; plot_height];

        // Plot main process
        let main_x = ((params.main_user_ms / params.max_user) * (plot_width - 1) as f64) as usize;
        let main_y = plot_height.saturating_sub(1).saturating_sub(
            ((params.main_system_ms / params.max_system) * (plot_height - 1) as f64) as usize,
        );
        if main_x < plot_width && main_y < plot_height {
            plot_grid[main_y][main_x] = '●'; // Main process marker
        }

        // Plot threads (use different marker)
        for thread in &params.process.threads {
            let thread_user_ms = thread.cpu_time_user as f64 / 1000.0;
            let thread_system_ms = thread.cpu_time_system as f64 / 1000.0;

            let thread_x = ((thread_user_ms / params.max_user) * (plot_width - 1) as f64) as usize;
            let thread_y = plot_height.saturating_sub(1).saturating_sub(
                ((thread_system_ms / params.max_system) * (plot_height - 1) as f64) as usize,
            );

            if thread_x < plot_width && thread_y < plot_height {
                if plot_grid[thread_y][thread_x] == ' ' {
                    plot_grid[thread_y][thread_x] = '○'; // Thread marker (hollow circle)
                } else if plot_grid[thread_y][thread_x] == '○' {
                    plot_grid[thread_y][thread_x] = '◎'; // Multiple threads at same point
                } else {
                    plot_grid[thread_y][thread_x] = '◉'; // Mixed threads/processes at same point
                }
            }
        }

        // Plot child processes
        for child in &params.process.child_processes {
            let child_user_ms = child.cpu_time_user as f64 / 1000.0;
            let child_system_ms = child.cpu_time_system as f64 / 1000.0;

            let child_x = ((child_user_ms / params.max_user) * (plot_width - 1) as f64) as usize;
            let child_y = plot_height.saturating_sub(1).saturating_sub(
                ((child_system_ms / params.max_system) * (plot_height - 1) as f64) as usize,
            );

            if child_x < plot_width && child_y < plot_height {
                if plot_grid[child_y][child_x] == ' ' {
                    plot_grid[child_y][child_x] = '•'; // Child process marker
                } else {
                    plot_grid[child_y][child_x] = '◉'; // Multiple items at same point
                }
            }
        }

        // Render the plot
        let mut lines = Vec::new();

        // Add Y-axis labels and plot content
        for (i, row) in plot_grid.iter().enumerate() {
            let y_value = params.max_system * (1.0 - (i as f64 / (plot_height - 1) as f64));
            // Always format with 4 characters width, right-aligned, to prevent axis shifting
            let y_label = if y_value >= 100.0 {
                format!("{y_value:>4.0}")
            } else {
                format!("{y_value:>4.1}")
            };

            let plot_content: String = row.iter().collect();

            lines.push(Line::from(vec![
                Span::styled(y_label, Style::default()),
                Span::styled(" │", Style::default()),
                Span::styled(plot_content, Style::default()),
            ]));
        }

        // Add X-axis
        let x_axis_padding = "     ".to_string(); // Match Y-axis label width
        let x_axis_line = "─".repeat(plot_width + 1);
        lines.push(Line::from(vec![
            Span::styled(x_axis_padding, Style::default()),
            Span::styled(x_axis_line, Style::default()),
        ]));

        // Add X-axis labels
        let x_label_start = "0.0".to_string();
        let x_label_mid = format!("{:.1}", params.max_user / 2.0);
        let x_label_end = format!("{:.1}", params.max_user);

        let spacing = plot_width / 3;
        let x_labels = format!(
            "     {}{}{}{}{}",
            x_label_start,
            " ".repeat(spacing.saturating_sub(x_label_start.len())),
            x_label_mid,
            " ".repeat(spacing.saturating_sub(x_label_mid.len())),
            x_label_end
        );

        lines.push(Line::from(vec![Span::styled(x_labels, Style::default())]));

        // Add axis titles
        lines.push(Line::from(vec![Span::styled(
            "     User CPU Time (ms) →",
            Style::default().add_modifier(Modifier::BOLD),
        )]));

        // Add legend
        lines.insert(
            0,
            Line::from(vec![Span::styled(
                "System CPU Time (ms) ↑",
                Style::default().add_modifier(Modifier::BOLD),
            )]),
        );
        lines.insert(
            1,
            Line::from(vec![Span::styled(
                "● Main Process  ○ Thread  • Child Process  ◉ Multiple",
                Style::default().add_modifier(Modifier::DIM),
            )]),
        );

        let content = Paragraph::new(lines)
            .style(Style::default())
            .alignment(Alignment::Left);

        f.render_widget(content, area);
    }

    fn render_loading_top_row(&self, f: &mut Frame, area: Rect) {
        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        self.render_loading_metadata(f, top_chunks[0]);
        self.render_loading_scatter(f, top_chunks[1]);
    }

    fn render_loading_middle_row(&self, f: &mut Frame, area: Rect) {
        let middle_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ])
            .split(area);

        self.render_loading_graphs(f, middle_chunks[0]);
        self.render_loading_table(f, middle_chunks[1]);
        self.render_loading_command(f, middle_chunks[2]);
    }

    fn render_loading_metadata(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Process Info & CPU History")
            .borders(Borders::ALL);

        let content = Paragraph::new("Loading process metadata...")
            .block(block)
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::DIM));

        f.render_widget(content, area);
    }

    fn render_loading_scatter(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Thread CPU Time Distribution")
            .borders(Borders::ALL);

        let content = Paragraph::new("Loading CPU time data...")
            .block(block)
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::DIM));

        f.render_widget(content, area);
    }

    fn render_loading_graphs(&self, f: &mut Frame, area: Rect) {
        let block = Block::default().title("Memory & I/O").borders(Borders::ALL);

        let content = Paragraph::new("Loading memory & I/O data...")
            .block(block)
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::DIM));

        f.render_widget(content, area);
    }

    fn render_loading_table(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Child Processes")
            .borders(Borders::ALL);

        let content = Paragraph::new("Loading child process data...")
            .block(block)
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::DIM));

        f.render_widget(content, area);
    }

    fn render_loading_command(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Command & Details")
            .borders(Borders::ALL);

        let content = Paragraph::new("Loading command details...")
            .block(block)
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::DIM));

        f.render_widget(content, area);
    }

    fn render_loading_journal_events(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Journal Events")
            .borders(Borders::ALL);

        let content = Paragraph::new("Loading journal entries...")
            .block(block)
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::DIM));

        f.render_widget(content, area);
    }

    fn render_unsupported_message(&self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title("Process Details")
            .borders(Borders::ALL);

        let content = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "⚠  Agent Update Required",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "This agent version does not support per-process metrics.",
                Style::default().add_modifier(Modifier::DIM),
            )),
            Line::from(Span::styled(
                "Please update your socktop_agent to the latest version.",
                Style::default().add_modifier(Modifier::DIM),
            )),
        ])
        .block(block)
        .alignment(Alignment::Center);

        f.render_widget(content, area);
    }

    fn render_top_row_with_sparkline(
        &self,
        f: &mut Frame,
        area: Rect,
        process: &socktop_connector::DetailedProcessInfo,
        cpu_history: &std::collections::VecDeque<f32>,
    ) {
        // Split top row: CPU sparkline (left 60%) | Thread scatter plot (right 40%)
        let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(60), // CPU sparkline
                Constraint::Percentage(40), // Thread scatter plot
            ])
            .split(area);

        self.render_cpu_sparkline(f, top_chunks[0], process, cpu_history);
        self.render_thread_scatter_plot(f, top_chunks[1], process);
    }

    fn render_cpu_sparkline(
        &self,
        f: &mut Frame,
        area: Rect,
        _process: &socktop_connector::DetailedProcessInfo,
        cpu_history: &std::collections::VecDeque<f32>,
    ) {
        // Calculate actual average and current
        let current_cpu = cpu_history.back().copied().unwrap_or(0.0);
        let avg_cpu = if cpu_history.is_empty() {
            0.0
        } else {
            cpu_history.iter().sum::<f32>() / cpu_history.len() as f32
        };
        let title = format!("📊 CPU avg: {avg_cpu:.1}% (now: {current_cpu:.1}%)");

        // Similar to main CPU rendering but for process CPU
        if cpu_history.len() < 2 {
            let block = Block::default().title(title).borders(Borders::ALL);
            let inner = block.inner(area);
            f.render_widget(block, area);

            let content = Paragraph::new("Collecting CPU history data...")
                .alignment(Alignment::Center)
                .style(Style::default().add_modifier(Modifier::DIM));
            f.render_widget(content, inner);
            return;
        }

        let max_points = area.width.saturating_sub(10) as usize; // Leave room for Y-axis labels
        let start = cpu_history.len().saturating_sub(max_points);

        // Create data points for the chart
        let data: Vec<(f64, f64)> = cpu_history
            .iter()
            .skip(start)
            .enumerate()
            .map(|(i, &val)| (i as f64, val as f64))
            .collect();

        let datasets = vec![
            Dataset::default()
                .name("CPU %")
                .marker(ratatui::symbols::Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::default().fg(Color::Cyan))
                .data(&data),
        ];

        let x_max = data.len().max(1) as f64;
        let y_max = cpu_history.iter().copied().fold(0.0f32, f32::max).max(10.0) as f64; // At least 10% scale

        let y_labels = vec![
            Line::from("0%"),
            Line::from(format!("{:.0}%", y_max / 2.0)),
            Line::from(format!("{y_max:.0}%")),
        ];

        let chart = Chart::new(datasets)
            .block(Block::default().borders(Borders::ALL).title(title))
            .x_axis(
                Axis::default()
                    .style(Style::default().fg(Color::Gray))
                    .bounds([0.0, x_max]),
            )
            .y_axis(
                Axis::default()
                    .style(Style::default().fg(Color::Gray))
                    .labels(y_labels)
                    .bounds([0.0, y_max]),
            );

        f.render_widget(chart, area);
    }

    fn render_middle_row_with_metadata(
        &mut self,
        f: &mut Frame,
        area: Rect,
        process: &socktop_connector::DetailedProcessInfo,
        mem_history: &std::collections::VecDeque<u64>,
        io_read_history: &std::collections::VecDeque<u64>,
        io_write_history: &std::collections::VecDeque<u64>,
    ) {
        // Split middle row: Memory/IO (30%) | Thread table (40%) | Command + Metadata (30%)
        let middle_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ])
            .split(area);

        self.render_memory_io_graphs(
            f,
            middle_chunks[0],
            process,
            mem_history,
            io_read_history,
            io_write_history,
        );
        self.render_thread_table(f, middle_chunks[1], process);
        self.render_command_and_metadata(f, middle_chunks[2], process);
    }

    fn render_command_and_metadata(
        &self,
        f: &mut Frame,
        area: Rect,
        process: &socktop_connector::DetailedProcessInfo,
    ) {
        let details_block = Block::default()
            .title("Command & Details")
            .borders(Borders::ALL);

        // Calculate uptime
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let uptime_secs = now.saturating_sub(process.start_time);
        let uptime_str = format_uptime(uptime_secs);

        // Format CPU times
        let user_time_sec = process.cpu_time_user as f64 / 1_000_000.0;
        let system_time_sec = process.cpu_time_system as f64 / 1_000_000.0;

        let mut content_lines = vec![
            Line::from(vec![
                Span::styled("⚡ Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&process.status),
            ]),
            Line::from(vec![
                Span::styled(
                    "⏱️  Uptime: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(uptime_str),
            ]),
            Line::from(vec![
                Span::styled(
                    "🧵 Threads: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("{}", process.thread_count)),
            ]),
            Line::from(vec![
                Span::styled(
                    "👶 Children: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("{}", process.child_processes.len())),
            ]),
        ];

        // Add file descriptors if available
        if let Some(fd_count) = process.fd_count {
            content_lines.push(Line::from(vec![
                Span::styled("📁 FDs: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(format!("{fd_count}")),
            ]));
        }

        content_lines.push(Line::from(""));

        // Process hierarchy with clickable parent PID
        if let Some(ppid) = process.parent_pid {
            content_lines.push(Line::from(vec![
                Span::styled(
                    "👪 Parent PID: ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{ppid} "),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::UNDERLINED),
                ),
                Span::styled(
                    "[P to open]",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::DIM),
                ),
            ]));
        }

        content_lines.push(Line::from(vec![
            Span::styled("👤 UID: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{}", process.user_id)),
            Span::styled("  👥 GID: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{}", process.group_id)),
        ]));

        content_lines.push(Line::from(""));
        content_lines.push(Line::from(vec![Span::styled(
            "⏲️  CPU Time",
            Style::default().add_modifier(Modifier::BOLD),
        )]));
        content_lines.push(Line::from(vec![
            Span::styled("  User: ", Style::default().add_modifier(Modifier::DIM)),
            Span::raw(format!("{user_time_sec:.2}s")),
        ]));
        content_lines.push(Line::from(vec![
            Span::styled("  System: ", Style::default().add_modifier(Modifier::DIM)),
            Span::raw(format!("{system_time_sec:.2}s")),
        ]));

        content_lines.push(Line::from(""));

        // Executable path if available
        if let Some(exe) = &process.executable_path {
            content_lines.push(Line::from(vec![Span::styled(
                "📂 Executable:",
                Style::default().add_modifier(Modifier::BOLD),
            )]));
            // Truncate if too long
            let max_width = (area.width.saturating_sub(4)) as usize;
            if exe.len() > max_width {
                let truncated = format!("...{}", &exe[exe.len().saturating_sub(max_width - 3)..]);
                content_lines.push(Line::from(Span::styled(
                    truncated,
                    Style::default().add_modifier(Modifier::DIM),
                )));
            } else {
                content_lines.push(Line::from(Span::styled(
                    exe,
                    Style::default().add_modifier(Modifier::DIM),
                )));
            }
        }

        // Working directory if available
        if let Some(cwd) = &process.working_directory {
            content_lines.push(Line::from(vec![Span::styled(
                "📁 Working Dir:",
                Style::default().add_modifier(Modifier::BOLD),
            )]));
            // Truncate if too long
            let max_width = (area.width.saturating_sub(4)) as usize;
            if cwd.len() > max_width {
                let truncated = format!("...{}", &cwd[cwd.len().saturating_sub(max_width - 3)..]);
                content_lines.push(Line::from(Span::styled(
                    truncated,
                    Style::default().add_modifier(Modifier::DIM),
                )));
            } else {
                content_lines.push(Line::from(Span::styled(
                    cwd,
                    Style::default().add_modifier(Modifier::DIM),
                )));
            }
        }

        content_lines.push(Line::from(""));

        // Add command line (wrap if needed)
        content_lines.push(Line::from(vec![Span::styled(
            "⚙️  Command:",
            Style::default().add_modifier(Modifier::BOLD),
        )]));

        // Split command into multiple lines if too long
        let cmd_text = &process.command;
        let max_width = (area.width.saturating_sub(4)) as usize;
        if cmd_text.len() > max_width {
            for chunk in cmd_text.as_bytes().chunks(max_width) {
                if let Ok(s) = std::str::from_utf8(chunk) {
                    content_lines.push(Line::from(Span::styled(
                        s,
                        Style::default().add_modifier(Modifier::DIM),
                    )));
                }
            }
        } else {
            content_lines.push(Line::from(Span::styled(
                cmd_text,
                Style::default().add_modifier(Modifier::DIM),
            )));
        }

        let content = Paragraph::new(content_lines)
            .block(details_block)
            .wrap(Wrap { trim: false });

        f.render_widget(content, area);
    }
}

fn format_uptime(secs: u64) -> String {
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
