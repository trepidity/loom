use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Modifier;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use crate::action::Action;
use crate::theme::Theme;
use loom_core::entry::LdapEntry;

/// The search results panel, shown as an overlay when a search has results.
pub struct SearchDialog {
    pub visible: bool,
    pub filter: String,
    pub results: Vec<LdapEntry>,
    list_state: ListState,
    theme: Theme,
}

impl SearchDialog {
    pub fn new(theme: Theme) -> Self {
        Self {
            visible: false,
            filter: String::new(),
            results: Vec::new(),
            list_state: ListState::default(),
            theme,
        }
    }

    pub fn show_results(&mut self, filter: String, results: Vec<LdapEntry>) {
        self.filter = filter;
        self.results = results;
        self.list_state.select(if self.results.is_empty() {
            None
        } else {
            Some(0)
        });
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        if !self.visible {
            return Action::None;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.list_state.selected().unwrap_or(0);
                if i > 0 {
                    self.list_state.select(Some(i - 1));
                }
                Action::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let i = self.list_state.selected().unwrap_or(0);
                if i + 1 < self.results.len() {
                    self.list_state.select(Some(i + 1));
                }
                Action::None
            }
            KeyCode::Enter => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(entry) = self.results.get(idx) {
                        let dn = entry.dn.clone();
                        self.visible = false;
                        return Action::TreeSelect(dn);
                    }
                }
                Action::None
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                self.hide();
                Action::ClosePopup
            }
            _ => Action::None,
        }
    }

    pub fn render(&self, frame: &mut Frame, full: Rect) {
        if !self.visible {
            return;
        }

        let popup_width = (full.width as u32 * 80 / 100).min(100) as u16;
        let popup_height = (full.height as u32 * 70 / 100).min(40) as u16;

        let x = full.x + (full.width.saturating_sub(popup_width)) / 2;
        let y = full.y + (full.height.saturating_sub(popup_height)) / 2;
        let area = Rect::new(x, y, popup_width, popup_height);

        frame.render_widget(Clear, area);

        let title = format!(" Search: {} ({} results) ", self.filter, self.results.len());
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(self.theme.popup_border)
            .title_style(self.theme.popup_title);

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if self.results.is_empty() {
            let msg = Paragraph::new("No results found.").style(self.theme.dimmed);
            frame.render_widget(msg, inner);
            return;
        }

        // Layout: hint (1 line) | results list
        let layout = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(inner);

        let hint = Line::from(vec![
            Span::styled("  \u{2191}/\u{2193}", self.theme.header),
            Span::styled(": navigate  ", self.theme.dimmed),
            Span::styled("Enter", self.theme.header),
            Span::styled(": select  ", self.theme.dimmed),
            Span::styled("Esc", self.theme.header),
            Span::styled(": close", self.theme.dimmed),
        ]);
        frame.render_widget(Paragraph::new(hint), layout[0]);

        let items: Vec<ListItem> = self
            .results
            .iter()
            .map(|entry| {
                let oc = entry.object_classes().join(", ");
                let line = Line::from(vec![
                    Span::styled(&entry.dn, self.theme.normal),
                    Span::styled(format!("  [{}]", oc), self.theme.dimmed),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list =
            List::new(items).highlight_style(self.theme.selected.add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(list, layout[1], &mut self.list_state.clone());
    }
}
