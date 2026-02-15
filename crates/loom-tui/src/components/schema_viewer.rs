use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Row, Table, TableState, Tabs};
use ratatui::Frame;

use crate::action::Action;
use crate::components::popup::Popup;
use crate::theme::Theme;
use loom_core::schema::{AttributeTypeInfo, ObjectClassInfo, ObjectClassKind, SchemaCache};

/// Which tab is active in the schema viewer.
#[derive(Debug, Clone, Copy, PartialEq)]
enum SchemaTab {
    ObjectClasses,
    AttributeTypes,
}

/// A full-screen schema browser overlay.
pub struct SchemaViewer {
    pub visible: bool,
    popup: Popup,
    theme: Theme,
    tab: SchemaTab,
    // Object classes
    oc_items: Vec<ObjectClassInfo>,
    oc_state: TableState,
    // Attribute types
    at_items: Vec<AttributeTypeInfo>,
    at_state: TableState,
    // Filter
    filter: String,
    filter_active: bool,
}

impl SchemaViewer {
    pub fn new(theme: Theme) -> Self {
        Self {
            visible: false,
            popup: Popup::new("Schema Browser", theme.clone()).with_size(80, 80),
            theme,
            tab: SchemaTab::ObjectClasses,
            oc_items: Vec::new(),
            oc_state: TableState::default(),
            at_items: Vec::new(),
            at_state: TableState::default(),
            filter: String::new(),
            filter_active: false,
        }
    }

    pub fn show(&mut self, schema: &SchemaCache) {
        // Deduplicate by OID (schema entries are stored per-name)
        let mut seen_oc = std::collections::HashSet::new();
        self.oc_items = schema
            .object_classes
            .values()
            .filter(|oc| seen_oc.insert(oc.oid.clone()))
            .cloned()
            .collect();
        self.oc_items.sort_by(|a, b| {
            a.names
                .first()
                .unwrap_or(&a.oid)
                .cmp(b.names.first().unwrap_or(&b.oid))
        });

        let mut seen_at = std::collections::HashSet::new();
        self.at_items = schema
            .attribute_types
            .values()
            .filter(|at| seen_at.insert(at.oid.clone()))
            .cloned()
            .collect();
        self.at_items.sort_by(|a, b| {
            a.names
                .first()
                .unwrap_or(&a.oid)
                .cmp(b.names.first().unwrap_or(&b.oid))
        });

        self.oc_state.select(if self.oc_items.is_empty() {
            None
        } else {
            Some(0)
        });
        self.at_state.select(if self.at_items.is_empty() {
            None
        } else {
            Some(0)
        });

        self.filter.clear();
        self.filter_active = false;
        self.visible = true;
        self.popup.show();
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.popup.hide();
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        if self.filter_active {
            return self.handle_filter_key(key);
        }

        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.hide();
                Action::ClosePopup
            }
            KeyCode::Tab => {
                self.tab = match self.tab {
                    SchemaTab::ObjectClasses => SchemaTab::AttributeTypes,
                    SchemaTab::AttributeTypes => SchemaTab::ObjectClasses,
                };
                Action::None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.move_selection(-1);
                Action::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.move_selection(1);
                Action::None
            }
            KeyCode::Char('/') => {
                self.filter_active = true;
                self.filter.clear();
                Action::None
            }
            _ => Action::None,
        }
    }

    fn handle_filter_key(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Esc => {
                self.filter_active = false;
                self.filter.clear();
                Action::None
            }
            KeyCode::Enter => {
                self.filter_active = false;
                Action::None
            }
            KeyCode::Backspace => {
                self.filter.pop();
                Action::None
            }
            KeyCode::Char(c) => {
                self.filter.push(c);
                Action::None
            }
            _ => Action::None,
        }
    }

    fn move_selection(&mut self, delta: i32) {
        let len = match self.tab {
            SchemaTab::ObjectClasses => self.filtered_oc_count(),
            SchemaTab::AttributeTypes => self.filtered_at_count(),
        };

        if len == 0 {
            return;
        }

        let state = match self.tab {
            SchemaTab::ObjectClasses => &mut self.oc_state,
            SchemaTab::AttributeTypes => &mut self.at_state,
        };

        let current = state.selected().unwrap_or(0) as i32;
        let next = (current + delta).clamp(0, len as i32 - 1) as usize;
        state.select(Some(next));
    }

    fn filtered_oc_count(&self) -> usize {
        if self.filter.is_empty() {
            self.oc_items.len()
        } else {
            let f = self.filter.to_lowercase();
            self.oc_items
                .iter()
                .filter(|oc| {
                    oc.names.iter().any(|n| n.to_lowercase().contains(&f))
                        || oc
                            .description
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&f)
                })
                .count()
        }
    }

    fn filtered_at_count(&self) -> usize {
        if self.filter.is_empty() {
            self.at_items.len()
        } else {
            let f = self.filter.to_lowercase();
            self.at_items
                .iter()
                .filter(|at| {
                    at.names.iter().any(|n| n.to_lowercase().contains(&f))
                        || at
                            .description
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&f)
                })
                .count()
        }
    }

    pub fn render(&mut self, frame: &mut Frame, full: Rect) {
        if !self.visible {
            return;
        }

        let area = self.popup.centered_area(full);
        frame.render_widget(Clear, area);

        let block = Block::default()
            .title(" Schema Browser ")
            .borders(Borders::ALL)
            .border_style(self.theme.popup_border)
            .title_style(self.theme.popup_title);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        // Layout: tabs (1) | table (flex) | filter/status (1)
        let layout = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(inner);

        // Tab bar
        let tab_titles = vec![
            Line::from(Span::raw("Object Classes")),
            Line::from(Span::raw("Attribute Types")),
        ];
        let selected_tab = match self.tab {
            SchemaTab::ObjectClasses => 0,
            SchemaTab::AttributeTypes => 1,
        };
        let tabs = Tabs::new(tab_titles)
            .select(selected_tab)
            .style(self.theme.tab_inactive)
            .highlight_style(self.theme.tab_active);
        frame.render_widget(tabs, layout[0]);

        // Table
        match self.tab {
            SchemaTab::ObjectClasses => self.render_oc_table(frame, layout[1]),
            SchemaTab::AttributeTypes => self.render_at_table(frame, layout[1]),
        }

        // Filter line
        let filter_line = if self.filter_active {
            Line::from(vec![
                Span::styled("/ ", self.theme.command_prompt),
                Span::styled(&self.filter, self.theme.normal),
                Span::styled("_", self.theme.command_prompt),
            ])
        } else if !self.filter.is_empty() {
            Line::from(vec![
                Span::styled("Filter: ", self.theme.dimmed),
                Span::styled(&self.filter, self.theme.normal),
                Span::styled(" (/ to edit, Esc to clear)", self.theme.dimmed),
            ])
        } else {
            Line::from(Span::styled(
                "Tab:switch  \u{2191}/\u{2193}:navigate  /:filter  q:close",
                self.theme.dimmed,
            ))
        };
        frame.render_widget(ratatui::widgets::Paragraph::new(filter_line), layout[2]);
    }

    fn render_oc_table(&mut self, frame: &mut Frame, area: Rect) {
        let filter_lower = self.filter.to_lowercase();
        let filtered: Vec<&ObjectClassInfo> = if self.filter.is_empty() {
            self.oc_items.iter().collect()
        } else {
            self.oc_items
                .iter()
                .filter(|oc| {
                    oc.names
                        .iter()
                        .any(|n| n.to_lowercase().contains(&filter_lower))
                        || oc
                            .description
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&filter_lower)
                })
                .collect()
        };

        let rows: Vec<Row> = filtered
            .iter()
            .map(|oc| {
                let name = oc.names.first().map(|s| s.as_str()).unwrap_or(&oc.oid);
                let kind = match oc.kind {
                    ObjectClassKind::Abstract => "ABSTRACT",
                    ObjectClassKind::Structural => "STRUCTURAL",
                    ObjectClassKind::Auxiliary => "AUXILIARY",
                };
                let sup = oc.superior.as_deref().unwrap_or("-");
                let must = oc.must.join(", ");
                let may_str = if oc.may.len() > 3 {
                    format!("{}... (+{})", oc.may[..3].join(", "), oc.may.len() - 3)
                } else {
                    oc.may.join(", ")
                };

                Row::new(vec![
                    Cell::from(Span::styled(name, self.theme.normal)),
                    Cell::from(Span::styled(kind, self.theme.dimmed)),
                    Cell::from(Span::styled(sup, self.theme.dimmed)),
                    Cell::from(Span::styled(must, self.theme.warning)),
                    Cell::from(Span::styled(may_str, self.theme.normal)),
                ])
            })
            .collect();

        let widths = [
            Constraint::Percentage(25),
            Constraint::Percentage(12),
            Constraint::Percentage(13),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ];

        let table = Table::new(rows, widths)
            .header(
                Row::new(vec![
                    Cell::from(Span::styled("Name", self.theme.header)),
                    Cell::from(Span::styled("Kind", self.theme.header)),
                    Cell::from(Span::styled("Superior", self.theme.header)),
                    Cell::from(Span::styled("MUST", self.theme.header)),
                    Cell::from(Span::styled("MAY", self.theme.header)),
                ])
                .style(self.theme.header),
            )
            .highlight_style(self.theme.selected.add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(table, area, &mut self.oc_state);
    }

    fn render_at_table(&mut self, frame: &mut Frame, area: Rect) {
        let filter_lower = self.filter.to_lowercase();
        let filtered: Vec<&AttributeTypeInfo> = if self.filter.is_empty() {
            self.at_items.iter().collect()
        } else {
            self.at_items
                .iter()
                .filter(|at| {
                    at.names
                        .iter()
                        .any(|n| n.to_lowercase().contains(&filter_lower))
                        || at
                            .description
                            .as_deref()
                            .unwrap_or("")
                            .to_lowercase()
                            .contains(&filter_lower)
                })
                .collect()
        };

        let rows: Vec<Row> = filtered
            .iter()
            .map(|at| {
                let name = at.names.first().map(|s| s.as_str()).unwrap_or(&at.oid);
                let syntax = format!("{:?}", at.syntax);
                let sv = if at.single_value { "Yes" } else { "No" };
                let desc = at.description.as_deref().unwrap_or("-");

                Row::new(vec![
                    Cell::from(Span::styled(name, self.theme.normal)),
                    Cell::from(Span::styled(syntax, self.theme.dimmed)),
                    Cell::from(Span::styled(sv, self.theme.dimmed)),
                    Cell::from(Span::styled(desc, self.theme.normal)),
                ])
            })
            .collect();

        let widths = [
            Constraint::Percentage(25),
            Constraint::Percentage(20),
            Constraint::Percentage(10),
            Constraint::Percentage(45),
        ];

        let table = Table::new(rows, widths)
            .header(
                Row::new(vec![
                    Cell::from(Span::styled("Name", self.theme.header)),
                    Cell::from(Span::styled("Syntax", self.theme.header)),
                    Cell::from(Span::styled("Single?", self.theme.header)),
                    Cell::from(Span::styled("Description", self.theme.header)),
                ])
                .style(self.theme.header),
            )
            .highlight_style(self.theme.selected.add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(table, area, &mut self.at_state);
    }
}
