use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use loom_core::connection::TlsMode;
use loom_core::credentials::CredentialMethod;

use crate::action::Action;
use crate::components::popup::Popup;
use crate::config::ConnectionProfile;
use crate::theme::Theme;

/// Which field is currently being edited.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Field {
    Name,
    Host,
    Port,
    BindDn,
    BaseDn,
    Password,
}

impl Field {
    fn next(self) -> Self {
        match self {
            Field::Name => Field::Host,
            Field::Host => Field::Port,
            Field::Port => Field::BindDn,
            Field::BindDn => Field::BaseDn,
            Field::BaseDn => Field::Password,
            Field::Password => Field::Name,
        }
    }

    fn prev(self) -> Self {
        match self {
            Field::Name => Field::Password,
            Field::Host => Field::Name,
            Field::Port => Field::Host,
            Field::BindDn => Field::Port,
            Field::BaseDn => Field::BindDn,
            Field::Password => Field::BaseDn,
        }
    }
}

/// Dialog for creating an ad-hoc LDAP connection.
pub struct NewConnectionDialog {
    pub visible: bool,
    popup: Popup,
    theme: Theme,
    active_field: Field,
    name: String,
    host: String,
    port: String,
    bind_dn: String,
    base_dn: String,
    password: String,
    tls_mode: TlsMode,
}

impl NewConnectionDialog {
    pub fn new(theme: Theme) -> Self {
        Self {
            visible: false,
            popup: Popup::new("New Connection", theme.clone()).with_size(60, 55),
            theme,
            active_field: Field::Host,
            name: String::new(),
            host: String::new(),
            port: "389".to_string(),
            bind_dn: String::new(),
            base_dn: String::new(),
            password: String::new(),
            tls_mode: TlsMode::Auto,
        }
    }

    pub fn show(&mut self) {
        self.name.clear();
        self.host.clear();
        self.port = "389".to_string();
        self.bind_dn.clear();
        self.base_dn.clear();
        self.password.clear();
        self.tls_mode = TlsMode::Auto;
        self.active_field = Field::Host;
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
                self.active_field = self.active_field.next();
                Action::None
            }
            KeyCode::BackTab => {
                self.active_field = self.active_field.prev();
                Action::None
            }
            KeyCode::F(2) => {
                self.tls_mode = self.tls_mode.next();
                Action::None
            }
            KeyCode::Enter => self.submit(),
            KeyCode::Backspace => {
                self.active_buffer_mut().pop();
                Action::None
            }
            KeyCode::Char(c) => {
                // Port field: digits only
                if self.active_field == Field::Port && !c.is_ascii_digit() {
                    return Action::None;
                }
                self.active_buffer_mut().push(c);
                Action::None
            }
            _ => Action::None,
        }
    }

    fn submit(&mut self) -> Action {
        // Validate host
        if self.host.trim().is_empty() {
            return Action::ErrorMessage("Host is required".to_string());
        }

        // Validate port
        let port: u16 = match self.port.parse() {
            Ok(p) => p,
            Err(_) => {
                return Action::ErrorMessage("Port must be a valid number (1-65535)".to_string())
            }
        };

        // Auto-generate name if empty
        let name = if self.name.trim().is_empty() {
            format!("{}:{}", self.host.trim(), port)
        } else {
            self.name.trim().to_string()
        };

        let profile = ConnectionProfile {
            name,
            host: self.host.trim().to_string(),
            port,
            tls_mode: self.tls_mode.clone(),
            bind_dn: if self.bind_dn.trim().is_empty() {
                None
            } else {
                Some(self.bind_dn.trim().to_string())
            },
            base_dn: if self.base_dn.trim().is_empty() {
                None
            } else {
                Some(self.base_dn.trim().to_string())
            },
            credential_method: CredentialMethod::Prompt,
            password_command: None,
            page_size: 500,
            timeout_secs: 30,
            relax_rules: false,
            folder: None,
            offline: false,
        };

        let password = self.password.clone();
        self.hide();
        Action::ConnectAdHoc(profile, password)
    }

    fn active_buffer_mut(&mut self) -> &mut String {
        match self.active_field {
            Field::Name => &mut self.name,
            Field::Host => &mut self.host,
            Field::Port => &mut self.port,
            Field::BindDn => &mut self.bind_dn,
            Field::BaseDn => &mut self.base_dn,
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
            .title(" New Connection ")
            .borders(Borders::ALL)
            .border_style(self.theme.popup_border)
            .title_style(self.theme.popup_title);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Layout: tls(2) | name(2) | host(2) | port(2) | bind_dn(2) | base_dn(2) | password(2) | hints(flex)
        let layout = Layout::vertical([
            Constraint::Length(2), // TLS mode
            Constraint::Length(2), // Name
            Constraint::Length(2), // Host
            Constraint::Length(2), // Port
            Constraint::Length(2), // Bind DN
            Constraint::Length(2), // Base DN
            Constraint::Length(2), // Password
            Constraint::Min(1),    // Hints
        ])
        .split(inner);

        // TLS Mode
        let tls_line = vec![
            Line::from(vec![
                Span::styled("TLS Mode: ", self.theme.header),
                Span::styled(self.tls_mode.label(), self.theme.success),
                Span::styled("  (F2 to cycle)", self.theme.dimmed),
            ]),
            Line::from(Span::raw("")),
        ];
        frame.render_widget(Paragraph::new(tls_line), layout[0]);

        // Fields
        self.render_field(frame, layout[1], "Name", &self.name, Field::Name, false);
        self.render_field(frame, layout[2], "Host", &self.host, Field::Host, false);
        self.render_field(frame, layout[3], "Port", &self.port, Field::Port, false);
        self.render_field(
            frame,
            layout[4],
            "Bind DN",
            &self.bind_dn,
            Field::BindDn,
            false,
        );
        self.render_field(
            frame,
            layout[5],
            "Base DN",
            &self.base_dn,
            Field::BaseDn,
            false,
        );
        self.render_field(
            frame,
            layout[6],
            "Password",
            &self.password,
            Field::Password,
            true,
        );

        // Hints
        let hints = Paragraph::new(Line::from(Span::styled(
            "Tab:next  Shift+Tab:prev  F2:TLS  Enter:connect  Esc:cancel",
            self.theme.dimmed,
        )));
        frame.render_widget(hints, layout[7]);
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
