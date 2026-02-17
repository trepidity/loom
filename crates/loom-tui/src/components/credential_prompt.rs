use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::action::Action;
use crate::components::popup::Popup;
use crate::config::ConnectionProfile;
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Field {
    BindDn,
    Password,
}

/// Dialog that prompts for bind credentials when a connection requires authentication.
pub struct CredentialPromptDialog {
    pub visible: bool,
    popup: Popup,
    theme: Theme,
    active_field: Field,
    bind_dn: String,
    password: String,
    profile: Option<ConnectionProfile>,
}

impl CredentialPromptDialog {
    pub fn new(theme: Theme) -> Self {
        Self {
            visible: false,
            popup: Popup::new("Credentials Required", theme.clone()).with_size(60, 30),
            theme,
            active_field: Field::Password,
            bind_dn: String::new(),
            password: String::new(),
            profile: None,
        }
    }

    pub fn show(&mut self, profile: ConnectionProfile) {
        self.bind_dn = profile.bind_dn.clone().unwrap_or_default();
        self.password.clear();
        // Focus password if bind_dn is pre-filled, otherwise focus bind_dn
        self.active_field = if self.bind_dn.is_empty() {
            Field::BindDn
        } else {
            Field::Password
        };
        self.profile = Some(profile);
        self.visible = true;
        self.popup.show();
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.popup.hide();
        self.password.clear();
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => {
                self.hide();
                Action::ClosePopup
            }
            KeyCode::Tab | KeyCode::BackTab => {
                self.active_field = match self.active_field {
                    Field::BindDn => Field::Password,
                    Field::Password => Field::BindDn,
                };
                Action::None
            }
            KeyCode::Enter => self.submit(),
            KeyCode::Backspace => {
                self.active_buffer_mut().pop();
                Action::None
            }
            KeyCode::Char(c) => {
                self.active_buffer_mut().push(c);
                Action::None
            }
            _ => Action::None,
        }
    }

    fn submit(&mut self) -> Action {
        let Some(mut profile) = self.profile.take() else {
            self.hide();
            return Action::ClosePopup;
        };

        if self.bind_dn.trim().is_empty() {
            self.profile = Some(profile);
            return Action::ErrorMessage("Bind DN is required".to_string());
        }

        // Update the profile's bind_dn with whatever the user entered
        profile.bind_dn = Some(self.bind_dn.trim().to_string());

        let password = self.password.clone();
        self.hide();
        Action::ConnectWithCredentials(profile, password)
    }

    fn active_buffer_mut(&mut self) -> &mut String {
        match self.active_field {
            Field::BindDn => &mut self.bind_dn,
            Field::Password => &mut self.password,
        }
    }

    pub fn render(&self, frame: &mut Frame, full: Rect) {
        if !self.visible {
            return;
        }

        let area = self.popup.centered_area(full);
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(" Credentials Required ")
            .borders(Borders::ALL)
            .border_style(self.theme.popup_border)
            .title_style(self.theme.popup_title);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Show which host we're connecting to
        let host_info = self
            .profile
            .as_ref()
            .map(|p| format!("{}:{}", p.host, p.port))
            .unwrap_or_default();

        let layout = Layout::vertical([
            Constraint::Length(2), // Host info
            Constraint::Length(2), // Bind DN
            Constraint::Length(2), // Password
            Constraint::Min(1),    // Hints
        ])
        .split(inner);

        // Host info
        let info_line = vec![
            Line::from(vec![
                Span::styled("Server: ", self.theme.dimmed),
                Span::styled(host_info, self.theme.normal),
            ]),
            Line::from(Span::raw("")),
        ];
        frame.render_widget(Paragraph::new(info_line), layout[0]);

        // Bind DN field
        self.render_field(
            frame,
            layout[1],
            "Bind DN",
            &self.bind_dn,
            Field::BindDn,
            false,
        );

        // Password field
        self.render_field(
            frame,
            layout[2],
            "Password",
            &self.password,
            Field::Password,
            true,
        );

        // Hints
        let hints = Paragraph::new(Line::from(Span::styled(
            "Tab:switch field  Enter:connect  Esc:cancel",
            self.theme.dimmed,
        )));
        frame.render_widget(hints, layout[3]);
    }

    fn render_field(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        value: &str,
        field: Field,
        masked: bool,
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

        let display_value = if masked && !value.is_empty() {
            "*".repeat(value.len())
        } else {
            value.to_string()
        };

        let lines = vec![
            Line::from(Span::styled(format!("{}:", label), label_style)),
            Line::from(vec![
                Span::styled(display_value, value_style),
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
