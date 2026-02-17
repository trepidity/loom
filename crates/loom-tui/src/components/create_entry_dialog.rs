use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::action::Action;
use crate::components::popup::Popup;
use crate::theme::Theme;

/// Which field is currently active.
#[derive(Debug, Clone, Copy, PartialEq)]
enum CreateField {
    Rdn,
    ObjectClasses,
    Attributes,
}

/// Dialog for creating a new LDAP entry under a selected parent DN.
pub struct CreateEntryDialog {
    pub visible: bool,
    popup: Popup,
    theme: Theme,
    active_field: CreateField,
    parent_dn: String,
    rdn: String,
    object_classes: String,
    extra_attributes: String,
}

impl CreateEntryDialog {
    pub fn new(theme: Theme) -> Self {
        Self {
            visible: false,
            popup: Popup::new("Create Entry", theme.clone()).with_size(60, 50),
            theme,
            active_field: CreateField::Rdn,
            parent_dn: String::new(),
            rdn: String::new(),
            object_classes: String::new(),
            extra_attributes: String::new(),
        }
    }

    pub fn show(&mut self, parent_dn: String) {
        self.parent_dn = parent_dn;
        self.rdn.clear();
        self.object_classes.clear();
        self.extra_attributes.clear();
        self.active_field = CreateField::Rdn;
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
                    CreateField::Rdn => CreateField::ObjectClasses,
                    CreateField::ObjectClasses => CreateField::Attributes,
                    CreateField::Attributes => CreateField::Rdn,
                };
                Action::None
            }
            KeyCode::BackTab => {
                self.active_field = match self.active_field {
                    CreateField::Rdn => CreateField::Attributes,
                    CreateField::ObjectClasses => CreateField::Rdn,
                    CreateField::Attributes => CreateField::ObjectClasses,
                };
                Action::None
            }
            KeyCode::Enter => self.submit(),
            KeyCode::Backspace => {
                self.active_text_buffer_mut().pop();
                Action::None
            }
            KeyCode::Char(c) => {
                self.active_text_buffer_mut().push(c);
                Action::None
            }
            _ => Action::None,
        }
    }

    fn active_text_buffer_mut(&mut self) -> &mut String {
        match self.active_field {
            CreateField::Rdn => &mut self.rdn,
            CreateField::ObjectClasses => &mut self.object_classes,
            CreateField::Attributes => &mut self.extra_attributes,
        }
    }

    fn submit(&mut self) -> Action {
        let rdn = self.rdn.trim();
        if rdn.is_empty() {
            return Action::ErrorMessage("RDN is required (e.g. cn=NewUser)".to_string());
        }
        if !rdn.contains('=') {
            return Action::ErrorMessage("RDN must contain '=' (e.g. cn=NewUser)".to_string());
        }

        let oc_input = self.object_classes.trim();
        if oc_input.is_empty() {
            return Action::ErrorMessage("At least one objectClass is required".to_string());
        }

        let full_dn = format!("{},{}", rdn, self.parent_dn);

        let mut attributes: Vec<(String, Vec<String>)> = Vec::new();

        // Add objectClass values
        let oc_values: Vec<String> = oc_input
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        attributes.push(("objectClass".to_string(), oc_values));

        // Add the RDN attribute (e.g. cn=NewUser -> cn: NewUser)
        if let Some(eq_pos) = rdn.find('=') {
            let rdn_attr = rdn[..eq_pos].trim().to_string();
            let rdn_val = rdn[eq_pos + 1..].trim().to_string();
            if !rdn_val.is_empty() {
                attributes.push((rdn_attr, vec![rdn_val]));
            }
        }

        // Parse additional attributes (comma-separated attr=value pairs)
        let extra = self.extra_attributes.trim();
        if !extra.is_empty() {
            for pair in extra.split(',') {
                let pair = pair.trim();
                if let Some(eq_pos) = pair.find('=') {
                    let attr = pair[..eq_pos].trim().to_string();
                    let val = pair[eq_pos + 1..].trim().to_string();
                    if !attr.is_empty() && !val.is_empty() {
                        // Check if we already have this attribute, and merge values
                        if let Some(existing) = attributes.iter_mut().find(|(a, _)| a == &attr) {
                            existing.1.push(val);
                        } else {
                            attributes.push((attr, vec![val]));
                        }
                    }
                }
            }
        }

        self.hide();
        Action::CreateEntry {
            dn: full_dn,
            attributes,
        }
    }

    pub fn render(&self, frame: &mut Frame, full: Rect) {
        if !self.visible {
            return;
        }

        let area = self.popup.centered_area(full);
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(" Create Entry ")
            .borders(Borders::ALL)
            .border_style(self.theme.popup_border)
            .title_style(self.theme.popup_title);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Layout: parent_dn(2) | rdn(2) | objectClasses(2) | extra_attrs(2) | preview(2) | hints(1)
        let layout = Layout::vertical([
            Constraint::Length(2), // Parent DN (read-only)
            Constraint::Length(2), // RDN
            Constraint::Length(2), // Object Classes
            Constraint::Length(2), // Additional Attributes
            Constraint::Length(2), // Preview full DN
            Constraint::Min(1),    // Hints
        ])
        .split(inner);

        // Parent DN (read-only)
        let parent_lines = vec![
            Line::from(Span::styled("Parent DN:", self.theme.dimmed)),
            Line::from(Span::styled(self.parent_dn.as_str(), self.theme.normal)),
        ];
        frame.render_widget(Paragraph::new(parent_lines), layout[0]);

        // RDN field
        self.render_text_field(
            frame,
            layout[1],
            "RDN (e.g. cn=NewUser)",
            &self.rdn,
            CreateField::Rdn,
        );

        // Object Classes field
        self.render_text_field(
            frame,
            layout[2],
            "Object Classes (comma-separated)",
            &self.object_classes,
            CreateField::ObjectClasses,
        );

        // Additional Attributes field
        self.render_text_field(
            frame,
            layout[3],
            "Extra Attributes (attr=val, ...)",
            &self.extra_attributes,
            CreateField::Attributes,
        );

        // Preview full DN
        let rdn = self.rdn.trim();
        let preview = if rdn.is_empty() {
            "...".to_string()
        } else {
            format!("{},{}", rdn, self.parent_dn)
        };
        let preview_lines = vec![
            Line::from(Span::styled("Full DN:", self.theme.dimmed)),
            Line::from(Span::styled(preview, self.theme.header)),
        ];
        frame.render_widget(Paragraph::new(preview_lines), layout[4]);

        // Hints
        let hints = Paragraph::new(Line::from(Span::styled(
            "Tab:next field  Enter:create  Esc:cancel",
            self.theme.dimmed,
        )));
        frame.render_widget(hints, layout[5]);
    }

    fn render_text_field(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        value: &str,
        field: CreateField,
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
