use std::sync::Arc;
use std::time::Duration;

use crossterm::event::MouseEventKind;
use ratatui::layout::{Constraint, Layout, Rect};
use tokio::sync::Mutex;
use tracing::{debug, error};

use loom_core::bulk::BulkMod;
use loom_core::connection::LdapConnection;
use loom_core::credentials::{CredentialMethod, CredentialProvider};
use loom_core::schema::SchemaCache;
use loom_core::tree::{DirectoryTree, TreeNode};

use crate::action::{Action, ConnectionId, FocusTarget};
use crate::component::Component;
use crate::components::attribute_editor::{AttributeEditor, EditOp};
use crate::components::bulk_update_dialog::BulkUpdateDialog;
use crate::components::command_panel::CommandPanel;
use crate::components::confirm_dialog::ConfirmDialog;
use crate::components::connect_dialog::ConnectDialog;
use crate::components::create_entry_dialog::CreateEntryDialog;
use crate::components::credential_prompt::CredentialPromptDialog;
use crate::components::detail_panel::DetailPanel;
use crate::components::export_dialog::ExportDialog;
use crate::components::log_panel::LogPanel;
use crate::components::new_connection_dialog::NewConnectionDialog;
use crate::components::schema_viewer::SchemaViewer;
use crate::components::search_dialog::SearchDialog;
use crate::components::status_bar::StatusBar;
use crate::components::tab_bar::TabBar;
use crate::components::tree_panel::TreePanel;
use crate::config::{AppConfig, ConnectionProfile};
use crate::event::{self, AppEvent};
use crate::focus::FocusManager;
use crate::keymap;
use crate::theme::Theme;
use crate::tui;

/// A single connection tab's state.
struct ConnectionTab {
    id: ConnectionId,
    label: String,
    host: String,
    server_type: String,
    connection: Arc<Mutex<LdapConnection>>,
    directory_tree: DirectoryTree,
    schema: Option<SchemaCache>,
}

/// The main application.
pub struct App {
    config: AppConfig,
    #[allow(dead_code)]
    theme: Theme,
    should_quit: bool,
    next_conn_id: ConnectionId,

    // Connection tabs
    tabs: Vec<ConnectionTab>,
    active_tab_id: Option<ConnectionId>,

    // UI components
    tab_bar: TabBar,
    tree_panel: TreePanel,
    detail_panel: DetailPanel,
    command_panel: CommandPanel,
    status_bar: StatusBar,
    focus: FocusManager,

    // Popup/dialogs
    confirm_dialog: ConfirmDialog,
    connect_dialog: ConnectDialog,
    new_connection_dialog: NewConnectionDialog,
    credential_prompt: CredentialPromptDialog,
    search_dialog: SearchDialog,
    attribute_editor: AttributeEditor,
    export_dialog: ExportDialog,
    bulk_update_dialog: BulkUpdateDialog,
    create_entry_dialog: CreateEntryDialog,
    schema_viewer: SchemaViewer,
    log_panel: LogPanel,

    // Ad-hoc connection tracking (for save-to-config)
    last_adhoc_profile: Option<ConnectionProfile>,

    // Track areas for mouse hit-testing
    tree_area: Option<Rect>,
    detail_area: Option<Rect>,
    command_area: Option<Rect>,
    tab_area: Option<Rect>,

    // Async communication
    action_tx: tokio::sync::mpsc::UnboundedSender<Action>,
    action_rx: tokio::sync::mpsc::UnboundedReceiver<Action>,
}

impl App {
    pub fn new(config: AppConfig) -> Self {
        let theme = Theme::load(&config.general.theme);
        let (action_tx, action_rx) = tokio::sync::mpsc::unbounded_channel();

        Self {
            config,
            theme: theme.clone(),
            should_quit: false,
            next_conn_id: 0,
            tabs: Vec::new(),
            active_tab_id: None,
            tab_bar: TabBar::new(theme.clone()),
            tree_panel: TreePanel::new(theme.clone()),
            detail_panel: DetailPanel::new(theme.clone()),
            command_panel: CommandPanel::new(theme.clone()),
            status_bar: StatusBar::new(theme.clone()),
            focus: FocusManager::new(),
            confirm_dialog: ConfirmDialog::new(theme.clone()),
            connect_dialog: ConnectDialog::new(theme.clone()),
            new_connection_dialog: NewConnectionDialog::new(theme.clone()),
            credential_prompt: CredentialPromptDialog::new(theme.clone()),
            search_dialog: SearchDialog::new(theme.clone()),
            attribute_editor: AttributeEditor::new(theme.clone()),
            export_dialog: ExportDialog::new(theme.clone()),
            bulk_update_dialog: BulkUpdateDialog::new(theme.clone()),
            create_entry_dialog: CreateEntryDialog::new(theme.clone()),
            schema_viewer: SchemaViewer::new(theme.clone()),
            log_panel: LogPanel::new(theme),
            last_adhoc_profile: None,
            tree_area: None,
            detail_area: None,
            command_area: None,
            tab_area: None,
            action_tx,
            action_rx,
        }
    }

    fn allocate_conn_id(&mut self) -> ConnectionId {
        let id = self.next_conn_id;
        self.next_conn_id += 1;
        id
    }

    fn active_tab(&self) -> Option<&ConnectionTab> {
        let id = self.active_tab_id?;
        self.tabs.iter().find(|t| t.id == id)
    }

    #[allow(dead_code)]
    fn active_tab_mut(&mut self) -> Option<&mut ConnectionTab> {
        let id = self.active_tab_id?;
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    /// Connect to the first configured connection profile.
    /// Auth errors are handled gracefully by showing a credential prompt.
    pub async fn connect_first_profile(&mut self) {
        if !self.config.connections.is_empty() {
            let profile = self.config.connections[0].clone();
            match self.connect_profile(&profile).await {
                Ok(()) => {}
                Err(e) if is_auth_error(&e) => {
                    self.command_panel
                        .push_error(format!("Authentication failed: {}", e));
                    self.credential_prompt.show(profile);
                }
                Err(e) => {
                    self.command_panel
                        .push_error(format!("Connection failed: {}", e));
                }
            }
        } else {
            self.command_panel.push_message(
                "No connections configured. Press Ctrl+T or add profiles to ~/.config/loom/config.toml".to_string(),
            );
        }
    }

    async fn connect_profile(&mut self, profile: &ConnectionProfile) -> anyhow::Result<()> {
        if profile.bind_dn.is_some() {
            match resolve_password(profile) {
                Ok(password) if !password.is_empty() => {
                    self.connect_with_password(profile, &password).await
                }
                _ => {
                    // No password available — need interactive prompt
                    self.credential_prompt.show(profile.clone());
                    Ok(())
                }
            }
        } else {
            self.connect_with_password(profile, "").await
        }
    }

    async fn connect_with_password(
        &mut self,
        profile: &ConnectionProfile,
        password: &str,
    ) -> anyhow::Result<()> {
        self.command_panel
            .push_message(format!("Connecting to {}...", profile.host));

        let settings = profile.to_connection_settings();
        let mut conn = LdapConnection::connect(settings).await?;

        // Bind with credential resolution
        if let Some(ref bind_dn) = profile.bind_dn {
            conn.simple_bind(bind_dn, password).await?;
        } else {
            conn.anonymous_bind().await?;
        }

        // Read RootDSE to detect server type and auto-discover base DN
        let server_type_str = match conn.read_root_dse().await {
            Ok(root_dse) => {
                let st = root_dse.server_type.to_string();
                self.command_panel
                    .push_message(format!("Server type: {}", st));
                st
            }
            Err(e) => {
                debug!("RootDSE read failed (non-fatal): {}", e);
                "LDAP".to_string()
            }
        };

        let conn_id = self.allocate_conn_id();
        let base_dn = conn.base_dn.clone();
        let label = profile.name.clone();
        let host = profile.host.clone();

        self.command_panel
            .push_message(format!("Connected to {} (base: {})", host, base_dn));
        self.status_bar.set_connected(&host, &server_type_str);

        let connection = Arc::new(Mutex::new(conn));
        let directory_tree = DirectoryTree::new(base_dn.clone());

        let tab = ConnectionTab {
            id: conn_id,
            label: label.clone(),
            host,
            server_type: server_type_str,
            connection,
            directory_tree,
            schema: None,
        };

        self.tabs.push(tab);
        self.tab_bar.add_tab(conn_id, label);
        self.active_tab_id = Some(conn_id);

        // Load root children
        self.spawn_load_children(conn_id, base_dn);

        Ok(())
    }

    fn spawn_load_children(&self, conn_id: ConnectionId, dn: String) {
        let tab = self.tabs.iter().find(|t| t.id == conn_id);
        if let Some(tab) = tab {
            let connection = tab.connection.clone();
            let tx = self.action_tx.clone();

            tokio::spawn(async move {
                let mut conn = connection.lock().await;
                let result = match conn.search_children(&dn).await {
                    Ok(entries) => Ok(entries),
                    Err(e) if LdapConnection::is_connection_error(&e) => {
                        let _ = tx.send(Action::StatusMessage("Reconnecting...".to_string()));
                        if conn.reconnect().await.is_ok() {
                            conn.search_children(&dn).await
                        } else {
                            Err(e)
                        }
                    }
                    Err(e) => Err(e),
                };

                match result {
                    Ok(entries) => {
                        let nodes: Vec<TreeNode> = entries
                            .iter()
                            .map(|e| TreeNode::new(e.dn.clone()))
                            .collect();
                        let _ = tx.send(Action::TreeChildrenLoaded(conn_id, dn, nodes));
                    }
                    Err(e) => {
                        let _ = tx.send(Action::ErrorMessage(format!(
                            "Failed to load {}: {}",
                            dn, e
                        )));
                    }
                }
            });
        }
    }

    fn spawn_load_entry(&self, conn_id: ConnectionId, dn: String) {
        let tab = self.tabs.iter().find(|t| t.id == conn_id);
        if let Some(tab) = tab {
            let connection = tab.connection.clone();
            let tx = self.action_tx.clone();

            tokio::spawn(async move {
                let mut conn = connection.lock().await;
                let result = match conn.search_entry(&dn).await {
                    Ok(entry) => Ok(entry),
                    Err(e) if LdapConnection::is_connection_error(&e) => {
                        let _ = tx.send(Action::StatusMessage("Reconnecting...".to_string()));
                        if conn.reconnect().await.is_ok() {
                            conn.search_entry(&dn).await
                        } else {
                            Err(e)
                        }
                    }
                    Err(e) => Err(e),
                };

                match result {
                    Ok(Some(entry)) => {
                        let _ = tx.send(Action::EntryLoaded(conn_id, entry));
                    }
                    Ok(None) => {
                        let _ = tx.send(Action::ErrorMessage(format!("Entry not found: {}", dn)));
                    }
                    Err(e) => {
                        let _ = tx.send(Action::ErrorMessage(format!(
                            "Failed to load {}: {}",
                            dn, e
                        )));
                    }
                }
            });
        }
    }

    fn spawn_search(&self, conn_id: ConnectionId, filter: String) {
        let tab = self.tabs.iter().find(|t| t.id == conn_id);
        if let Some(tab) = tab {
            let connection = tab.connection.clone();
            let base_dn = tab.directory_tree.root_dn.clone();
            let tx = self.action_tx.clone();

            tokio::spawn(async move {
                let mut conn = connection.lock().await;
                let result = match conn.search_subtree(&base_dn, &filter, vec!["*"]).await {
                    Ok(entries) => Ok(entries),
                    Err(e) if LdapConnection::is_connection_error(&e) => {
                        let _ = tx.send(Action::StatusMessage("Reconnecting...".to_string()));
                        if conn.reconnect().await.is_ok() {
                            conn.search_subtree(&base_dn, &filter, vec!["*"]).await
                        } else {
                            Err(e)
                        }
                    }
                    Err(e) => Err(e),
                };

                match result {
                    Ok(entries) => {
                        let _ = tx.send(Action::SearchResults(conn_id, entries));
                    }
                    Err(e) => {
                        let _ = tx.send(Action::ErrorMessage(format!("Search failed: {}", e)));
                    }
                }
            });
        }
    }

    fn spawn_save_attribute(
        &self,
        conn_id: ConnectionId,
        result: crate::components::attribute_editor::EditResult,
    ) {
        let tab = self.tabs.iter().find(|t| t.id == conn_id);
        if let Some(tab) = tab {
            let connection = tab.connection.clone();
            let tx = self.action_tx.clone();

            tokio::spawn(async move {
                debug!(
                    "spawn_save_attribute: dn={} op={:?} new_value={}",
                    result.dn, result.op, result.new_value
                );
                let mut conn = connection.lock().await;
                let modify_result = match &result.op {
                    EditOp::Replace { attr, old_value } => {
                        conn.replace_attribute_value(&result.dn, attr, old_value, &result.new_value)
                            .await
                    }
                    EditOp::Add { attr } => {
                        conn.add_attribute_value(&result.dn, attr, &result.new_value)
                            .await
                    }
                    EditOp::Delete { attr, value } => {
                        conn.delete_attribute_value(&result.dn, attr, value).await
                    }
                };

                match modify_result {
                    Ok(()) => {
                        let _ = tx.send(Action::AttributeSaved(result.dn));
                    }
                    Err(e) => {
                        let _ = tx.send(Action::ErrorMessage(format!("Failed to save: {}", e)));
                    }
                }
            });
        }
    }

    fn spawn_load_schema(&self, conn_id: ConnectionId) {
        let tab = self.tabs.iter().find(|t| t.id == conn_id);
        if let Some(tab) = tab {
            let connection = tab.connection.clone();
            let tx = self.action_tx.clone();

            tokio::spawn(async move {
                let mut conn = connection.lock().await;
                match conn.load_schema(None).await {
                    Ok(schema) => {
                        let _ = tx.send(Action::SchemaLoaded(conn_id, Box::new(schema)));
                    }
                    Err(e) => {
                        let _ = tx.send(Action::ErrorMessage(format!(
                            "Failed to load schema: {}",
                            e
                        )));
                    }
                }
            });
        }
    }

    fn spawn_export(
        &self,
        conn_id: ConnectionId,
        path: String,
        filter: String,
        attributes: Vec<String>,
    ) {
        let tab = self.tabs.iter().find(|t| t.id == conn_id);
        if let Some(tab) = tab {
            let connection = tab.connection.clone();
            let base_dn = tab.directory_tree.root_dn.clone();
            let tx = self.action_tx.clone();

            tokio::spawn(async move {
                let mut conn = connection.lock().await;
                let attr_refs: Vec<&str> = attributes.iter().map(|s| s.as_str()).collect();
                match conn.search_subtree(&base_dn, &filter, attr_refs).await {
                    Ok(entries) => {
                        let filepath = std::path::Path::new(&path);
                        match loom_core::export::export_entries(&entries, filepath) {
                            Ok(count) => {
                                let _ = tx.send(Action::ExportComplete(format!(
                                    "Exported {} entries to {}",
                                    count, path
                                )));
                            }
                            Err(e) => {
                                let _ =
                                    tx.send(Action::ErrorMessage(format!("Export failed: {}", e)));
                            }
                        }
                    }
                    Err(e) => {
                        let _ =
                            tx.send(Action::ErrorMessage(format!("Export search failed: {}", e)));
                    }
                }
            });
        }
    }

    fn spawn_bulk_update(
        &self,
        conn_id: ConnectionId,
        filter: String,
        modifications: Vec<BulkMod>,
    ) {
        let tab = self.tabs.iter().find(|t| t.id == conn_id);
        if let Some(tab) = tab {
            let connection = tab.connection.clone();
            let tx = self.action_tx.clone();

            tokio::spawn(async move {
                let mut conn = connection.lock().await;
                match conn.bulk_update(&filter, &modifications).await {
                    Ok(result) => {
                        let msg = format!(
                            "Bulk update: {} succeeded, {} failed out of {}",
                            result.succeeded, result.failed, result.total
                        );
                        let _ = tx.send(Action::BulkUpdateComplete(msg));
                    }
                    Err(e) => {
                        let _ = tx.send(Action::ErrorMessage(format!("Bulk update failed: {}", e)));
                    }
                }
            });
        }
    }

    fn spawn_create_entry(
        &self,
        conn_id: ConnectionId,
        dn: String,
        attributes: Vec<(String, Vec<String>)>,
    ) {
        let tab = self.tabs.iter().find(|t| t.id == conn_id);
        if let Some(tab) = tab {
            let connection = tab.connection.clone();
            let tx = self.action_tx.clone();

            tokio::spawn(async move {
                let mut conn = connection.lock().await;
                // Convert Vec<String> -> HashSet<String> for ldap3
                let attrs: Vec<(String, std::collections::HashSet<String>)> = attributes
                    .into_iter()
                    .map(|(k, v)| (k, v.into_iter().collect()))
                    .collect();

                match conn.add_entry(&dn, attrs).await {
                    Ok(()) => {
                        let _ = tx.send(Action::EntryCreated(dn));
                    }
                    Err(e) => {
                        let _ = tx.send(Action::ErrorMessage(format!(
                            "Failed to create entry: {}",
                            e
                        )));
                    }
                }
            });
        }
    }

    fn spawn_delete_entry(&self, conn_id: ConnectionId, dn: String) {
        let tab = self.tabs.iter().find(|t| t.id == conn_id);
        if let Some(tab) = tab {
            let connection = tab.connection.clone();
            let tx = self.action_tx.clone();

            tokio::spawn(async move {
                let mut conn = connection.lock().await;
                match conn.delete_entry(&dn).await {
                    Ok(()) => {
                        let _ = tx.send(Action::EntryDeleted(dn));
                    }
                    Err(e) => {
                        let _ = tx.send(Action::ErrorMessage(format!(
                            "Failed to delete entry: {}",
                            e
                        )));
                    }
                }
            });
        }
    }

    /// Check if any popup/dialog is currently visible.
    fn popup_active(&self) -> bool {
        self.confirm_dialog.visible
            || self.connect_dialog.visible
            || self.new_connection_dialog.visible
            || self.credential_prompt.visible
            || self.search_dialog.visible
            || self.attribute_editor.visible
            || self.export_dialog.visible
            || self.bulk_update_dialog.visible
            || self.create_entry_dialog.visible
            || self.schema_viewer.visible
            || self.log_panel.visible
    }

    /// Main event loop.
    pub async fn run(&mut self) -> anyhow::Result<()> {
        tui::install_panic_hook();
        let mut terminal = tui::init()?;

        let tick_rate = Duration::from_millis(self.config.general.tick_rate_ms);

        loop {
            // Render
            terminal.draw(|frame| self.render(frame))?;

            // Poll for events
            if let Some(app_event) = event::poll_event(tick_rate) {
                match app_event {
                    AppEvent::Key(key) => {
                        // Popups intercept keys first
                        let action = if self.attribute_editor.visible {
                            self.attribute_editor.handle_key_event(key)
                        } else if self.confirm_dialog.visible {
                            self.confirm_dialog.handle_key_event(key)
                        } else if self.connect_dialog.visible {
                            self.connect_dialog.handle_key_event(key)
                        } else if self.new_connection_dialog.visible {
                            self.new_connection_dialog.handle_key_event(key)
                        } else if self.credential_prompt.visible {
                            self.credential_prompt.handle_key_event(key)
                        } else if self.search_dialog.visible {
                            self.search_dialog.handle_key_event(key)
                        } else if self.export_dialog.visible {
                            self.export_dialog.handle_key_event(key)
                        } else if self.bulk_update_dialog.visible {
                            self.bulk_update_dialog.handle_key_event(key)
                        } else if self.create_entry_dialog.visible {
                            self.create_entry_dialog.handle_key_event(key)
                        } else if self.schema_viewer.visible {
                            self.schema_viewer.handle_key_event(key)
                        } else if self.log_panel.visible {
                            self.log_panel.handle_key_event(key)
                        } else if self.command_panel.input_active {
                            self.command_panel.handle_input_key(key)
                        } else {
                            // Try panel-specific handler first, fall back to global keymap
                            let panel_action = match self.focus.current() {
                                FocusTarget::TreePanel => self.tree_panel.handle_key_event(key),
                                FocusTarget::DetailPanel => self.detail_panel.handle_key_event(key),
                                FocusTarget::CommandPanel => {
                                    self.command_panel.handle_input_key(key)
                                }
                            };
                            if matches!(panel_action, Action::None) {
                                keymap::resolve_key(key, self.focus.current())
                            } else {
                                panel_action
                            }
                        };
                        let _ = self.action_tx.send(action);
                    }
                    AppEvent::Mouse(mouse) => {
                        let action = self.handle_mouse(mouse);
                        if !matches!(action, Action::None) {
                            let _ = self.action_tx.send(action);
                        }
                    }
                    AppEvent::Resize(w, h) => {
                        let _ = self.action_tx.send(Action::Resize(w, h));
                    }
                    AppEvent::Tick => {
                        let _ = self.action_tx.send(Action::Tick);
                    }
                }
            }

            // Drain action queue
            while let Ok(action) = self.action_rx.try_recv() {
                self.process_action(action).await;
            }

            if self.should_quit {
                break;
            }
        }

        tui::restore()?;
        Ok(())
    }

    fn handle_mouse(&self, mouse: crossterm::event::MouseEvent) -> Action {
        // Only handle click events, ignore popups
        if self.popup_active() {
            return Action::None;
        }

        match mouse.kind {
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                let pos = Rect::new(mouse.column, mouse.row, 1, 1);

                // Check which panel was clicked
                if let Some(tree) = self.tree_area {
                    if tree.intersects(pos) {
                        return Action::FocusPanel(FocusTarget::TreePanel);
                    }
                }
                if let Some(detail) = self.detail_area {
                    if detail.intersects(pos) {
                        return Action::FocusPanel(FocusTarget::DetailPanel);
                    }
                }
                if let Some(cmd) = self.command_area {
                    if cmd.intersects(pos) {
                        return Action::FocusPanel(FocusTarget::CommandPanel);
                    }
                }
                Action::None
            }
            _ => Action::None,
        }
    }

    async fn process_action(&mut self, action: Action) {
        match action {
            Action::Quit => {
                self.should_quit = true;
            }
            Action::FocusNext => {
                self.focus.next();
            }
            Action::FocusPrev => {
                self.focus.prev();
            }
            Action::FocusPanel(target) => {
                self.focus.set(target);
            }

            // Tab management
            Action::NextTab => {
                self.tab_bar.next_tab();
                if let Some(id) = self.tab_bar.active_tab {
                    self.switch_to_tab(id);
                }
            }
            Action::PrevTab => {
                self.tab_bar.prev_tab();
                if let Some(id) = self.tab_bar.active_tab {
                    self.switch_to_tab(id);
                }
            }
            Action::SwitchTab(id) => {
                self.switch_to_tab(id);
            }
            Action::ShowConnectDialog => {
                self.connect_dialog.show(self.config.connections.clone());
            }
            Action::ShowNewConnectionForm => {
                self.connect_dialog.hide();
                self.new_connection_dialog.show();
            }
            Action::ConnectByIndex(idx) => {
                let profile = self.config.connections.get(idx).cloned();
                if let Some(profile) = profile {
                    match self.connect_profile(&profile).await {
                        Ok(()) => {}
                        Err(e) if is_auth_error(&e) => {
                            self.command_panel
                                .push_error(format!("Authentication failed: {}", e));
                            self.credential_prompt.show(profile);
                        }
                        Err(e) => {
                            self.command_panel
                                .push_error(format!("Connection failed: {}", e));
                        }
                    }
                }
            }
            Action::ConnectAdHoc(profile, password) => {
                let profile_clone = profile.clone();
                match self.connect_with_password(&profile, &password).await {
                    Ok(()) => {
                        self.last_adhoc_profile = Some(profile_clone);
                        self.command_panel.push_message(
                            "Tip: Press Ctrl+W to save this connection to config".to_string(),
                        );
                    }
                    Err(e) if is_auth_error(&e) => {
                        self.command_panel
                            .push_error(format!("Authentication failed: {}", e));
                        self.credential_prompt.show(profile_clone);
                    }
                    Err(e) => {
                        self.command_panel
                            .push_error(format!("Connection failed: {}", e));
                    }
                }
            }
            Action::PromptCredentials(profile) => {
                self.credential_prompt.show(profile);
            }
            Action::ConnectWithCredentials(profile, password) => {
                let profile_clone = profile.clone();
                match self.connect_with_password(&profile, &password).await {
                    Ok(()) => {
                        self.command_panel.push_message(format!(
                            "Authenticated as {}",
                            profile.bind_dn.as_deref().unwrap_or("anonymous")
                        ));
                    }
                    Err(e) if is_auth_error(&e) => {
                        self.command_panel
                            .push_error(format!("Authentication failed: {}", e));
                        self.credential_prompt.show(profile_clone);
                    }
                    Err(e) => {
                        self.command_panel
                            .push_error(format!("Connection failed: {}", e));
                    }
                }
            }
            Action::SaveCurrentConnection => {
                if let Some(profile) = self.last_adhoc_profile.take() {
                    match AppConfig::append_connection(&profile) {
                        Ok(()) => {
                            self.command_panel.push_message(format!(
                                "Saved connection '{}' to config",
                                profile.name
                            ));
                            self.config.connections.push(profile);
                        }
                        Err(e) => {
                            self.command_panel
                                .push_error(format!("Failed to save connection: {}", e));
                        }
                    }
                } else {
                    self.command_panel
                        .push_message("No ad-hoc connection to save".to_string());
                }
            }
            Action::CloseTab(id) => {
                self.tabs.retain(|t| t.id != id);
                self.tab_bar.remove_tab(id);
                if self.active_tab_id == Some(id) {
                    self.active_tab_id = self.tab_bar.active_tab;
                    self.detail_panel.clear();
                    if self.active_tab_id.is_none() {
                        self.status_bar.set_disconnected();
                    }
                }
            }

            // Tree
            Action::TreeExpand(dn) => {
                if !dn.is_empty() {
                    if let Some(id) = self.active_tab_id {
                        self.spawn_load_children(id, dn.clone());
                        self.spawn_load_entry(id, dn);
                    }
                }
            }
            Action::TreeCollapse(_dn) => {}
            Action::TreeSelect(dn) => {
                if !dn.is_empty() {
                    if let Some(id) = self.active_tab_id {
                        self.spawn_load_entry(id, dn);
                    }
                }
            }
            Action::TreeChildrenLoaded(conn_id, parent_dn, nodes) => {
                if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == conn_id) {
                    tab.directory_tree.insert_children(&parent_dn, nodes);
                    self.command_panel.push_message(format!(
                        "Loaded children of {}",
                        loom_core::dn::rdn_display_name(&parent_dn)
                    ));
                }
            }
            Action::EntryLoaded(_conn_id, entry) => {
                self.detail_panel.set_entry(entry);
            }
            Action::EntryRefresh => {
                if let (Some(id), Some(ref entry)) = (self.active_tab_id, &self.detail_panel.entry)
                {
                    self.spawn_load_entry(id, entry.dn.clone());
                }
            }

            // Search
            Action::SearchExecute(filter) => {
                if let Some(id) = self.active_tab_id {
                    self.command_panel
                        .push_message(format!("Searching: {}...", filter));
                    self.spawn_search(id, filter);
                } else {
                    self.command_panel
                        .push_error("No active connection".to_string());
                }
            }
            Action::SearchResults(conn_id, entries) => {
                if self.active_tab_id == Some(conn_id) {
                    let count = entries.len();
                    self.command_panel
                        .push_message(format!("Found {} entries", count));
                    if entries.is_empty() {
                        self.command_panel
                            .push_message("No results found.".to_string());
                    } else {
                        self.search_dialog
                            .show_results("search".to_string(), entries);
                    }
                }
            }
            Action::SearchFocusInput => {
                self.focus.set(FocusTarget::CommandPanel);
                self.command_panel.activate_input();
            }

            // Attribute editing
            Action::EditAttribute(dn, attr, value) => {
                self.attribute_editor.edit_value(dn, attr, value);
            }
            Action::AddAttribute(dn, attr) => {
                self.attribute_editor.add_value(dn, attr);
            }
            Action::SaveAttribute(result) => {
                if let Some(id) = self.active_tab_id {
                    self.spawn_save_attribute(id, result);
                }
            }
            Action::AttributeSaved(dn) => {
                self.command_panel.push_message(format!(
                    "Saved changes to {}",
                    loom_core::dn::rdn_display_name(&dn)
                ));
                // Refresh the entry
                if let Some(id) = self.active_tab_id {
                    self.spawn_load_entry(id, dn);
                }
            }

            // Export
            Action::ShowExportDialog => {
                if let Some(tab) = self.active_tab() {
                    // Count entries (approximate via tree)
                    let count = tab
                        .directory_tree
                        .root
                        .children
                        .as_ref()
                        .map(|c| c.len())
                        .unwrap_or(0);
                    self.export_dialog.show(count);
                } else {
                    self.command_panel
                        .push_error("No active connection".to_string());
                }
            }
            Action::ExportExecute {
                path,
                filter,
                attributes,
            } => {
                if let Some(id) = self.active_tab_id {
                    self.command_panel
                        .push_message(format!("Exporting to {} (filter: {})...", path, filter));
                    self.spawn_export(id, path, filter, attributes);
                }
            }
            Action::ExportComplete(msg) => {
                self.command_panel.push_message(msg);
            }

            // Bulk Update
            Action::ShowBulkUpdateDialog => {
                if self.active_tab_id.is_some() {
                    self.bulk_update_dialog.show();
                } else {
                    self.command_panel
                        .push_error("No active connection".to_string());
                }
            }
            Action::BulkUpdateExecute {
                filter,
                attribute,
                value,
                op,
            } => {
                if let Some(id) = self.active_tab_id {
                    use crate::components::bulk_update_dialog::BulkOp;
                    let modification = match op {
                        BulkOp::Replace => BulkMod::ReplaceAttribute {
                            attr: attribute,
                            value,
                        },
                        BulkOp::Add => BulkMod::AddValue {
                            attr: attribute,
                            value,
                        },
                        BulkOp::Delete => {
                            if value.is_empty() {
                                BulkMod::DeleteAttribute { attr: attribute }
                            } else {
                                BulkMod::DeleteValue {
                                    attr: attribute,
                                    value,
                                }
                            }
                        }
                    };
                    self.command_panel
                        .push_message(format!("Executing bulk update: {}...", filter));
                    self.spawn_bulk_update(id, filter, vec![modification]);
                }
            }
            Action::BulkUpdateComplete(msg) => {
                self.command_panel.push_message(msg);
            }

            // Create / Delete Entry
            Action::ShowCreateEntryDialog(parent_dn) => {
                if self.active_tab_id.is_some() {
                    self.create_entry_dialog.show(parent_dn);
                } else {
                    self.command_panel
                        .push_error("No active connection".to_string());
                }
            }
            Action::CreateEntry { dn, attributes } => {
                if let Some(id) = self.active_tab_id {
                    self.command_panel
                        .push_message(format!("Creating entry: {}...", dn));
                    self.spawn_create_entry(id, dn, attributes);
                }
            }
            Action::EntryCreated(dn) => {
                self.command_panel.push_message(format!(
                    "Created entry: {}",
                    loom_core::dn::rdn_display_name(&dn)
                ));
                // Refresh parent's children in the tree
                if let Some(id) = self.active_tab_id {
                    if let Some(parent) = loom_core::dn::parent_dn(&dn) {
                        self.spawn_load_children(id, parent.to_string());
                    }
                }
            }
            Action::DeleteEntry(dn) => {
                if let Some(id) = self.active_tab_id {
                    self.command_panel
                        .push_message(format!("Deleting entry: {}...", dn));
                    self.spawn_delete_entry(id, dn);
                }
            }
            Action::EntryDeleted(dn) => {
                self.command_panel.push_message(format!(
                    "Deleted entry: {}",
                    loom_core::dn::rdn_display_name(&dn)
                ));
                // Clear detail panel if showing the deleted entry
                if let Some(ref entry) = self.detail_panel.entry {
                    if entry.dn == dn {
                        self.detail_panel.clear();
                    }
                }
                // Refresh parent's children in the tree
                if let Some(id) = self.active_tab_id {
                    if let Some(parent) = loom_core::dn::parent_dn(&dn) {
                        self.spawn_load_children(id, parent.to_string());
                    }
                }
            }

            // Schema
            Action::ShowSchemaViewer => {
                let schema_and_id = self.active_tab().map(|tab| {
                    if let Some(ref schema) = tab.schema {
                        (schema.clone(), tab.id)
                    } else {
                        (SchemaCache::new(), tab.id)
                    }
                });
                match schema_and_id {
                    Some((schema, _id)) if !schema.attribute_types.is_empty() => {
                        self.schema_viewer.show(&schema);
                    }
                    Some((_, id)) => {
                        self.command_panel
                            .push_message("Loading schema...".to_string());
                        self.spawn_load_schema(id);
                    }
                    None => {
                        self.command_panel
                            .push_error("No active connection".to_string());
                    }
                }
            }
            Action::SchemaLoaded(conn_id, schema) => {
                if let Some(tab) = self.tabs.iter_mut().find(|t| t.id == conn_id) {
                    self.command_panel.push_message(format!(
                        "Schema loaded: {} attribute types, {} object classes",
                        schema.attribute_types.len(),
                        schema.object_classes.len()
                    ));
                    tab.schema = Some(*schema.clone());
                    self.schema_viewer.show(&schema);
                }
            }

            // Log Panel
            Action::ToggleLogPanel => {
                self.log_panel.toggle();
            }

            // Popups
            Action::ShowConfirm(msg, on_confirm) => {
                self.confirm_dialog.show(msg, *on_confirm);
            }
            Action::ClosePopup => {
                self.confirm_dialog.hide();
                self.connect_dialog.hide();
                self.new_connection_dialog.hide();
                self.credential_prompt.hide();
                self.search_dialog.hide();
                self.attribute_editor.hide();
                self.export_dialog.hide();
                self.bulk_update_dialog.hide();
                self.create_entry_dialog.hide();
                self.schema_viewer.hide();
                self.log_panel.hide();
            }

            // Status
            Action::StatusMessage(msg) => {
                self.log_panel.push_info(msg.clone());
                self.command_panel.push_message(msg);
            }
            Action::ErrorMessage(msg) => {
                error!("{}", msg);
                self.log_panel.push_error(msg.clone());
                self.command_panel.push_error(msg);
            }

            Action::Tick | Action::Render | Action::Resize(_, _) | Action::None => {}
            _ => {}
        }
    }

    fn switch_to_tab(&mut self, id: ConnectionId) {
        self.active_tab_id = Some(id);
        self.tab_bar.set_active(id);
        self.detail_panel.clear();
        self.tree_panel.tree_state = tui_tree_widget::TreeState::default();

        if let Some(tab) = self.tabs.iter().find(|t| t.id == id) {
            self.status_bar.set_connected(&tab.host, &tab.server_type);
        }
    }

    fn render(&mut self, frame: &mut ratatui::Frame) {
        let full = frame.area();

        // Vertical: tab bar (1) | content area | status bar (1)
        let outer = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
        ])
        .split(full);

        let tab_area = outer[0];
        let content_area = outer[1];
        let status_area = outer[2];

        self.tab_area = Some(tab_area);

        // Render tab bar
        self.tab_bar.render(frame, tab_area);

        // Horizontal: tree (25%) | right panels (75%)
        let horizontal =
            Layout::horizontal([Constraint::Percentage(25), Constraint::Percentage(75)])
                .split(content_area);

        let tree_area = horizontal[0];
        let right_area = horizontal[1];

        // Right side: detail (75%) | command (25%)
        let right_vertical =
            Layout::vertical([Constraint::Percentage(75), Constraint::Percentage(25)])
                .split(right_area);

        let detail_area = right_vertical[0];
        let command_area = right_vertical[1];

        // Store areas for mouse hit-testing
        self.tree_area = Some(tree_area);
        self.detail_area = Some(detail_area);
        self.command_area = Some(command_area);

        // Render tree panel
        let tree_focused = self.focus.is_focused(FocusTarget::TreePanel);
        if let Some(tab) = self.active_tab() {
            let items = TreePanel::build_tree_items(&tab.directory_tree.root);
            let label = tab.label.clone();
            self.tree_panel
                .render_with_items(frame, tree_area, tree_focused, &items, &label);
        } else {
            self.tree_panel.render_empty(frame, tree_area, tree_focused);
        }

        // Render detail and command panels
        self.detail_panel.render(
            frame,
            detail_area,
            self.focus.is_focused(FocusTarget::DetailPanel),
        );
        self.command_panel.render(
            frame,
            command_area,
            self.focus.is_focused(FocusTarget::CommandPanel),
        );

        // Status bar
        self.status_bar.render(frame, status_area, false);

        // Render popups on top (order matters: last rendered is on top)
        if self.confirm_dialog.visible {
            self.confirm_dialog.render(frame, full);
        }
        if self.connect_dialog.visible {
            self.connect_dialog.render(frame, full);
        }
        if self.new_connection_dialog.visible {
            self.new_connection_dialog.render(frame, full);
        }
        if self.credential_prompt.visible {
            self.credential_prompt.render(frame, full);
        }
        if self.search_dialog.visible {
            self.search_dialog.render(frame, full);
        }
        if self.attribute_editor.visible {
            self.attribute_editor.render(frame, full);
        }
        if self.export_dialog.visible {
            self.export_dialog.render(frame, full);
        }
        if self.bulk_update_dialog.visible {
            self.bulk_update_dialog.render(frame, full);
        }
        if self.create_entry_dialog.visible {
            self.create_entry_dialog.render(frame, full);
        }
        if self.schema_viewer.visible {
            self.schema_viewer.render(frame, full);
        }
        if self.log_panel.visible {
            self.log_panel.render(frame, full);
        }
    }
}

/// Resolve password from the connection profile's credential method.
/// Returns empty string for Prompt method when LOOM_PASSWORD is not set,
/// which signals the caller to show an interactive credential prompt.
fn resolve_password(profile: &ConnectionProfile) -> anyhow::Result<String> {
    match profile.credential_method {
        CredentialMethod::Prompt => Ok(std::env::var("LOOM_PASSWORD").unwrap_or_default()),
        CredentialMethod::Command => {
            let cmd = profile.password_command.as_deref().ok_or_else(|| {
                anyhow::anyhow!("credential_method is 'command' but no password_command configured")
            })?;
            Ok(CredentialProvider::from_command(cmd)?)
        }
        CredentialMethod::Keychain => Ok(CredentialProvider::from_keychain(&profile.name)?),
    }
}

/// Check if an error is an LDAP authentication/bind failure (rc=49 etc.).
fn is_auth_error(err: &anyhow::Error) -> bool {
    let msg = err.to_string().to_lowercase();
    msg.contains("bind failed")
        || msg.contains("rc=49")
        || msg.contains("invalid credentials")
        || msg.contains("password must be provided")
}
