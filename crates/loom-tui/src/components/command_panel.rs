use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::action::Action;
use crate::component::Component;
use crate::theme::Theme;

/// The bottom-right panel: command input and status messages.
pub struct CommandPanel {
    pub messages: Vec<StatusMessage>,
    /// When true, the input line is active and capturing keystrokes.
    pub input_active: bool,
    pub input_buffer: String,
    theme: Theme,
    area: Option<Rect>,
}

pub struct StatusMessage {
    pub text: String,
    pub is_error: bool,
}

impl CommandPanel {
    pub fn new(theme: Theme) -> Self {
        Self {
            messages: Vec::new(),
            input_active: false,
            input_buffer: String::new(),
            theme,
            area: None,
        }
    }

    pub fn push_message(&mut self, text: String) {
        self.messages.push(StatusMessage {
            text,
            is_error: false,
        });
        // Keep last 100 messages
        if self.messages.len() > 100 {
            self.messages.remove(0);
        }
    }

    pub fn push_error(&mut self, text: String) {
        self.messages.push(StatusMessage {
            text,
            is_error: true,
        });
        if self.messages.len() > 100 {
            self.messages.remove(0);
        }
    }

    pub fn activate_input(&mut self) {
        self.input_active = true;
        self.input_buffer.clear();
    }

    pub fn deactivate_input(&mut self) {
        self.input_active = false;
        self.input_buffer.clear();
    }

    /// Handle key events when the command panel is focused.
    /// Returns an Action for the app to dispatch.
    pub fn handle_input_key(&mut self, key: KeyEvent) -> Action {
        if !self.input_active {
            // Activate on '/' or ':'
            match key.code {
                KeyCode::Char('/') | KeyCode::Char(':') => {
                    self.activate_input();
                    return Action::None;
                }
                _ => return Action::None,
            }
        }

        // Input mode: capture text
        match key.code {
            KeyCode::Enter => {
                let query = self.input_buffer.clone();
                self.deactivate_input();
                if query.is_empty() {
                    Action::None
                } else {
                    self.push_message(format!("Search: {}", query));
                    Action::SearchExecute(query)
                }
            }
            KeyCode::Esc => {
                self.deactivate_input();
                Action::None
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
                Action::None
            }
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
                Action::None
            }
            _ => Action::None,
        }
    }
}

impl CommandPanel {
    /// Render as a read-only status log with a custom title.
    pub fn render_status(&self, frame: &mut Frame, area: Rect, title: &str) {
        let block = Block::default()
            .title(format!(" {} ", title))
            .borders(Borders::ALL)
            .border_style(self.theme.border);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let msg_height = inner.height as usize;
        let start = self.messages.len().saturating_sub(msg_height);
        let visible_messages = &self.messages[start..];

        let lines: Vec<Line> = visible_messages
            .iter()
            .map(|msg| {
                let style = if msg.is_error {
                    self.theme.error
                } else {
                    self.theme.normal
                };
                Line::from(Span::styled(&msg.text, style))
            })
            .collect();

        frame.render_widget(Paragraph::new(lines), inner);
    }
}

impl Component for CommandPanel {
    fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused {
            self.theme.border_focused
        } else {
            self.theme.border
        };

        let mut block = Block::default()
            .title(" Command ")
            .borders(Borders::ALL)
            .border_style(border_style);
        if focused {
            block = block.border_type(BorderType::Double);
        }

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Layout: messages (flex) | input line (1)
        let layout = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);

        // Messages
        let msg_height = layout[0].height as usize;
        let start = self.messages.len().saturating_sub(msg_height);
        let visible_messages = &self.messages[start..];

        let lines: Vec<Line> = visible_messages
            .iter()
            .map(|msg| {
                let style = if msg.is_error {
                    self.theme.error
                } else {
                    self.theme.normal
                };
                Line::from(Span::styled(&msg.text, style))
            })
            .collect();

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, layout[0]);

        // Input line
        let input_line = if self.input_active {
            Line::from(vec![
                Span::styled("/ ", self.theme.command_prompt),
                Span::styled(&self.input_buffer, self.theme.normal),
                Span::styled("_", self.theme.command_prompt),
            ])
        } else if focused {
            Line::from(Span::styled("Press / to search", self.theme.dimmed))
        } else {
            Line::from(Span::raw(""))
        };

        frame.render_widget(Paragraph::new(input_line), layout[1]);
    }

    fn last_area(&self) -> Option<Rect> {
        self.area
    }
}
