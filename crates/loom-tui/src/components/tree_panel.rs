use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;
use tui_tree_widget::{Tree, TreeItem, TreeState};

use crate::action::{Action, ContextMenuSource};
use crate::theme::Theme;
use loom_core::tree::TreeNode;

/// The left panel: directory tree browser.
pub struct TreePanel {
    pub tree_state: TreeState<String>,
    pub theme: Theme,
    area: Option<Rect>,
}

impl TreePanel {
    pub fn new(theme: Theme) -> Self {
        Self {
            tree_state: TreeState::default(),
            theme,
            area: None,
        }
    }

    /// Build tree items from the directory tree for rendering.
    pub fn build_tree_items(node: &TreeNode) -> Vec<TreeItem<'static, String>> {
        let mut items = Vec::new();

        if let Some(ref children) = node.children {
            for child in children {
                let child_items = Self::build_tree_items(child);
                let item = TreeItem::new(child.dn.clone(), child.display_name.clone(), child_items)
                    .expect("tree item creation");
                items.push(item);
            }
        }

        items
    }

    /// Get the currently selected DN.
    pub fn selected_dn(&self) -> Option<&String> {
        self.tree_state.selected().last()
    }

    /// Handle key events, mutating tree state.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.tree_state.key_up();
                if let Some(dn) = self.selected_dn().cloned() {
                    Action::TreeSelect(dn)
                } else {
                    Action::None
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.tree_state.key_down();
                if let Some(dn) = self.selected_dn().cloned() {
                    Action::TreeSelect(dn)
                } else {
                    Action::None
                }
            }
            KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => {
                if let Some(dn) = self.selected_dn().cloned() {
                    self.tree_state.toggle_selected();
                    Action::TreeExpand(dn)
                } else {
                    Action::None
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if let Some(dn) = self.selected_dn().cloned() {
                    self.tree_state.key_left();
                    Action::TreeCollapse(dn)
                } else {
                    Action::None
                }
            }
            KeyCode::PageUp => {
                self.tree_state
                    .select_relative(|current| current.map_or(0, |c| c.saturating_sub(10)));
                if let Some(dn) = self.selected_dn().cloned() {
                    Action::TreeSelect(dn)
                } else {
                    Action::None
                }
            }
            KeyCode::PageDown => {
                self.tree_state
                    .select_relative(|current| current.map_or(0, |c| c.saturating_add(10)));
                if let Some(dn) = self.selected_dn().cloned() {
                    Action::TreeSelect(dn)
                } else {
                    Action::None
                }
            }
            KeyCode::Char('a') => {
                if let Some(dn) = self.selected_dn().cloned() {
                    Action::ShowCreateEntryDialog(dn)
                } else {
                    Action::None
                }
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                if let Some(dn) = self.selected_dn().cloned() {
                    let msg = format!("Delete entry?\n{}", dn);
                    Action::ShowConfirm(msg, Box::new(Action::DeleteEntry(dn)))
                } else {
                    Action::None
                }
            }
            KeyCode::Char(' ') => {
                if let Some(dn) = self.selected_dn().cloned() {
                    Action::ShowContextMenu(ContextMenuSource::Tree { dn })
                } else {
                    Action::None
                }
            }
            _ => Action::None,
        }
    }

    /// Render the tree panel with items and mutable state access.
    pub fn render_with_items(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        focused: bool,
        items: &[TreeItem<'_, String>],
        title: &str,
    ) {
        let border_style = if focused {
            self.theme.border_focused
        } else {
            self.theme.border
        };

        let mut block = Block::default()
            .title(format!(" {} ", title))
            .borders(Borders::ALL)
            .border_style(border_style);
        if focused {
            block = block.border_type(BorderType::Double);
        }

        let tree_widget = Tree::new(items)
            .expect("tree widget")
            .block(block)
            .highlight_style(self.theme.tree_node_selected.add_modifier(Modifier::BOLD));

        frame.render_stateful_widget(tree_widget, area, &mut self.tree_state);
        self.area = Some(area);
    }

    /// Render an empty placeholder (no connection).
    pub fn render_empty(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused {
            self.theme.border_focused
        } else {
            self.theme.border
        };

        let mut block = Block::default()
            .title(" Tree ")
            .borders(Borders::ALL)
            .border_style(border_style);
        if focused {
            block = block.border_type(BorderType::Double);
        }

        let empty = Paragraph::new("No connection")
            .style(self.theme.dimmed)
            .block(block);
        frame.render_widget(empty, area);
    }
}
