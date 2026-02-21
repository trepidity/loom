use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;
use tui_tree_widget::{Tree, TreeItem, TreeState};

use crate::action::{Action, ConnectionId};
use crate::config::ConnectionProfile;
use crate::theme::Theme;

use std::collections::BTreeMap;

/// Info about an active connection for display in the tree.
#[derive(Debug, Clone)]
pub struct ActiveConnInfo {
    pub id: ConnectionId,
    pub label: String,
}

/// Left panel in Connections layout: folder tree of saved profiles + active connections.
pub struct ConnectionsTree {
    pub tree_state: TreeState<String>,
    theme: Theme,
    /// Maps tree item keys like "profile:0" to the profile index
    profile_keys: Vec<(String, usize)>,
    /// Maps tree item keys like "active:0" to the connection id
    active_keys: Vec<(String, ConnectionId)>,
    /// Maps tree item keys like "folder:Production" to folder paths
    folder_keys: Vec<(String, String)>,
}

impl ConnectionsTree {
    pub fn new(theme: Theme) -> Self {
        Self {
            tree_state: TreeState::default(),
            theme,
            profile_keys: Vec::new(),
            active_keys: Vec::new(),
            folder_keys: Vec::new(),
        }
    }

    /// Get the currently selected key from the tree state.
    fn selected_key(&self) -> Option<&String> {
        self.tree_state.selected().last()
    }

    /// Get the profile index for the currently selected item, if it's a profile.
    fn selected_profile_index(&self) -> Option<usize> {
        let key = self.selected_key()?;
        self.profile_keys
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, idx)| *idx)
    }

    /// Get the folder path for the currently selected item, if it's a folder.
    fn selected_folder_path(&self) -> Option<&str> {
        let key = self.selected_key()?;
        self.folder_keys
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, path)| path.as_str())
    }

    /// Get the connection id for the currently selected item, if it's an active connection.
    #[allow(dead_code)]
    fn selected_active_id(&self) -> Option<ConnectionId> {
        let key = self.selected_key()?;
        self.active_keys
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, id)| *id)
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.tree_state.key_up();
                self.on_selection_changed()
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.tree_state.key_down();
                self.on_selection_changed()
            }
            KeyCode::Right | KeyCode::Char('l') => {
                self.tree_state.toggle_selected();
                self.on_selection_changed()
            }
            KeyCode::Left | KeyCode::Char('h') => {
                self.tree_state.key_left();
                Action::None
            }
            KeyCode::Char('c') => {
                if let Some(idx) = self.selected_profile_index() {
                    Action::ConnMgrConnect(idx)
                } else {
                    Action::None
                }
            }
            KeyCode::Char('d') | KeyCode::Delete => {
                if let Some(idx) = self.selected_profile_index() {
                    Action::ShowConfirm(
                        "Delete this connection profile?".to_string(),
                        Box::new(Action::ConnMgrDelete(idx)),
                    )
                } else {
                    Action::None
                }
            }
            KeyCode::Char('n') => Action::ConnMgrNew,
            KeyCode::Char('e') => {
                if let Some(idx) = self.selected_profile_index() {
                    Action::ConnMgrSelect(idx)
                } else {
                    Action::None
                }
            }
            KeyCode::Char('x') => Action::ConnMgrExport,
            KeyCode::Char('i') => Action::ConnMgrImport,
            _ => Action::None,
        }
    }

    fn on_selection_changed(&self) -> Action {
        if let Some(idx) = self.selected_profile_index() {
            Action::ConnMgrSelect(idx)
        } else if let Some(path) = self.selected_folder_path() {
            Action::ConnMgrSelectFolder(path.to_string())
        } else {
            Action::None
        }
    }

    /// Build tree items from profiles and active connections.
    pub fn build_tree_items(
        &mut self,
        profiles: &[ConnectionProfile],
        active: &[ActiveConnInfo],
    ) -> Vec<TreeItem<'static, String>> {
        self.profile_keys.clear();
        self.active_keys.clear();
        self.folder_keys.clear();

        let mut top_items: Vec<TreeItem<'static, String>> = Vec::new();

        // Active connections section
        if !active.is_empty() {
            let mut active_children = Vec::new();
            for info in active {
                let key = format!("active:{}", info.id);
                self.active_keys.push((key.clone(), info.id));
                let item = TreeItem::new_leaf(key, format!("* {}", info.label));
                active_children.push(item);
            }
            let active_section = TreeItem::new(
                "section:active".to_string(),
                "Active".to_string(),
                active_children,
            )
            .expect("tree item");
            top_items.push(active_section);
        }

        // Group profiles by folder
        let mut folders: BTreeMap<String, Vec<(usize, &ConnectionProfile)>> = BTreeMap::new();
        let mut ungrouped: Vec<(usize, &ConnectionProfile)> = Vec::new();

        for (idx, profile) in profiles.iter().enumerate() {
            if let Some(ref folder) = profile.folder {
                folders
                    .entry(folder.clone())
                    .or_default()
                    .push((idx, profile));
            } else {
                ungrouped.push((idx, profile));
            }
        }

        // Build folder tree nodes
        // We need to handle nested folders like "System/Production"
        let mut folder_tree: BTreeMap<String, Vec<TreeItem<'static, String>>> = BTreeMap::new();

        for (folder_path, profiles_in_folder) in &folders {
            let parts: Vec<&str> = folder_path.split('/').collect();
            let folder_str = folder_path.as_str();
            let leaf_folder = parts.last().unwrap_or(&folder_str);

            let mut children = Vec::new();
            for (idx, profile) in profiles_in_folder {
                let key = format!("profile:{}", idx);
                self.profile_keys.push((key.clone(), *idx));
                let item = TreeItem::new_leaf(key, profile.name.clone());
                children.push(item);
            }

            if parts.len() == 1 {
                // Simple folder
                let folder_key = format!("folder:{}", folder_path);
                self.folder_keys
                    .push((folder_key.clone(), folder_path.clone()));
                let folder_item = TreeItem::new(folder_key, leaf_folder.to_string(), children)
                    .expect("tree item");
                top_items.push(folder_item);
            } else {
                // Nested: group under top-level
                let top_key = parts[0].to_string();
                let sub_key = parts[1..].join("/");
                let sub_folder_key = format!("folder:{}", folder_path);
                self.folder_keys
                    .push((sub_folder_key.clone(), folder_path.clone()));
                let sub_item =
                    TreeItem::new(sub_folder_key, sub_key.clone(), children).expect("tree item");
                folder_tree.entry(top_key).or_default().push(sub_item);
            }
        }

        // Merge nested folder groups
        for (top_name, sub_items) in folder_tree {
            let folder_key = format!("folder:{}", top_name);
            self.folder_keys
                .push((folder_key.clone(), top_name.clone()));
            let folder_item = TreeItem::new(folder_key, top_name, sub_items).expect("tree item");
            top_items.push(folder_item);
        }

        // Ungrouped profiles at root level
        for (idx, profile) in &ungrouped {
            let key = format!("profile:{}", idx);
            self.profile_keys.push((key.clone(), *idx));
            let item = TreeItem::new_leaf(key, profile.name.clone());
            top_items.push(item);
        }

        // "+ New..." at the bottom
        top_items.push(TreeItem::new_leaf(
            "action:new".to_string(),
            "+ New...".to_string(),
        ));

        top_items
    }

    pub fn render_with_items(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        focused: bool,
        items: &[TreeItem<'_, String>],
    ) {
        let border_style = if focused {
            self.theme.border_focused
        } else {
            self.theme.border
        };

        let mut block = Block::default()
            .title(" Profiles ")
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
    }

    pub fn render_empty(&self, frame: &mut Frame, area: Rect, focused: bool) {
        let border_style = if focused {
            self.theme.border_focused
        } else {
            self.theme.border
        };

        let mut block = Block::default()
            .title(" Profiles ")
            .borders(Borders::ALL)
            .border_style(border_style);
        if focused {
            block = block.border_type(BorderType::Double);
        }

        let empty = Paragraph::new("No profiles configured")
            .style(self.theme.dimmed)
            .block(block);
        frame.render_widget(empty, area);
    }
}
