use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::action::Action;
use crate::components::popup::Popup;
use crate::components::profile_export_dialog::expand_tilde;
use crate::config::{AppConfig, ConnectionProfile};
use crate::theme::Theme;

/// Which phase the import dialog is in.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    /// User enters a file path.
    FilePath,
    /// User selects which profiles to import.
    SelectProfiles,
}

/// Dialog for importing connection profiles from a TOML file.
pub struct ProfileImportDialog {
    pub visible: bool,
    popup: Popup,
    theme: Theme,
    phase: Phase,
    /// File path input.
    file_path: String,
    /// Parsed profiles with (name, host, selected).
    parsed_profiles: Vec<(ConnectionProfile, bool)>,
    /// Cursor position in profile list.
    cursor: usize,
}

impl ProfileImportDialog {
    pub fn new(theme: Theme) -> Self {
        Self {
            visible: false,
            popup: Popup::new("Import Profiles", theme.clone()).with_size(55, 60),
            theme,
            phase: Phase::FilePath,
            file_path: String::new(),
            parsed_profiles: Vec::new(),
            cursor: 0,
        }
    }

    pub fn show(&mut self) {
        self.phase = Phase::FilePath;
        self.file_path = "profiles.toml".to_string();
        self.parsed_profiles.clear();
        self.cursor = 0;
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
                if self.phase == Phase::SelectProfiles {
                    // Go back to file path phase
                    self.phase = Phase::FilePath;
                    self.parsed_profiles.clear();
                    return Action::None;
                }
                self.hide();
                Action::ClosePopup
            }
            KeyCode::Enter => match self.phase {
                Phase::FilePath => self.open_file(),
                Phase::SelectProfiles => self.submit(),
            },
            _ => match self.phase {
                Phase::FilePath => self.handle_filepath_key(key),
                Phase::SelectProfiles => self.handle_select_key(key),
            },
        }
    }

    fn handle_filepath_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Backspace => {
                self.file_path.pop();
                Action::None
            }
            KeyCode::Char(c) => {
                self.file_path.push(c);
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_select_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
                Action::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.cursor + 1 < self.parsed_profiles.len() {
                    self.cursor += 1;
                }
                Action::None
            }
            KeyCode::Char(' ') => {
                if let Some(item) = self.parsed_profiles.get_mut(self.cursor) {
                    item.1 = !item.1;
                }
                Action::None
            }
            KeyCode::Char('a') => {
                let all_selected = self.parsed_profiles.iter().all(|(_, sel)| *sel);
                for item in &mut self.parsed_profiles {
                    item.1 = !all_selected;
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    fn open_file(&mut self) -> Action {
        if self.file_path.trim().is_empty() {
            return Action::ErrorMessage("File path is required".to_string());
        }

        let path = expand_tilde(self.file_path.trim());
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                return Action::ErrorMessage(format!("Failed to read {}: {}", path, e));
            }
        };

        match AppConfig::import_profiles(&content) {
            Ok(profiles) => {
                self.parsed_profiles = profiles.into_iter().map(|p| (p, true)).collect();
                self.cursor = 0;
                self.phase = Phase::SelectProfiles;
                Action::None
            }
            Err(e) => Action::ErrorMessage(e),
        }
    }

    fn submit(&mut self) -> Action {
        let selected: Vec<ConnectionProfile> = self
            .parsed_profiles
            .iter()
            .filter(|(_, sel)| *sel)
            .map(|(p, _)| p.clone())
            .collect();

        if selected.is_empty() {
            return Action::ErrorMessage("No profiles selected".to_string());
        }

        self.hide();
        Action::ConnMgrImportExecute(selected)
    }

    pub fn render(&self, frame: &mut Frame, full: Rect) {
        if !self.visible {
            return;
        }

        let area = self.popup.centered_area(full);
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(" Import Profiles ")
            .borders(Borders::ALL)
            .border_style(self.theme.popup_border)
            .title_style(self.theme.popup_title);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        match self.phase {
            Phase::FilePath => self.render_filepath(frame, inner),
            Phase::SelectProfiles => self.render_select(frame, inner),
        }
    }

    fn render_filepath(&self, frame: &mut Frame, area: Rect) {
        let layout = Layout::vertical([
            Constraint::Length(2), // File path
            Constraint::Min(1),   // Hints
        ])
        .split(area);

        let lines = vec![
            Line::from(Span::styled("File path:", self.theme.header)),
            Line::from(vec![
                Span::styled(&self.file_path, self.theme.normal),
                Span::styled("_", self.theme.command_prompt),
            ]),
        ];
        frame.render_widget(Paragraph::new(lines), layout[0]);

        let hints = Paragraph::new(Line::from(Span::styled(
            "Enter:open file  Esc:cancel",
            self.theme.dimmed,
        )));
        frame.render_widget(hints, layout[1]);
    }

    fn render_select(&self, frame: &mut Frame, area: Rect) {
        let list_height = self.parsed_profiles.len().max(1) as u16 + 1; // +1 for label
        let layout = Layout::vertical([
            Constraint::Length(list_height), // Profile list
            Constraint::Min(1),             // Hints
        ])
        .split(area);

        let mut lines = vec![Line::from(Span::styled(
            format!("Found {} profile(s):", self.parsed_profiles.len()),
            self.theme.header,
        ))];

        for (i, (profile, selected)) in self.parsed_profiles.iter().enumerate() {
            let marker = if *selected { "[x] " } else { "[ ] " };
            let is_cursor = i == self.cursor;
            let style = if is_cursor {
                self.theme.selected.add_modifier(Modifier::BOLD)
            } else if *selected {
                self.theme.normal
            } else {
                self.theme.dimmed
            };
            let prefix = if is_cursor { "> " } else { "  " };
            let label = format!("{} ({}:{})", profile.name, profile.host, profile.port);
            lines.push(Line::from(Span::styled(
                format!("{}{}{}", prefix, marker, label),
                style,
            )));
        }
        frame.render_widget(Paragraph::new(lines), layout[0]);

        let hints = Paragraph::new(Line::from(Span::styled(
            "Space:toggle  a:all  Enter:import  Esc:back",
            self.theme.dimmed,
        )));
        frame.render_widget(hints, layout[1]);
    }
}
