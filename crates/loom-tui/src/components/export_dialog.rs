use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::action::Action;
use crate::components::popup::Popup;
use crate::theme::Theme;

/// Export format options.
const FORMATS: &[(&str, &str)] = &[
    ("LDIF", ".ldif"),
    ("JSON", ".json"),
    ("CSV", ".csv"),
    ("Excel", ".xlsx"),
];

/// Which field is currently active.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ExportField {
    Filter,
    Attributes,
    Format,
    Filename,
}

/// Dialog for exporting entries to a file.
pub struct ExportDialog {
    pub visible: bool,
    popup: Popup,
    theme: Theme,
    active_field: ExportField,
    format_idx: usize,
    filter: String,
    attributes: String,
    filename: String,
}

impl ExportDialog {
    pub fn new(theme: Theme) -> Self {
        Self {
            visible: false,
            popup: Popup::new("Export Entries", theme.clone()).with_size(60, 55),
            theme,
            active_field: ExportField::Filter,
            format_idx: 0,
            filter: String::new(),
            attributes: String::new(),
            filename: String::new(),
        }
    }

    pub fn show(&mut self, _entry_count: usize) {
        self.filter = "(objectClass=*)".to_string();
        self.attributes = "*".to_string();
        self.format_idx = 0;
        self.filename = format!("export{}", FORMATS[0].1);
        self.active_field = ExportField::Filter;
        self.visible = true;
        self.popup.show();
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.popup.hide();
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => {
                self.hide();
                Action::ClosePopup
            }
            KeyCode::Tab => {
                self.active_field = match self.active_field {
                    ExportField::Filter => ExportField::Attributes,
                    ExportField::Attributes => ExportField::Format,
                    ExportField::Format => ExportField::Filename,
                    ExportField::Filename => ExportField::Filter,
                };
                Action::None
            }
            KeyCode::BackTab => {
                self.active_field = match self.active_field {
                    ExportField::Filter => ExportField::Filename,
                    ExportField::Attributes => ExportField::Filter,
                    ExportField::Format => ExportField::Attributes,
                    ExportField::Filename => ExportField::Format,
                };
                Action::None
            }
            KeyCode::F(2) if self.active_field == ExportField::Format => {
                self.format_idx = (self.format_idx + 1) % FORMATS.len();
                self.update_filename_ext();
                Action::None
            }
            KeyCode::Up | KeyCode::Char('k') if self.active_field == ExportField::Format => {
                if self.format_idx > 0 {
                    self.format_idx -= 1;
                    self.update_filename_ext();
                }
                Action::None
            }
            KeyCode::Down | KeyCode::Char('j') if self.active_field == ExportField::Format => {
                if self.format_idx + 1 < FORMATS.len() {
                    self.format_idx += 1;
                    self.update_filename_ext();
                }
                Action::None
            }
            KeyCode::Enter => self.submit(),
            KeyCode::Backspace => {
                if let Some(buf) = self.active_text_buffer_mut() {
                    buf.pop();
                }
                Action::None
            }
            KeyCode::Char(c) => {
                if let Some(buf) = self.active_text_buffer_mut() {
                    buf.push(c);
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn submit(&mut self) -> Action {
        if self.filter.trim().is_empty() {
            return Action::ErrorMessage("Search filter is required".to_string());
        }
        if self.filename.trim().is_empty() {
            return Action::ErrorMessage("Filename is required".to_string());
        }

        let path = self.filename.trim().to_string();
        let filter = self.filter.trim().to_string();
        let attributes = self.attributes.trim().to_string();

        // Parse attributes: comma or space separated, or "*" for all
        let attrs: Vec<String> = if attributes.is_empty() || attributes == "*" {
            vec!["*".to_string()]
        } else {
            attributes
                .split(|c: char| c == ',' || c.is_whitespace())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        };

        self.hide();
        Action::ExportExecute {
            path,
            filter,
            attributes: attrs,
        }
    }

    /// Returns mutable reference to the active text field, or None for Format.
    fn active_text_buffer_mut(&mut self) -> Option<&mut String> {
        match self.active_field {
            ExportField::Filter => Some(&mut self.filter),
            ExportField::Attributes => Some(&mut self.attributes),
            ExportField::Filename => Some(&mut self.filename),
            ExportField::Format => None,
        }
    }

    fn update_filename_ext(&mut self) {
        let ext = FORMATS[self.format_idx].1;
        if let Some(dot_pos) = self.filename.rfind('.') {
            self.filename.truncate(dot_pos);
        }
        self.filename.push_str(ext);
    }

    pub fn render(&self, frame: &mut Frame, full: Rect) {
        if !self.visible {
            return;
        }

        let area = self.popup.centered_area(full);
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(" Export Entries ")
            .borders(Borders::ALL)
            .border_style(self.theme.popup_border)
            .title_style(self.theme.popup_title);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Layout: filter(2) | attributes(2) | format(formats+1) | filename(2) | hints(1)
        let layout = Layout::vertical([
            Constraint::Length(2),                        // Filter
            Constraint::Length(2),                        // Attributes
            Constraint::Length(FORMATS.len() as u16 + 1), // Format
            Constraint::Length(2),                        // Filename
            Constraint::Min(1),                           // Hints
        ])
        .split(inner);

        // Filter field
        self.render_text_field(
            frame,
            layout[0],
            "Search Filter",
            &self.filter,
            ExportField::Filter,
        );

        // Attributes field
        self.render_text_field(
            frame,
            layout[1],
            "Attributes",
            &self.attributes,
            ExportField::Attributes,
        );

        // Format selector
        let format_active = self.active_field == ExportField::Format;
        let format_label_style = if format_active {
            self.theme.header
        } else {
            self.theme.dimmed
        };
        let mut format_lines = vec![Line::from(Span::styled("Format:", format_label_style))];
        for (i, (name, ext)) in FORMATS.iter().enumerate() {
            let marker = if i == self.format_idx { "> " } else { "  " };
            let style = if i == self.format_idx && format_active {
                self.theme.selected.add_modifier(Modifier::BOLD)
            } else if i == self.format_idx {
                self.theme.normal.add_modifier(Modifier::BOLD)
            } else if format_active {
                self.theme.normal
            } else {
                self.theme.dimmed
            };
            format_lines.push(Line::from(Span::styled(
                format!("{}{} ({})", marker, name, ext),
                style,
            )));
        }
        frame.render_widget(Paragraph::new(format_lines), layout[2]);

        // Filename field
        self.render_text_field(
            frame,
            layout[3],
            "Filename",
            &self.filename,
            ExportField::Filename,
        );

        // Hints
        let hint_text = if format_active {
            "Tab:next  \u{2191}/\u{2193}:select  F2:cycle  Enter:export  Esc:cancel"
        } else {
            "Tab:next  Enter:export  Esc:cancel"
        };
        let hints = Paragraph::new(Line::from(Span::styled(hint_text, self.theme.dimmed)));
        frame.render_widget(hints, layout[4]);
    }

    fn render_text_field(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        value: &str,
        field: ExportField,
    ) {
        let is_active = self.active_field == field;
        let label_style = if is_active {
            self.theme.header
        } else {
            self.theme.dimmed
        };
        let value_style = if is_active {
            self.theme.normal
        } else {
            self.theme.dimmed
        };

        let lines = vec![
            Line::from(Span::styled(format!("{}:", label), label_style)),
            Line::from(vec![
                Span::styled(value, value_style),
                if is_active {
                    Span::styled("_", self.theme.command_prompt)
                } else {
                    Span::raw("")
                },
            ]),
        ];
        frame.render_widget(Paragraph::new(lines), area);
    }
}
