use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use loom_core::connection::TlsMode;
use loom_core::credentials::CredentialMethod;

use crate::action::Action;
use crate::config::ConnectionProfile;
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq)]
enum FormMode {
    View,
    Edit,
    Create,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Field {
    Name,
    Host,
    Port,
    BindDn,
    BaseDn,
    Folder,
    TlsMode,
    CredentialMethod,
    PasswordCommand,
    PageSize,
    Timeout,
    RelaxRules,
}

impl Field {
    fn next(self) -> Self {
        match self {
            Field::Name => Field::Host,
            Field::Host => Field::Port,
            Field::Port => Field::BindDn,
            Field::BindDn => Field::BaseDn,
            Field::BaseDn => Field::Folder,
            Field::Folder => Field::TlsMode,
            Field::TlsMode => Field::CredentialMethod,
            Field::CredentialMethod => Field::PasswordCommand,
            Field::PasswordCommand => Field::PageSize,
            Field::PageSize => Field::Timeout,
            Field::Timeout => Field::RelaxRules,
            Field::RelaxRules => Field::Name,
        }
    }

    fn prev(self) -> Self {
        match self {
            Field::Name => Field::RelaxRules,
            Field::Host => Field::Name,
            Field::Port => Field::Host,
            Field::BindDn => Field::Port,
            Field::BaseDn => Field::BindDn,
            Field::Folder => Field::BaseDn,
            Field::TlsMode => Field::Folder,
            Field::CredentialMethod => Field::TlsMode,
            Field::PasswordCommand => Field::CredentialMethod,
            Field::PageSize => Field::PasswordCommand,
            Field::Timeout => Field::PageSize,
            Field::RelaxRules => Field::Timeout,
        }
    }
}

/// Right panel in Connections layout: view/edit/create form for ConnectionProfile.
pub struct ConnectionForm {
    mode: FormMode,
    theme: Theme,
    active_field: Field,
    /// Index of the profile being viewed/edited (None for Create mode)
    profile_index: Option<usize>,

    // Form fields (string buffers for editing)
    name: String,
    host: String,
    port: String,
    bind_dn: String,
    base_dn: String,
    folder: String,
    tls_mode: TlsMode,
    credential_method: CredentialMethod,
    password_command: String,
    page_size: String,
    timeout: String,
    relax_rules: bool,
}

impl ConnectionForm {
    pub fn new(theme: Theme) -> Self {
        Self {
            mode: FormMode::View,
            theme,
            active_field: Field::Name,
            profile_index: None,
            name: String::new(),
            host: String::new(),
            port: "389".to_string(),
            bind_dn: String::new(),
            base_dn: String::new(),
            folder: String::new(),
            tls_mode: TlsMode::Auto,
            credential_method: CredentialMethod::Prompt,
            password_command: String::new(),
            page_size: "500".to_string(),
            timeout: "30".to_string(),
            relax_rules: false,
        }
    }

    /// Load a profile for viewing.
    pub fn view_profile(&mut self, index: usize, profile: &ConnectionProfile) {
        self.mode = FormMode::View;
        self.profile_index = Some(index);
        self.load_from_profile(profile);
    }

    /// Switch to edit mode (must be viewing a profile).
    pub fn edit_profile(&mut self) {
        if self.profile_index.is_some() {
            self.mode = FormMode::Edit;
            self.active_field = Field::Name;
        }
    }

    /// Clear form and switch to create mode.
    pub fn new_profile(&mut self) {
        self.mode = FormMode::Create;
        self.profile_index = None;
        self.active_field = Field::Name;
        self.name.clear();
        self.host.clear();
        self.port = "389".to_string();
        self.bind_dn.clear();
        self.base_dn.clear();
        self.folder.clear();
        self.tls_mode = TlsMode::Auto;
        self.credential_method = CredentialMethod::Prompt;
        self.password_command.clear();
        self.page_size = "500".to_string();
        self.timeout = "30".to_string();
        self.relax_rules = false;
    }

    /// Clear the form (no profile selected).
    pub fn clear(&mut self) {
        self.mode = FormMode::View;
        self.profile_index = None;
        self.name.clear();
        self.host.clear();
        self.port.clear();
        self.bind_dn.clear();
        self.base_dn.clear();
        self.folder.clear();
        self.password_command.clear();
        self.page_size.clear();
        self.timeout.clear();
        self.relax_rules = false;
    }

    fn load_from_profile(&mut self, profile: &ConnectionProfile) {
        self.name = profile.name.clone();
        self.host = profile.host.clone();
        self.port = profile.port.to_string();
        self.bind_dn = profile.bind_dn.clone().unwrap_or_default();
        self.base_dn = profile.base_dn.clone().unwrap_or_default();
        self.folder = profile.folder.clone().unwrap_or_default();
        self.tls_mode = profile.tls_mode.clone();
        self.credential_method = profile.credential_method.clone();
        self.password_command = profile.password_command.clone().unwrap_or_default();
        self.page_size = profile.page_size.to_string();
        self.timeout = profile.timeout_secs.to_string();
        self.relax_rules = profile.relax_rules;
    }

    fn to_profile(&self) -> Result<ConnectionProfile, String> {
        if self.host.trim().is_empty() {
            return Err("Host is required".to_string());
        }
        let port: u16 = self
            .port
            .parse()
            .map_err(|_| "Port must be a valid number (1-65535)".to_string())?;
        let page_size: u32 = self
            .page_size
            .parse()
            .map_err(|_| "Page size must be a valid number".to_string())?;
        let timeout: u64 = self
            .timeout
            .parse()
            .map_err(|_| "Timeout must be a valid number".to_string())?;

        let name = if self.name.trim().is_empty() {
            format!("{}:{}", self.host.trim(), port)
        } else {
            self.name.trim().to_string()
        };

        Ok(ConnectionProfile {
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
            folder: if self.folder.trim().is_empty() {
                None
            } else {
                Some(self.folder.trim().to_string())
            },
            credential_method: self.credential_method.clone(),
            password_command: if self.password_command.trim().is_empty() {
                None
            } else {
                Some(self.password_command.trim().to_string())
            },
            page_size,
            timeout_secs: timeout,
            relax_rules: self.relax_rules,
        })
    }

    fn submit(&mut self) -> Action {
        match self.to_profile() {
            Ok(profile) => match self.mode {
                FormMode::Edit => {
                    if let Some(idx) = self.profile_index {
                        Action::ConnMgrSave(idx, Box::new(profile))
                    } else {
                        Action::None
                    }
                }
                FormMode::Create => Action::ConnMgrCreate(Box::new(profile)),
                FormMode::View => Action::None,
            },
            Err(msg) => Action::ErrorMessage(msg),
        }
    }

    fn active_buffer_mut(&mut self) -> Option<&mut String> {
        match self.active_field {
            Field::Name => Some(&mut self.name),
            Field::Host => Some(&mut self.host),
            Field::Port => Some(&mut self.port),
            Field::BindDn => Some(&mut self.bind_dn),
            Field::BaseDn => Some(&mut self.base_dn),
            Field::Folder => Some(&mut self.folder),
            Field::PasswordCommand => Some(&mut self.password_command),
            Field::PageSize => Some(&mut self.page_size),
            Field::Timeout => Some(&mut self.timeout),
            // These are cycled with special keys, not typed
            Field::TlsMode | Field::CredentialMethod | Field::RelaxRules => None,
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        match self.mode {
            FormMode::View => self.handle_view_key(key),
            FormMode::Edit | FormMode::Create => self.handle_edit_key(key),
        }
    }

    fn handle_view_key(&mut self, key: KeyEvent) -> Action {
        if self.profile_index.is_none() {
            return Action::None;
        }
        match key.code {
            KeyCode::Char('e') => {
                self.edit_profile();
                Action::None
            }
            KeyCode::Char('c') => {
                if let Some(idx) = self.profile_index {
                    Action::ConnMgrConnect(idx)
                } else {
                    Action::None
                }
            }
            KeyCode::Char('d') => {
                if let Some(idx) = self.profile_index {
                    Action::ShowConfirm(
                        "Delete this connection profile?".to_string(),
                        Box::new(Action::ConnMgrDelete(idx)),
                    )
                } else {
                    Action::None
                }
            }
            _ => Action::None,
        }
    }

    fn handle_edit_key(&mut self, key: KeyEvent) -> Action {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) => {
                // Cancel: revert to view or clear
                if self.mode == FormMode::Edit {
                    self.mode = FormMode::View;
                } else {
                    self.clear();
                }
                Action::None
            }
            (KeyModifiers::NONE, KeyCode::Tab) => {
                self.active_field = self.active_field.next();
                Action::None
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                self.active_field = self.active_field.prev();
                Action::None
            }
            (KeyModifiers::NONE, KeyCode::F(2)) => {
                // Cycle TLS mode
                self.tls_mode = self.tls_mode.next();
                Action::None
            }
            (KeyModifiers::NONE, KeyCode::F(3)) => {
                // Cycle credential method
                self.credential_method = match self.credential_method {
                    CredentialMethod::Prompt => CredentialMethod::Command,
                    CredentialMethod::Command => CredentialMethod::Keychain,
                    CredentialMethod::Keychain => CredentialMethod::Prompt,
                };
                Action::None
            }
            (KeyModifiers::NONE, KeyCode::F(10)) | (KeyModifiers::CONTROL, KeyCode::Enter) => {
                self.submit()
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                // Enter on toggle fields acts like a cycle
                match self.active_field {
                    Field::TlsMode => {
                        self.tls_mode = self.tls_mode.next();
                        Action::None
                    }
                    Field::CredentialMethod => {
                        self.credential_method = match self.credential_method {
                            CredentialMethod::Prompt => CredentialMethod::Command,
                            CredentialMethod::Command => CredentialMethod::Keychain,
                            CredentialMethod::Keychain => CredentialMethod::Prompt,
                        };
                        Action::None
                    }
                    Field::RelaxRules => {
                        self.relax_rules = !self.relax_rules;
                        Action::None
                    }
                    _ => self.submit(),
                }
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                if let Some(buf) = self.active_buffer_mut() {
                    buf.pop();
                }
                Action::None
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) | (KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                // For numeric-only fields, reject non-digits
                if matches!(
                    self.active_field,
                    Field::Port | Field::PageSize | Field::Timeout
                ) && !c.is_ascii_digit()
                {
                    return Action::None;
                }
                // Toggle fields: space toggles
                if self.active_field == Field::RelaxRules {
                    self.relax_rules = !self.relax_rules;
                    return Action::None;
                }
                if let Some(buf) = self.active_buffer_mut() {
                    buf.push(c);
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused {
            self.theme.border_focused
        } else {
            self.theme.border
        };

        let title = match self.mode {
            FormMode::View => " Profile ",
            FormMode::Edit => " Edit Profile ",
            FormMode::Create => " New Profile ",
        };

        let mut block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(border_style);
        if focused {
            block = block.border_type(BorderType::Double);
        }

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.profile_index.is_none() && self.mode == FormMode::View {
            let empty = Paragraph::new("Select a profile or press 'n' to create one")
                .style(self.theme.dimmed);
            frame.render_widget(empty, inner);
            return;
        }

        let editable = self.mode != FormMode::View;

        // Layout: 12 fields at 2 lines each + hints
        let layout = Layout::vertical([
            Constraint::Length(2), // Name
            Constraint::Length(2), // Host
            Constraint::Length(2), // Port
            Constraint::Length(2), // Bind DN
            Constraint::Length(2), // Base DN
            Constraint::Length(2), // Folder
            Constraint::Length(2), // TLS Mode
            Constraint::Length(2), // Credential Method
            Constraint::Length(2), // Password Command
            Constraint::Length(2), // Page Size
            Constraint::Length(2), // Timeout
            Constraint::Length(2), // Relax Rules
            Constraint::Min(1),    // Hints
        ])
        .split(inner);

        self.render_field(frame, layout[0], "Name", &self.name, Field::Name, editable);
        self.render_field(frame, layout[1], "Host", &self.host, Field::Host, editable);
        self.render_field(frame, layout[2], "Port", &self.port, Field::Port, editable);
        self.render_field(
            frame,
            layout[3],
            "Bind DN",
            &self.bind_dn,
            Field::BindDn,
            editable,
        );
        self.render_field(
            frame,
            layout[4],
            "Base DN",
            &self.base_dn,
            Field::BaseDn,
            editable,
        );
        self.render_field(
            frame,
            layout[5],
            "Folder",
            &self.folder,
            Field::Folder,
            editable,
        );

        // TLS Mode (special: shows label, not a text buffer)
        self.render_field(
            frame,
            layout[6],
            "TLS Mode",
            self.tls_mode.label(),
            Field::TlsMode,
            editable,
        );

        // Credential Method
        let cred_label = match self.credential_method {
            CredentialMethod::Prompt => "Prompt",
            CredentialMethod::Command => "Command",
            CredentialMethod::Keychain => "Keychain",
        };
        self.render_field(
            frame,
            layout[7],
            "Credential",
            cred_label,
            Field::CredentialMethod,
            editable,
        );

        self.render_field(
            frame,
            layout[8],
            "Password Cmd",
            &self.password_command,
            Field::PasswordCommand,
            editable,
        );
        self.render_field(
            frame,
            layout[9],
            "Page Size",
            &self.page_size,
            Field::PageSize,
            editable,
        );
        self.render_field(
            frame,
            layout[10],
            "Timeout (s)",
            &self.timeout,
            Field::Timeout,
            editable,
        );

        // Relax Rules (boolean toggle)
        let relax_str = if self.relax_rules { "Yes" } else { "No" };
        self.render_field(
            frame,
            layout[11],
            "Relax Rules",
            relax_str,
            Field::RelaxRules,
            editable,
        );

        // Hints
        let hints_text = match self.mode {
            FormMode::View => "e:Edit  c:Connect  d:Delete",
            FormMode::Edit => "Tab:next  F2:TLS  F3:Cred  F10:Save  Esc:Cancel",
            FormMode::Create => "Tab:next  F2:TLS  F3:Cred  F10:Save  Esc:Cancel",
        };
        let hints = Paragraph::new(Line::from(Span::styled(hints_text, self.theme.dimmed)));
        frame.render_widget(hints, layout[12]);
    }

    fn render_field(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        value: &str,
        field: Field,
        editable: bool,
    ) {
        let is_active = editable && self.active_field == field;
        let label_style = if is_active {
            self.theme.header
        } else {
            self.theme.dimmed
        };
        let value_style = self.theme.normal;

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
