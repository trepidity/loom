use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::action::Action;
use crate::components::popup::Popup;
use crate::config::{AppConfig, ConnectionProfile};
use crate::theme::Theme;

/// Which part of the dialog is active.
#[derive(Debug, Clone, Copy, PartialEq)]
enum ActiveField {
    ProfileList,
    Filename,
}

/// Dialog for exporting connection profiles to a TOML file.
pub struct ProfileExportDialog {
    pub visible: bool,
    popup: Popup,
    theme: Theme,
    active_field: ActiveField,
    /// Profile names and their selected state.
    profiles: Vec<(String, bool)>,
    /// Cursor position in the profile list.
    cursor: usize,
    /// Output filename.
    filename: String,
}

impl ProfileExportDialog {
    pub fn new(theme: Theme) -> Self {
        Self {
            visible: false,
            popup: Popup::new("Export Profiles", theme.clone()).with_size(55, 60),
            theme,
            active_field: ActiveField::ProfileList,
            profiles: Vec::new(),
            cursor: 0,
            filename: String::new(),
        }
    }

    /// Show the dialog populated with the given profiles.
    pub fn show(&mut self, profiles: &[ConnectionProfile]) {
        self.profiles = profiles
            .iter()
            .map(|p| (p.name.clone(), true))
            .collect();
        self.cursor = 0;
        self.filename = "profiles.toml".to_string();
        self.active_field = ActiveField::ProfileList;
        self.visible = true;
        self.popup.show();
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.popup.hide();
    }

    pub fn handle_key_event(
        &mut self,
        key: KeyEvent,
        all_profiles: &[ConnectionProfile],
    ) -> Action {
        match key.code {
            KeyCode::Esc => {
                self.hide();
                Action::ClosePopup
            }
            KeyCode::Tab => {
                self.active_field = match self.active_field {
                    ActiveField::ProfileList => ActiveField::Filename,
                    ActiveField::Filename => ActiveField::ProfileList,
                };
                Action::None
            }
            KeyCode::BackTab => {
                self.active_field = match self.active_field {
                    ActiveField::ProfileList => ActiveField::Filename,
                    ActiveField::Filename => ActiveField::ProfileList,
                };
                Action::None
            }
            KeyCode::Enter => self.submit(all_profiles),
            _ => match self.active_field {
                ActiveField::ProfileList => self.handle_list_key(key),
                ActiveField::Filename => self.handle_filename_key(key),
            },
        }
    }

    fn handle_list_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                Action::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.cursor + 1 < self.profiles.len() {
                    self.cursor += 1;
                }
                Action::None
            }
            KeyCode::Char(' ') => {
                if let Some(item) = self.profiles.get_mut(self.cursor) {
                    item.1 = !item.1;
                }
                Action::None
            }
            KeyCode::Char('a') => {
                let all_selected = self.profiles.iter().all(|(_, sel)| *sel);
                for item in &mut self.profiles {
                    item.1 = !all_selected;
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_filename_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Backspace => {
                self.filename.pop();
                Action::None
            }
            KeyCode::Char(c) => {
                self.filename.push(c);
                Action::None
            }
            _ => Action::None,
        }
    }

    fn submit(&mut self, all_profiles: &[ConnectionProfile]) -> Action {
        if self.filename.trim().is_empty() {
            return Action::ErrorMessage("Filename is required".to_string());
        }

        let selected: Vec<ConnectionProfile> = self
            .profiles
            .iter()
            .enumerate()
            .filter(|(_, (_, sel))| *sel)
            .filter_map(|(i, _)| all_profiles.get(i).cloned())
            .collect();

        if selected.is_empty() {
            return Action::ErrorMessage("No profiles selected".to_string());
        }

        let content = match AppConfig::export_profiles(&selected) {
            Ok(c) => c,
            Err(e) => {
                self.hide();
                return Action::ErrorMessage(e);
            }
        };

        // Expand ~/
        let path = expand_tilde(self.filename.trim());

        if let Err(e) = std::fs::write(&path, &content) {
            self.hide();
            return Action::ErrorMessage(format!("Failed to write {}: {}", path, e));
        }

        self.hide();
        Action::StatusMessage(format!(
            "Exported {} profile(s) to {}",
            selected.len(),
            path
        ))
    }

    pub fn render(&self, frame: &mut Frame, full: Rect) {
        if !self.visible {
            return;
        }

        let area = self.popup.centered_area(full);
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(" Export Profiles ")
            .borders(Borders::ALL)
            .border_style(self.theme.popup_border)
            .title_style(self.theme.popup_title);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        let list_height = self.profiles.len().max(1) as u16 + 1; // +1 for label
        let layout = Layout::vertical([
            Constraint::Length(list_height), // Profile list
            Constraint::Length(2),           // Filename
            Constraint::Min(1),             // Hints
        ])
        .split(inner);

        // Profile list
        let list_active = self.active_field == ActiveField::ProfileList;
        let label_style = if list_active {
            self.theme.header
        } else {
            self.theme.dimmed
        };

        let mut lines = vec![Line::from(Span::styled("Profiles:", label_style))];
        for (i, (name, selected)) in self.profiles.iter().enumerate() {
            let marker = if *selected { "[x] " } else { "[ ] " };
            let is_cursor = list_active && i == self.cursor;
            let style = if is_cursor {
                self.theme.selected.add_modifier(Modifier::BOLD)
            } else if *selected {
                self.theme.normal
            } else {
                self.theme.dimmed
            };
            let prefix = if is_cursor { "> " } else { "  " };
            lines.push(Line::from(Span::styled(
                format!("{}{}{}", prefix, marker, name),
                style,
            )));
        }
        frame.render_widget(Paragraph::new(lines), layout[0]);

        // Filename field
        let fn_active = self.active_field == ActiveField::Filename;
        let fn_label_style = if fn_active {
            self.theme.header
        } else {
            self.theme.dimmed
        };
        let fn_value_style = if fn_active {
            self.theme.normal
        } else {
            self.theme.dimmed
        };
        let fn_lines = vec![
            Line::from(Span::styled("Filename:", fn_label_style)),
            Line::from(vec![
                Span::styled(&self.filename, fn_value_style),
                if fn_active {
                    Span::styled("_", self.theme.command_prompt)
                } else {
                    Span::raw("")
                },
            ]),
        ];
        frame.render_widget(Paragraph::new(fn_lines), layout[1]);

        // Hints
        let hint_text = if list_active {
            "Space:toggle  a:all  Tab:filename  Enter:export  Esc:cancel"
        } else {
            "Tab:profiles  Enter:export  Esc:cancel"
        };
        let hints = Paragraph::new(Line::from(Span::styled(hint_text, self.theme.dimmed)));
        frame.render_widget(hints, layout[2]);
    }
}

/// Expand a leading `~/` to the user's home directory.
pub fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest).to_string_lossy().to_string();
        }
    }
    path.to_string()
}
