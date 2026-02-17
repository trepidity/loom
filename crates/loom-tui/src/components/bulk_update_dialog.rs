use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::action::Action;
use crate::components::popup::Popup;
use crate::theme::Theme;

/// Which field is currently being edited.
#[derive(Debug, Clone, Copy, PartialEq)]
enum BulkField {
    Filter,
    Attribute,
    Value,
}

/// Operation type for the bulk update.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BulkOp {
    Replace,
    Add,
    Delete,
}

impl BulkOp {
    fn label(&self) -> &'static str {
        match self {
            BulkOp::Replace => "Replace",
            BulkOp::Add => "Add",
            BulkOp::Delete => "Delete",
        }
    }

    fn next(&self) -> Self {
        match self {
            BulkOp::Replace => BulkOp::Add,
            BulkOp::Add => BulkOp::Delete,
            BulkOp::Delete => BulkOp::Replace,
        }
    }
}

/// Dialog for specifying bulk update parameters.
pub struct BulkUpdateDialog {
    pub visible: bool,
    popup: Popup,
    theme: Theme,
    active_field: BulkField,
    pub filter: String,
    pub attribute: String,
    pub value: String,
    pub op: BulkOp,
}

impl BulkUpdateDialog {
    pub fn new(theme: Theme) -> Self {
        Self {
            visible: false,
            popup: Popup::new("Bulk Update", theme.clone()).with_size(60, 45),
            theme,
            active_field: BulkField::Filter,
            filter: String::new(),
            attribute: String::new(),
            value: String::new(),
            op: BulkOp::Replace,
        }
    }

    pub fn show(&mut self) {
        self.filter.clear();
        self.attribute.clear();
        self.value.clear();
        self.op = BulkOp::Replace;
        self.active_field = BulkField::Filter;
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
                    BulkField::Filter => BulkField::Attribute,
                    BulkField::Attribute => BulkField::Value,
                    BulkField::Value => BulkField::Filter,
                };
                Action::None
            }
            KeyCode::BackTab => {
                self.active_field = match self.active_field {
                    BulkField::Filter => BulkField::Value,
                    BulkField::Attribute => BulkField::Filter,
                    BulkField::Value => BulkField::Attribute,
                };
                Action::None
            }
            KeyCode::F(2) => {
                self.op = self.op.next();
                Action::None
            }
            KeyCode::Enter => {
                if self.filter.is_empty() || self.attribute.is_empty() {
                    return Action::ErrorMessage("Filter and attribute are required".to_string());
                }
                let filter = self.filter.clone();
                let attr = self.attribute.clone();
                let value = self.value.clone();
                let op = self.op;
                self.hide();
                Action::BulkUpdateExecute {
                    filter,
                    attribute: attr,
                    value,
                    op,
                }
            }
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

    fn active_buffer_mut(&mut self) -> &mut String {
        match self.active_field {
            BulkField::Filter => &mut self.filter,
            BulkField::Attribute => &mut self.attribute,
            BulkField::Value => &mut self.value,
        }
    }

    pub fn render(&self, frame: &mut Frame, full: Rect) {
        if !self.visible {
            return;
        }

        let area = self.popup.centered_area(full);
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(" Bulk Update ")
            .borders(Borders::ALL)
            .border_style(self.theme.popup_border)
            .title_style(self.theme.popup_title);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Layout: operation (2) | filter (2) | attribute (2) | value (2) | hints (flex)
        let layout = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Length(2),
            Constraint::Min(1),
        ])
        .split(inner);

        // Operation
        let op_line = vec![
            Line::from(vec![
                Span::styled("Operation: ", self.theme.header),
                Span::styled(self.op.label(), self.theme.success),
                Span::styled("  (F2 to cycle)", self.theme.dimmed),
            ]),
            Line::from(Span::raw("")),
        ];
        frame.render_widget(Paragraph::new(op_line), layout[0]);

        // Filter field
        self.render_field(frame, layout[1], "Filter", &self.filter, BulkField::Filter);

        // Attribute field
        self.render_field(
            frame,
            layout[2],
            "Attribute",
            &self.attribute,
            BulkField::Attribute,
        );

        // Value field
        self.render_field(frame, layout[3], "Value", &self.value, BulkField::Value);

        // Hints
        let hints = Paragraph::new(Line::from(Span::styled(
            "Tab:next field  F2:operation  Enter:execute  Esc:cancel",
            self.theme.dimmed,
        )));
        frame.render_widget(hints, layout[4]);
    }

    fn render_field(
        &self,
        frame: &mut Frame,
        area: Rect,
        label: &str,
        value: &str,
        field: BulkField,
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
