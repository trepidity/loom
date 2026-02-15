use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::action::Action;
use crate::components::popup::Popup;
use crate::theme::Theme;

/// A toggleable in-TUI log viewer.
/// Collects log messages and displays them in a scrollable popup.
pub struct LogPanel {
    pub visible: bool,
    popup: Popup,
    theme: Theme,
    messages: Vec<LogEntry>,
    scroll_offset: usize,
}

struct LogEntry {
    level: LogLevel,
    message: String,
}

#[derive(Clone, Copy)]
enum LogLevel {
    Info,
    Error,
    Debug,
}

impl LogPanel {
    pub fn new(theme: Theme) -> Self {
        Self {
            visible: false,
            popup: Popup::new("Logs", theme.clone()).with_size(80, 60),
            theme,
            messages: Vec::new(),
            scroll_offset: 0,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if self.visible {
            self.popup.show();
            // Scroll to bottom
            self.scroll_offset = self.messages.len().saturating_sub(1);
        } else {
            self.popup.hide();
        }
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.popup.hide();
    }

    pub fn push_info(&mut self, msg: String) {
        self.messages.push(LogEntry {
            level: LogLevel::Info,
            message: msg,
        });
        self.trim();
    }

    pub fn push_error(&mut self, msg: String) {
        self.messages.push(LogEntry {
            level: LogLevel::Error,
            message: msg,
        });
        self.trim();
    }

    pub fn push_debug(&mut self, msg: String) {
        self.messages.push(LogEntry {
            level: LogLevel::Debug,
            message: msg,
        });
        self.trim();
    }

    fn trim(&mut self) {
        if self.messages.len() > 500 {
            self.messages.drain(..self.messages.len() - 500);
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.hide();
                Action::ClosePopup
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                Action::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.scroll_offset + 1 < self.messages.len() {
                    self.scroll_offset += 1;
                }
                Action::None
            }
            KeyCode::Home | KeyCode::Char('g') => {
                self.scroll_offset = 0;
                Action::None
            }
            KeyCode::End | KeyCode::Char('G') => {
                self.scroll_offset = self.messages.len().saturating_sub(1);
                Action::None
            }
            _ => Action::None,
        }
    }

    pub fn render(&self, frame: &mut Frame, full: Rect) {
        if !self.visible {
            return;
        }

        let area = self.popup.centered_area(full);
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(format!(" Logs ({}) ", self.messages.len()))
            .borders(Borders::ALL)
            .border_style(self.theme.popup_border)
            .title_style(self.theme.popup_title);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Layout: messages | hints (1)
        let layout = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);

        // Messages
        let visible_height = layout[0].height as usize;
        let start = self
            .scroll_offset
            .saturating_sub(visible_height.saturating_sub(1));
        let end = (start + visible_height).min(self.messages.len());

        let lines: Vec<Line> = self.messages[start..end]
            .iter()
            .map(|entry| {
                let (prefix, style) = match entry.level {
                    LogLevel::Info => ("[INFO] ", self.theme.normal),
                    LogLevel::Error => ("[ERR]  ", self.theme.error),
                    LogLevel::Debug => ("[DBG]  ", self.theme.dimmed),
                };
                Line::from(vec![
                    Span::styled(prefix, style),
                    Span::styled(&entry.message, style),
                ])
            })
            .collect();

        frame.render_widget(Paragraph::new(lines), layout[0]);

        // Hints
        let hints = Line::from(Span::styled(
            "\u{2191}/\u{2193}:scroll  g/G:top/bottom  q:close",
            self.theme.dimmed,
        ));
        frame.render_widget(Paragraph::new(hints), layout[1]);
    }
}
