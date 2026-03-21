slint::include_modules!();

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use async_compat::Compat;
use slint::{Model, ModelRc, SharedString, VecModel};
use tracing::{error, info};

use loom_core::config::{AppConfig, ConnectionProfile};
use loom_core::connection::{ConnectionSettings, LdapConnection, TlsMode};
use loom_core::credentials::{CredentialMethod, CredentialProvider};
use loom_core::vault::Vault;

/// Per-node metadata stored alongside the flat tree model.
#[derive(Clone, Debug)]
#[allow(dead_code)]
struct TreeNodeMeta {
    dn: String,
    indent_level: i32,
    has_children: bool,
}

/// State for a single connection.
struct ConnectionState {
    conn: LdapConnection,
    tree_meta: Vec<TreeNodeMeta>,
    tree_nodes: Vec<TreeNode>,
    attributes: Vec<AttributeRow>,
    entry_dn: SharedString,
    selected_index: i32,
}

/// Build the sidebar model from config profiles, connection state, and active profile.
fn build_sidebar_model(
    config: &AppConfig,
    connections: &HashMap<usize, ConnectionState>,
    active: Option<usize>,
    expanded_folders: &HashSet<String>,
    filter: &str,
) -> Vec<SidebarProfile> {
    let filter_lower = filter.to_lowercase();
    let has_filter = !filter_lower.is_empty();

    // Collect profiles with their config indices
    let mut foldered: HashMap<String, Vec<(usize, &ConnectionProfile)>> = HashMap::new();
    let mut ungrouped: Vec<(usize, &ConnectionProfile)> = Vec::new();

    for (i, profile) in config.connections.iter().enumerate() {
        // Apply filter
        if has_filter {
            let name_match = profile.name.to_lowercase().contains(&filter_lower);
            let host_match = profile.host.to_lowercase().contains(&filter_lower);
            let folder_match = profile
                .folder
                .as_ref()
                .is_some_and(|f| f.to_lowercase().contains(&filter_lower));
            let label_match = profile
                .labels
                .iter()
                .any(|l| l.to_lowercase().contains(&filter_lower));
            if !name_match && !host_match && !folder_match && !label_match {
                continue;
            }
        }

        match &profile.folder {
            Some(folder) if !folder.is_empty() => {
                foldered
                    .entry(folder.clone())
                    .or_default()
                    .push((i, profile));
            }
            _ => {
                ungrouped.push((i, profile));
            }
        }
    }

    let mut result = Vec::new();

    // Sort folders alphabetically
    let mut folder_names: Vec<String> = foldered.keys().cloned().collect();
    folder_names.sort();

    for folder_name in &folder_names {
        let profiles = &foldered[folder_name];

        // Handle nested folders (e.g., "A/B")
        let parts: Vec<&str> = folder_name.split('/').collect();
        for (depth, part) in parts.iter().enumerate() {
            let folder_path = parts[..=depth].join("/");
            let is_expanded = has_filter || expanded_folders.contains(&folder_path);

            // Only emit the folder row if we haven't already (for shared prefixes)
            let already_emitted = result.iter().any(|p: &SidebarProfile| {
                p.is_folder && p.name.as_str() == *part && p.indent_level == depth as i32
            });
            if !already_emitted {
                result.push(SidebarProfile {
                    name: SharedString::from(*part),
                    host: SharedString::default(),
                    index: -1,
                    labels: SharedString::default(),
                    is_connected: false,
                    is_active: false,
                    indent_level: depth as i32,
                    is_folder: true,
                    is_expanded,
                });
            }

            // If this intermediate folder is not expanded and no filter, skip deeper
            if !is_expanded && depth < parts.len() - 1 {
                break;
            }
        }

        // Check if the full folder path is expanded
        let full_expanded = has_filter || expanded_folders.contains(folder_name);
        if full_expanded {
            let indent = parts.len() as i32;
            for (idx, profile) in profiles {
                result.push(make_sidebar_profile(
                    *idx,
                    profile,
                    connections,
                    active,
                    indent,
                ));
            }
        }
    }

    // Ungrouped profiles at indent 0
    for (idx, profile) in &ungrouped {
        result.push(make_sidebar_profile(*idx, profile, connections, active, 0));
    }

    result
}

fn make_sidebar_profile(
    idx: usize,
    profile: &ConnectionProfile,
    connections: &HashMap<usize, ConnectionState>,
    active: Option<usize>,
    indent: i32,
) -> SidebarProfile {
    SidebarProfile {
        name: SharedString::from(&profile.name),
        host: SharedString::from(&profile.host),
        index: idx as i32,
        labels: SharedString::from(profile.labels.join(", ")),
        is_connected: connections.contains_key(&idx),
        is_active: active == Some(idx),
        indent_level: indent,
        is_folder: false,
        is_expanded: false,
    }
}

/// Helper to save current active connection's UI state back into the connections map.
fn save_active_state(
    connections: &mut HashMap<usize, ConnectionState>,
    active: Option<usize>,
    tree_model: &VecModel<TreeNode>,
    attr_model: &VecModel<AttributeRow>,
    entry_dn: &SharedString,
    selected_index: i32,
) {
    if let Some(active_idx) = active {
        if let Some(state) = connections.get_mut(&active_idx) {
            // Save tree nodes
            state.tree_nodes = (0..tree_model.row_count())
                .map(|i| tree_model.row_data(i).unwrap())
                .collect();
            // Save attributes
            state.attributes = (0..attr_model.row_count())
                .map(|i| attr_model.row_data(i).unwrap())
                .collect();
            state.entry_dn = entry_dn.clone();
            state.selected_index = selected_index;
        }
    }
}

/// Helper to load a connection's saved state into the UI models.
fn load_connection_state(
    state: &ConnectionState,
    tree_model: &VecModel<TreeNode>,
    attr_model: &VecModel<AttributeRow>,
    win: &MainWindow,
) {
    tree_model.set_vec(state.tree_nodes.clone());
    attr_model.set_vec(state.attributes.clone());
    win.set_entry_dn(state.entry_dn.clone());
    win.set_tree_selected_index(state.selected_index);
}

fn refresh_sidebar(
    win: &MainWindow,
    sidebar_model: &VecModel<SidebarProfile>,
    config: &AppConfig,
    connections: &HashMap<usize, ConnectionState>,
    active: Option<usize>,
    expanded_folders: &HashSet<String>,
    filter: &str,
) {
    let items = build_sidebar_model(config, connections, active, expanded_folders, filter);
    sidebar_model.set_vec(items);
    win.set_has_active_connection(active.is_some());
}

pub fn run() -> Result<(), slint::PlatformError> {
    let config = AppConfig::load();
    let main_window = MainWindow::new()?;

    apply_theme(&main_window, &config.general.theme);
    main_window.set_current_theme(SharedString::from(&config.general.theme));

    main_window.set_status_message("Ready".into());

    // Shared state
    let config = Rc::new(RefCell::new(config));
    let connections: Rc<RefCell<HashMap<usize, ConnectionState>>> =
        Rc::new(RefCell::new(HashMap::new()));
    let active_profile: Rc<RefCell<Option<usize>>> = Rc::new(RefCell::new(None));
    let expanded_folders: Rc<RefCell<HashSet<String>>> = Rc::new(RefCell::new(HashSet::new()));
    let sidebar_filter: Rc<RefCell<String>> = Rc::new(RefCell::new(String::new()));
    let vault_state: Rc<RefCell<Option<Vault>>> = Rc::new(RefCell::new(None));
    let pending_profile_index: Rc<RefCell<Option<i32>>> = Rc::new(RefCell::new(None));

    // Shared VecModels
    let tree_model: Rc<VecModel<TreeNode>> = Rc::new(VecModel::default());
    main_window.set_tree_model(ModelRc::from(tree_model.clone()));

    let attr_model: Rc<VecModel<AttributeRow>> = Rc::new(VecModel::default());
    main_window.set_attributes(ModelRc::from(attr_model.clone()));

    let sidebar_model: Rc<VecModel<SidebarProfile>> = Rc::new(VecModel::default());
    main_window.set_sidebar_model(ModelRc::from(sidebar_model.clone()));

    // Build initial sidebar
    {
        let cfg = config.borrow();
        let conns = connections.borrow();
        let expanded = expanded_folders.borrow();
        let items = build_sidebar_model(&cfg, &conns, None, &expanded, "");
        sidebar_model.set_vec(items);
    }

    // --- Sidebar: profile clicked ---
    {
        let weak = main_window.as_weak();
        let config = config.clone();
        let connections = connections.clone();
        let active_profile = active_profile.clone();
        let expanded_folders = expanded_folders.clone();
        let sidebar_filter = sidebar_filter.clone();
        let tree_model = tree_model.clone();
        let attr_model = attr_model.clone();
        let sidebar_model = sidebar_model.clone();
        let vault_state_c = vault_state.clone();
        let pending_profile_index_c = pending_profile_index.clone();

        main_window.on_sidebar_profile_clicked(move |sidebar_index| {
            let sidebar_idx = sidebar_index as usize;

            // Look up the SidebarProfile at this index
            let sidebar_entry = match sidebar_model.row_data(sidebar_idx) {
                Some(entry) => entry,
                None => return,
            };

            // Skip if it's a folder
            if sidebar_entry.is_folder {
                return;
            }

            let profile_config_index = sidebar_entry.index as usize;

            // Check if already connected
            {
                let mut conns = connections.borrow_mut();
                let current_active = *active_profile.borrow();

                if conns.contains_key(&profile_config_index) {
                    // Save current active state
                    if let Some(win) = weak.upgrade() {
                        save_active_state(
                            &mut conns,
                            current_active,
                            &tree_model,
                            &attr_model,
                            &win.get_entry_dn(),
                            win.get_tree_selected_index(),
                        );

                        // Load target connection state
                        if let Some(state) = conns.get(&profile_config_index) {
                            load_connection_state(state, &tree_model, &attr_model, &win);
                        }

                        *active_profile.borrow_mut() = Some(profile_config_index);

                        let cfg = config.borrow();
                        let expanded = expanded_folders.borrow();
                        let filter = sidebar_filter.borrow();
                        refresh_sidebar(
                            &win,
                            &sidebar_model,
                            &cfg,
                            &conns,
                            Some(profile_config_index),
                            &expanded,
                            &filter,
                        );
                    }
                    return;
                }
            }

            // Not connected yet — initiate connection
            let cfg = config.borrow();
            let profile = match cfg.connections.get(profile_config_index) {
                Some(p) => p.clone(),
                None => {
                    if let Some(win) = weak.upgrade() {
                        win.set_status_message(SharedString::from(format!(
                            "Profile index {} not found",
                            profile_config_index
                        )));
                        win.set_status_is_error(true);
                    }
                    return;
                }
            };

            let host = profile.host.clone();
            let profile_name = profile.name.clone();
            let settings = profile.to_connection_settings();
            let base_dn = profile.base_dn.clone().unwrap_or_default();
            let bind_dn = profile.bind_dn.clone();
            let credential_method = profile.credential_method.clone();
            let password_command = profile.password_command.clone();

            // Resolve password
            let password: Option<String> = if bind_dn.is_some() {
                match credential_method {
                    CredentialMethod::Command => {
                        if let Some(ref cmd) = password_command {
                            match CredentialProvider::from_command(cmd) {
                                Ok(pw) => Some(pw),
                                Err(e) => {
                                    error!("Password command failed: {}", e);
                                    None
                                }
                            }
                        } else {
                            None
                        }
                    }
                    CredentialMethod::Keychain => {
                        match CredentialProvider::from_keychain(&profile_name) {
                            Ok(pw) => Some(pw),
                            Err(e) => {
                                error!("Keychain lookup failed: {}", e);
                                None
                            }
                        }
                    }
                    CredentialMethod::Vault => {
                        let vault = vault_state_c.borrow();
                        match vault.as_ref() {
                            Some(v) => v.get_password(&profile_name).map(|s| s.to_string()),
                            None => None,
                        }
                    }
                    CredentialMethod::Prompt => None,
                }
            } else {
                None
            };

            // If bind_dn is set but no password, show credential dialog
            if bind_dn.is_some() && password.is_none() {
                *pending_profile_index_c.borrow_mut() = Some(profile_config_index as i32);
                if let Some(win) = weak.upgrade() {
                    win.set_credential_profile_name(SharedString::from(&profile_name));
                    win.set_credential_bind_dn(SharedString::from(
                        bind_dn.as_deref().unwrap_or(""),
                    ));
                    win.set_credential_dialog_visible(true);
                }
                return;
            }

            if let Some(win) = weak.upgrade() {
                win.set_status_message(SharedString::from(format!("Connecting to {}...", &host)));
                win.set_status_is_error(false);
            }

            let weak = weak.clone();
            let connections = connections.clone();
            let active_profile = active_profile.clone();
            let expanded_folders = expanded_folders.clone();
            let sidebar_filter = sidebar_filter.clone();
            let config = config.clone();
            let tree_model = tree_model.clone();
            let attr_model = attr_model.clone();
            let sidebar_model = sidebar_model.clone();

            spawn_connect(
                weak,
                connections,
                active_profile,
                config,
                expanded_folders,
                sidebar_filter,
                tree_model,
                attr_model,
                sidebar_model,
                settings,
                host,
                profile_name,
                base_dn,
                bind_dn,
                password,
                profile_config_index,
            );
        });
    }

    // --- Sidebar: folder toggled ---
    {
        let weak = main_window.as_weak();
        let config = config.clone();
        let connections = connections.clone();
        let active_profile = active_profile.clone();
        let expanded_folders = expanded_folders.clone();
        let sidebar_filter = sidebar_filter.clone();
        let sidebar_model = sidebar_model.clone();

        main_window.on_sidebar_folder_toggled(move |sidebar_index| {
            let sidebar_idx = sidebar_index as usize;
            let entry = match sidebar_model.row_data(sidebar_idx) {
                Some(e) => e,
                None => return,
            };

            if !entry.is_folder {
                return;
            }

            // Reconstruct the folder path from the sidebar model
            // Walk backwards from this index to build the full path
            let mut path_parts: Vec<String> = vec![entry.name.to_string()];
            let target_indent = entry.indent_level;

            if target_indent > 0 {
                let mut look_indent = target_indent - 1;
                let mut i = sidebar_idx;
                while i > 0 && look_indent >= 0 {
                    i -= 1;
                    if let Some(prev) = sidebar_model.row_data(i) {
                        if prev.is_folder && prev.indent_level == look_indent {
                            path_parts.push(prev.name.to_string());
                            if look_indent == 0 {
                                break;
                            }
                            look_indent -= 1;
                        }
                    }
                }
                path_parts.reverse();
            }

            let folder_path = path_parts.join("/");

            {
                let mut expanded = expanded_folders.borrow_mut();
                if expanded.contains(&folder_path) {
                    expanded.remove(&folder_path);
                    // Also remove any sub-folder expansions
                    let prefix = format!("{}/", folder_path);
                    expanded.retain(|p| !p.starts_with(&prefix));
                } else {
                    expanded.insert(folder_path);
                }
            }

            if let Some(win) = weak.upgrade() {
                let cfg = config.borrow();
                let conns = connections.borrow();
                let active = *active_profile.borrow();
                let expanded = expanded_folders.borrow();
                let filter = sidebar_filter.borrow();
                refresh_sidebar(
                    &win,
                    &sidebar_model,
                    &cfg,
                    &conns,
                    active,
                    &expanded,
                    &filter,
                );
            }
        });
    }

    // --- Sidebar: filter changed ---
    {
        let weak = main_window.as_weak();
        let config = config.clone();
        let connections = connections.clone();
        let active_profile = active_profile.clone();
        let expanded_folders = expanded_folders.clone();
        let sidebar_filter = sidebar_filter.clone();
        let sidebar_model = sidebar_model.clone();

        main_window.on_sidebar_filter_changed(move |text| {
            *sidebar_filter.borrow_mut() = text.to_string();
            if let Some(win) = weak.upgrade() {
                let cfg = config.borrow();
                let conns = connections.borrow();
                let active = *active_profile.borrow();
                let expanded = expanded_folders.borrow();
                refresh_sidebar(&win, &sidebar_model, &cfg, &conns, active, &expanded, &text);
            }
        });
    }

    // --- Sidebar: new profile ---
    {
        let weak = main_window.as_weak();
        main_window.on_sidebar_new_profile(move || {
            if let Some(win) = weak.upgrade() {
                win.set_profile_dialog_title("New Profile".into());
                win.set_profile_dialog_name(SharedString::default());
                win.set_profile_dialog_host(SharedString::default());
                win.set_profile_dialog_port("389".into());
                win.set_profile_dialog_tls_mode("auto".into());
                win.set_profile_dialog_bind_dn(SharedString::default());
                win.set_profile_dialog_base_dn(SharedString::default());
                win.set_profile_dialog_credential_method("prompt".into());
                win.set_profile_dialog_folder(SharedString::default());
                win.set_profile_dialog_labels(SharedString::default());
                win.set_profile_dialog_visible(true);
            }
        });
    }

    // --- connect-profile callback (kept for credential dialog flow) ---
    {
        let weak = main_window.as_weak();
        let config = config.clone();
        let connections = connections.clone();
        let active_profile = active_profile.clone();
        let expanded_folders = expanded_folders.clone();
        let sidebar_filter = sidebar_filter.clone();
        let tree_model = tree_model.clone();
        let attr_model = attr_model.clone();
        let sidebar_model = sidebar_model.clone();
        let vault_state_c = vault_state.clone();
        let pending_profile_index_c = pending_profile_index.clone();

        main_window.on_connect_profile(move |profile_index| {
            let index = profile_index as usize;
            let cfg = config.borrow();
            let profile = match cfg.connections.get(index) {
                Some(p) => p.clone(),
                None => {
                    if let Some(win) = weak.upgrade() {
                        win.set_status_message(SharedString::from(format!(
                            "Profile index {} not found",
                            index
                        )));
                        win.set_status_is_error(true);
                    }
                    return;
                }
            };

            let host = profile.host.clone();
            let profile_name = profile.name.clone();
            let settings = profile.to_connection_settings();
            let base_dn = profile.base_dn.clone().unwrap_or_default();
            let bind_dn = profile.bind_dn.clone();
            let credential_method = profile.credential_method.clone();
            let password_command = profile.password_command.clone();

            let password: Option<String> = if bind_dn.is_some() {
                match credential_method {
                    CredentialMethod::Command => {
                        if let Some(ref cmd) = password_command {
                            match CredentialProvider::from_command(cmd) {
                                Ok(pw) => Some(pw),
                                Err(e) => {
                                    error!("Password command failed: {}", e);
                                    None
                                }
                            }
                        } else {
                            None
                        }
                    }
                    CredentialMethod::Keychain => {
                        match CredentialProvider::from_keychain(&profile_name) {
                            Ok(pw) => Some(pw),
                            Err(e) => {
                                error!("Keychain lookup failed: {}", e);
                                None
                            }
                        }
                    }
                    CredentialMethod::Vault => {
                        let vault = vault_state_c.borrow();
                        match vault.as_ref() {
                            Some(v) => v.get_password(&profile_name).map(|s| s.to_string()),
                            None => None,
                        }
                    }
                    CredentialMethod::Prompt => None,
                }
            } else {
                None
            };

            if bind_dn.is_some() && password.is_none() {
                *pending_profile_index_c.borrow_mut() = Some(profile_index);
                if let Some(win) = weak.upgrade() {
                    win.set_credential_profile_name(SharedString::from(&profile_name));
                    win.set_credential_bind_dn(SharedString::from(
                        bind_dn.as_deref().unwrap_or(""),
                    ));
                    win.set_credential_dialog_visible(true);
                }
                return;
            }

            if let Some(win) = weak.upgrade() {
                win.set_status_message(SharedString::from(format!("Connecting to {}...", &host)));
                win.set_status_is_error(false);
            }

            let weak = weak.clone();
            let connections = connections.clone();
            let active_profile = active_profile.clone();
            let config = config.clone();
            let expanded_folders = expanded_folders.clone();
            let sidebar_filter = sidebar_filter.clone();
            let tree_model = tree_model.clone();
            let attr_model = attr_model.clone();
            let sidebar_model = sidebar_model.clone();

            spawn_connect(
                weak,
                connections,
                active_profile,
                config,
                expanded_folders,
                sidebar_filter,
                tree_model,
                attr_model,
                sidebar_model,
                settings,
                host,
                profile_name,
                base_dn,
                bind_dn,
                password,
                index,
            );
        });
    }

    // --- Credential-connect callback ---
    {
        let weak = main_window.as_weak();
        let config = config.clone();
        let connections = connections.clone();
        let active_profile = active_profile.clone();
        let expanded_folders = expanded_folders.clone();
        let sidebar_filter = sidebar_filter.clone();
        let tree_model = tree_model.clone();
        let attr_model = attr_model.clone();
        let sidebar_model = sidebar_model.clone();
        let pending_profile_index_c = pending_profile_index.clone();

        main_window.on_credential_connect(move |password| {
            let profile_index = match *pending_profile_index_c.borrow() {
                Some(idx) => idx,
                None => return,
            };
            *pending_profile_index_c.borrow_mut() = None;

            let index = profile_index as usize;
            let cfg = config.borrow();
            let profile = match cfg.connections.get(index) {
                Some(p) => p.clone(),
                None => return,
            };

            let host = profile.host.clone();
            let profile_name = profile.name.clone();
            let settings = profile.to_connection_settings();
            let base_dn = profile.base_dn.clone().unwrap_or_default();
            let bind_dn = profile.bind_dn.clone();

            if let Some(win) = weak.upgrade() {
                win.set_credential_dialog_visible(false);
                win.set_status_message(SharedString::from(format!("Connecting to {}...", &host)));
                win.set_status_is_error(false);
            }

            let weak = weak.clone();
            let connections = connections.clone();
            let active_profile = active_profile.clone();
            let config = config.clone();
            let expanded_folders = expanded_folders.clone();
            let sidebar_filter = sidebar_filter.clone();
            let tree_model = tree_model.clone();
            let attr_model = attr_model.clone();
            let sidebar_model = sidebar_model.clone();

            spawn_connect(
                weak,
                connections,
                active_profile,
                config,
                expanded_folders,
                sidebar_filter,
                tree_model,
                attr_model,
                sidebar_model,
                settings,
                host,
                profile_name,
                base_dn,
                bind_dn,
                Some(password.to_string()),
                index,
            );
        });
    }

    // --- Credential-cancel callback ---
    {
        let weak = main_window.as_weak();
        let pending_profile_index_c = pending_profile_index.clone();

        main_window.on_credential_cancel(move || {
            *pending_profile_index_c.borrow_mut() = None;
            if let Some(win) = weak.upgrade() {
                win.set_credential_dialog_visible(false);
                win.set_status_message("Connection cancelled".into());
                win.set_status_is_error(false);
            }
        });
    }

    // --- Tree toggle expand ---
    {
        let weak = main_window.as_weak();
        let connections = connections.clone();
        let active_profile = active_profile.clone();
        let tree_model = tree_model.clone();

        main_window.on_tree_toggle_expand(move |index| {
            let idx = index as usize;
            if idx >= tree_model.row_count() {
                return;
            }

            let node = tree_model.row_data(idx).unwrap();
            let active = *active_profile.borrow();

            if node.expanded {
                // Collapse: remove all descendants
                let my_indent = node.indent_level;
                let mut collapsed_node = node.clone();
                collapsed_node.expanded = false;
                tree_model.set_row_data(idx, collapsed_node);

                let mut remove_count = 0;
                let start = idx + 1;
                while start < tree_model.row_count() {
                    let child = tree_model.row_data(start).unwrap();
                    if child.indent_level > my_indent {
                        tree_model.remove(start);
                        remove_count += 1;
                    } else {
                        break;
                    }
                }

                if let Some(active_idx) = active {
                    let mut conns = connections.borrow_mut();
                    if let Some(state) = conns.get_mut(&active_idx) {
                        for _ in 0..remove_count {
                            if idx + 1 < state.tree_meta.len() {
                                state.tree_meta.remove(idx + 1);
                            }
                        }
                    }
                }
            } else {
                // Expand: fetch children asynchronously
                let dn = {
                    let active_idx = match active {
                        Some(a) => a,
                        None => return,
                    };
                    let conns = connections.borrow();
                    match conns.get(&active_idx) {
                        Some(s) if idx < s.tree_meta.len() => s.tree_meta[idx].dn.clone(),
                        _ => return,
                    }
                };

                let mut loading_node = node.clone();
                loading_node.expanded = true;
                loading_node.is_loading = true;
                tree_model.set_row_data(idx, loading_node);

                let weak = weak.clone();
                let connections = connections.clone();
                let active_profile = active_profile.clone();
                let tree_model = tree_model.clone();
                let indent = node.indent_level + 1;

                #[allow(clippy::await_holding_refcell_ref)]
                slint::spawn_local(Compat::new(async move {
                    let active_idx = match *active_profile.borrow() {
                        Some(a) => a,
                        None => return,
                    };

                    let children = {
                        let mut conns = connections.borrow_mut();
                        let state = match conns.get_mut(&active_idx) {
                            Some(s) => s,
                            None => return,
                        };
                        state.conn.search_children(&dn).await
                    };

                    match children {
                        Ok(entries) => {
                            if idx < tree_model.row_count() {
                                let mut updated = tree_model.row_data(idx).unwrap();
                                updated.is_loading = false;
                                updated.has_children = !entries.is_empty();
                                tree_model.set_row_data(idx, updated);
                            }

                            let insert_pos = idx + 1;
                            let mut new_meta = Vec::new();
                            let mut new_nodes = Vec::new();

                            for entry in &entries {
                                new_meta.push(TreeNodeMeta {
                                    dn: entry.dn.clone(),
                                    indent_level: indent,
                                    has_children: true,
                                });
                                new_nodes.push(TreeNode {
                                    text: SharedString::from(entry.rdn()),
                                    indent_level: indent,
                                    expanded: false,
                                    has_children: true,
                                    is_loading: false,
                                    is_selected: false,
                                });
                            }

                            for (i, node) in new_nodes.into_iter().enumerate() {
                                tree_model.insert(insert_pos + i, node);
                            }

                            {
                                let mut conns = connections.borrow_mut();
                                if let Some(state) = conns.get_mut(&active_idx) {
                                    for (i, m) in new_meta.into_iter().enumerate() {
                                        state.tree_meta.insert(insert_pos + i, m);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to expand {}: {}", dn, e);
                            if idx < tree_model.row_count() {
                                let mut updated = tree_model.row_data(idx).unwrap();
                                updated.is_loading = false;
                                updated.expanded = false;
                                tree_model.set_row_data(idx, updated);
                            }
                            if let Some(win) = weak.upgrade() {
                                win.set_status_message(SharedString::from(format!(
                                    "Expand failed: {}",
                                    e
                                )));
                                win.set_status_is_error(true);
                            }
                        }
                    }
                }))
                .unwrap();
            }
        });
    }

    // --- Tree node selected ---
    {
        let weak = main_window.as_weak();
        let connections = connections.clone();
        let active_profile = active_profile.clone();
        let tree_model = tree_model.clone();
        let attr_model = attr_model.clone();

        main_window.on_tree_node_selected(move |index| {
            let idx = index as usize;
            if idx >= tree_model.row_count() {
                return;
            }

            let row_count = tree_model.row_count();
            for i in 0..row_count {
                let mut node = tree_model.row_data(i).unwrap();
                let should_select = i == idx;
                if node.is_selected != should_select {
                    node.is_selected = should_select;
                    tree_model.set_row_data(i, node);
                }
            }

            if let Some(win) = weak.upgrade() {
                win.set_tree_selected_index(index);
            }

            let active_idx = match *active_profile.borrow() {
                Some(a) => a,
                None => return,
            };

            let dn = {
                let conns = connections.borrow();
                match conns.get(&active_idx) {
                    Some(s) if idx < s.tree_meta.len() => s.tree_meta[idx].dn.clone(),
                    _ => return,
                }
            };

            if let Some(win) = weak.upgrade() {
                win.set_entry_dn(SharedString::from(&dn));
            }

            let weak = weak.clone();
            let connections = connections.clone();
            let active_profile = active_profile.clone();
            let attr_model = attr_model.clone();

            #[allow(clippy::await_holding_refcell_ref)]
            slint::spawn_local(Compat::new(async move {
                let active_idx = match *active_profile.borrow() {
                    Some(a) => a,
                    None => return,
                };

                let entry = {
                    let mut conns = connections.borrow_mut();
                    let state = match conns.get_mut(&active_idx) {
                        Some(s) => s,
                        None => return,
                    };
                    state.conn.search_entry(&dn).await
                };

                match entry {
                    Ok(Some(entry)) => {
                        let mut rows = Vec::new();
                        for (name, values) in &entry.attributes {
                            if values.len() == 1 {
                                rows.push(AttributeRow {
                                    name: SharedString::from(name.as_str()),
                                    value: SharedString::from(values[0].as_str()),
                                });
                            } else {
                                for val in values {
                                    rows.push(AttributeRow {
                                        name: SharedString::from(name.as_str()),
                                        value: SharedString::from(val.as_str()),
                                    });
                                }
                            }
                        }
                        attr_model.set_vec(rows);

                        if let Some(win) = weak.upgrade() {
                            win.set_entry_dn(SharedString::from(&entry.dn));
                        }
                    }
                    Ok(None) => {
                        attr_model.set_vec(vec![]);
                        if let Some(win) = weak.upgrade() {
                            win.set_status_message(SharedString::from("Entry not found"));
                            win.set_status_is_error(true);
                        }
                    }
                    Err(e) => {
                        error!("Failed to fetch entry {}: {}", dn, e);
                        if let Some(win) = weak.upgrade() {
                            win.set_status_message(SharedString::from(format!(
                                "Failed to load entry: {}",
                                e
                            )));
                            win.set_status_is_error(true);
                        }
                    }
                }
            }))
            .unwrap();
        });
    }

    // --- New profile requested ---
    {
        let weak = main_window.as_weak();
        main_window.on_new_profile_requested(move || {
            if let Some(win) = weak.upgrade() {
                win.set_profile_dialog_title("New Profile".into());
                win.set_profile_dialog_name(SharedString::default());
                win.set_profile_dialog_host(SharedString::default());
                win.set_profile_dialog_port("389".into());
                win.set_profile_dialog_tls_mode("auto".into());
                win.set_profile_dialog_bind_dn(SharedString::default());
                win.set_profile_dialog_base_dn(SharedString::default());
                win.set_profile_dialog_credential_method("prompt".into());
                win.set_profile_dialog_folder(SharedString::default());
                win.set_profile_dialog_labels(SharedString::default());
                win.set_profile_dialog_visible(true);
            }
        });
    }

    // --- Save profile ---
    {
        let weak = main_window.as_weak();
        let config = config.clone();
        let connections = connections.clone();
        let active_profile = active_profile.clone();
        let expanded_folders = expanded_folders.clone();
        let sidebar_filter = sidebar_filter.clone();
        let sidebar_model = sidebar_model.clone();

        main_window.on_save_profile(
            move |name,
                  host,
                  port,
                  tls_mode,
                  bind_dn,
                  base_dn,
                  credential_method,
                  folder,
                  labels| {
                let tls = match tls_mode.as_str() {
                    "ldaps" => TlsMode::Ldaps,
                    "starttls" => TlsMode::StartTls,
                    "none" => TlsMode::None,
                    _ => TlsMode::Auto,
                };

                let cred = match credential_method.as_str() {
                    "command" => CredentialMethod::Command,
                    "keychain" => CredentialMethod::Keychain,
                    "vault" => CredentialMethod::Vault,
                    _ => CredentialMethod::Prompt,
                };

                let profile = ConnectionProfile {
                    name: name.to_string(),
                    host: host.to_string(),
                    port: port as u16,
                    tls_mode: tls,
                    bind_dn: if bind_dn.is_empty() {
                        None
                    } else {
                        Some(bind_dn.to_string())
                    },
                    base_dn: if base_dn.is_empty() {
                        None
                    } else {
                        Some(base_dn.to_string())
                    },
                    credential_method: cred,
                    password_command: None,
                    page_size: 500,
                    timeout_secs: 10,
                    relax_rules: false,
                    folder: if folder.is_empty() {
                        None
                    } else {
                        Some(folder.to_string())
                    },
                    read_only: false,
                    offline: false,
                    labels: labels
                        .to_string()
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                };

                let profile_name = profile.name.clone();
                config.borrow_mut().connections.push(profile);

                if let Err(e) = config.borrow().save() {
                    error!("Failed to save config: {}", e);
                    if let Some(win) = weak.upgrade() {
                        win.set_status_message(SharedString::from(format!(
                            "Failed to save profile: {}",
                            e
                        )));
                        win.set_status_is_error(true);
                    }
                    return;
                }

                info!("Saved new profile: {}", &profile_name);

                if let Some(win) = weak.upgrade() {
                    win.set_profile_dialog_visible(false);
                    win.set_status_message(SharedString::from(format!(
                        "Profile '{}' saved",
                        &profile_name
                    )));
                    win.set_status_is_error(false);

                    // Rebuild sidebar
                    let cfg = config.borrow();
                    let conns = connections.borrow();
                    let active = *active_profile.borrow();
                    let expanded = expanded_folders.borrow();
                    let filter = sidebar_filter.borrow();
                    refresh_sidebar(
                        &win,
                        &sidebar_model,
                        &cfg,
                        &conns,
                        active,
                        &expanded,
                        &filter,
                    );
                }
            },
        );
    }

    // --- Vault dialog on startup ---
    {
        let vault_path = Vault::default_path();
        let cfg = config.borrow();
        if cfg.general.vault_enabled && Vault::exists(&vault_path) {
            main_window.set_vault_dialog_visible(true);
            info!("Vault file found, prompting for password");
        }
    }

    // --- Vault unlock ---
    {
        let weak = main_window.as_weak();
        let vault_state = vault_state.clone();

        main_window.on_vault_unlock(move |password| {
            let vault_path = Vault::default_path();
            match Vault::open(&vault_path, password.as_str()) {
                Ok(vault) => {
                    info!("Vault unlocked successfully");
                    *vault_state.borrow_mut() = Some(vault);
                    if let Some(win) = weak.upgrade() {
                        win.set_vault_dialog_visible(false);
                        win.set_vault_error(SharedString::default());
                        win.set_status_message("Vault unlocked".into());
                        win.set_status_is_error(false);
                    }
                }
                Err(e) => {
                    error!("Vault unlock failed: {}", e);
                    if let Some(win) = weak.upgrade() {
                        win.set_vault_error(SharedString::from(format!(
                            "Failed to unlock vault: {}",
                            e
                        )));
                    }
                }
            }
        });
    }

    // --- Vault skip ---
    {
        let weak = main_window.as_weak();

        main_window.on_vault_skip(move || {
            info!("Vault skipped by user");
            if let Some(win) = weak.upgrade() {
                win.set_vault_dialog_visible(false);
                win.set_vault_error(SharedString::default());
            }
        });
    }

    // --- Show export dialog ---
    {
        let weak = main_window.as_weak();
        let connections = connections.clone();
        let active_profile = active_profile.clone();

        main_window.on_show_export_dialog(move || {
            if let Some(win) = weak.upgrade() {
                let base_dn = {
                    let active = *active_profile.borrow();
                    let conns = connections.borrow();
                    if let Some(active_idx) = active {
                        if let Some(ref s) = conns.get(&active_idx) {
                            let selected = win.get_tree_selected_index();
                            if selected >= 0 && (selected as usize) < s.tree_meta.len() {
                                s.tree_meta[selected as usize].dn.clone()
                            } else if !s.tree_meta.is_empty() {
                                s.tree_meta[0].dn.clone()
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    }
                };

                win.set_export_base_dn(SharedString::from(&base_dn));
                win.set_export_dialog_visible(true);
            }
        });
    }

    // --- Export execute ---
    {
        let weak = main_window.as_weak();
        let connections = connections.clone();
        let active_profile = active_profile.clone();

        main_window.on_export_execute(move |base_dn, file_path, format_index| {
            let base_dn = base_dn.to_string();
            let file_path = file_path.to_string();

            if file_path.is_empty() {
                if let Some(win) = weak.upgrade() {
                    win.set_status_message("Export failed: no file path specified".into());
                    win.set_status_is_error(true);
                }
                return;
            }

            let file_path = {
                let path = std::path::Path::new(&file_path);
                if path.extension().is_some() {
                    file_path
                } else {
                    let ext = match format_index {
                        1 => ".json",
                        2 => ".csv",
                        3 => ".xlsx",
                        _ => ".ldif",
                    };
                    format!("{}{}", file_path, ext)
                }
            };

            if let Some(win) = weak.upgrade() {
                win.set_export_dialog_visible(false);
                win.set_status_message(SharedString::from("Exporting..."));
                win.set_status_is_error(false);
            }

            let weak = weak.clone();
            let connections = connections.clone();
            let active_profile = active_profile.clone();

            #[allow(clippy::await_holding_refcell_ref)]
            slint::spawn_local(Compat::new(async move {
                let active_idx = match *active_profile.borrow() {
                    Some(a) => a,
                    None => {
                        if let Some(win) = weak.upgrade() {
                            win.set_status_message("Export failed: no active connection".into());
                            win.set_status_is_error(true);
                        }
                        return;
                    }
                };

                let entries = {
                    let mut conns = connections.borrow_mut();
                    let state = match conns.get_mut(&active_idx) {
                        Some(s) => s,
                        None => {
                            if let Some(win) = weak.upgrade() {
                                win.set_status_message(
                                    "Export failed: no active connection".into(),
                                );
                                win.set_status_is_error(true);
                            }
                            return;
                        }
                    };
                    state
                        .conn
                        .search_subtree(&base_dn, "(objectClass=*)", &["*"])
                        .await
                };

                match entries {
                    Ok(entries) => {
                        let path = std::path::Path::new(&file_path);
                        match loom_core::export::export_entries(&entries, path, &[]) {
                            Ok(count) => {
                                info!("Exported {} entries to {}", count, &file_path);
                                if let Some(win) = weak.upgrade() {
                                    win.set_status_message(SharedString::from(format!(
                                        "Exported {} entries to {}",
                                        count, &file_path
                                    )));
                                    win.set_status_is_error(false);
                                }
                            }
                            Err(e) => {
                                error!("Export failed: {}", e);
                                if let Some(win) = weak.upgrade() {
                                    win.set_status_message(SharedString::from(format!(
                                        "Export failed: {}",
                                        e
                                    )));
                                    win.set_status_is_error(true);
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("Export search failed: {}", e);
                        if let Some(win) = weak.upgrade() {
                            win.set_status_message(SharedString::from(format!(
                                "Export failed: {}",
                                e
                            )));
                            win.set_status_is_error(true);
                        }
                    }
                }
            }))
            .unwrap();
        });
    }

    // --- Export cancel ---
    {
        let weak = main_window.as_weak();

        main_window.on_export_cancel(move || {
            if let Some(win) = weak.upgrade() {
                win.set_export_dialog_visible(false);
            }
        });
    }

    // --- Show search dialog ---
    {
        let weak = main_window.as_weak();
        let connections = connections.clone();
        let active_profile = active_profile.clone();

        main_window.on_show_search_dialog(move || {
            if let Some(win) = weak.upgrade() {
                let base_dn = {
                    let active = *active_profile.borrow();
                    let conns = connections.borrow();
                    if let Some(active_idx) = active {
                        if let Some(ref s) = conns.get(&active_idx) {
                            let selected = win.get_tree_selected_index();
                            if selected >= 0 && (selected as usize) < s.tree_meta.len() {
                                s.tree_meta[selected as usize].dn.clone()
                            } else if !s.tree_meta.is_empty() {
                                s.tree_meta[0].dn.clone()
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        }
                    } else {
                        String::new()
                    }
                };

                win.set_search_base_dn(SharedString::from(&base_dn));
                win.set_search_filter("(objectClass=*)".into());
                win.set_search_scope_index(0);
                win.set_search_results(ModelRc::from(Rc::new(VecModel::<SearchResult>::default())));
                win.set_search_dialog_visible(true);
            }
        });
    }

    // --- Search execute ---
    let search_result_dns: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    {
        let weak = main_window.as_weak();
        let connections = connections.clone();
        let active_profile = active_profile.clone();
        let search_result_dns = search_result_dns.clone();

        main_window.on_search_execute(move |base_dn, filter, scope_index| {
            let base_dn = base_dn.to_string();
            let filter = filter.to_string();

            if let Some(win) = weak.upgrade() {
                win.set_status_message("Searching...".into());
                win.set_status_is_error(false);
            }

            let weak = weak.clone();
            let connections = connections.clone();
            let active_profile = active_profile.clone();
            let search_result_dns = search_result_dns.clone();

            #[allow(clippy::await_holding_refcell_ref)]
            slint::spawn_local(Compat::new(async move {
                let scope = match scope_index {
                    1 => loom_core::Scope::OneLevel,
                    2 => loom_core::Scope::Base,
                    _ => loom_core::Scope::Subtree,
                };

                let active_idx = match *active_profile.borrow() {
                    Some(a) => a,
                    None => {
                        if let Some(win) = weak.upgrade() {
                            win.set_status_message("Search failed: no active connection".into());
                            win.set_status_is_error(true);
                        }
                        return;
                    }
                };

                let results = {
                    let mut conns = connections.borrow_mut();
                    let state = match conns.get_mut(&active_idx) {
                        Some(s) => s,
                        None => {
                            if let Some(win) = weak.upgrade() {
                                win.set_status_message(
                                    "Search failed: no active connection".into(),
                                );
                                win.set_status_is_error(true);
                            }
                            return;
                        }
                    };
                    state.conn.search(&base_dn, scope, &filter, &["dn"]).await
                };

                match results {
                    Ok(entries) => {
                        let count = entries.len();
                        let mut dns = Vec::with_capacity(count);
                        let results_model = Rc::new(VecModel::<SearchResult>::default());

                        for entry in &entries {
                            dns.push(entry.dn.clone());
                            results_model.push(SearchResult {
                                text: SharedString::from(&entry.dn),
                            });
                        }

                        *search_result_dns.borrow_mut() = dns;

                        if let Some(win) = weak.upgrade() {
                            win.set_search_results(ModelRc::from(results_model));
                            win.set_status_message(SharedString::from(format!(
                                "Found {} entries",
                                count
                            )));
                            win.set_status_is_error(false);
                        }
                    }
                    Err(e) => {
                        error!("Search failed: {}", e);
                        if let Some(win) = weak.upgrade() {
                            win.set_status_message(SharedString::from(format!(
                                "Search failed: {}",
                                e
                            )));
                            win.set_status_is_error(true);
                        }
                    }
                }
            }))
            .unwrap();
        });
    }

    // --- Search result selected ---
    {
        let weak = main_window.as_weak();
        let connections = connections.clone();
        let active_profile = active_profile.clone();
        let attr_model = attr_model.clone();
        let search_result_dns = search_result_dns.clone();

        main_window.on_search_result_selected(move |index| {
            let dn = {
                let dns = search_result_dns.borrow();
                let idx = index as usize;
                if idx >= dns.len() {
                    return;
                }
                dns[idx].clone()
            };

            if let Some(win) = weak.upgrade() {
                win.set_search_dialog_visible(false);
                win.set_entry_dn(SharedString::from(&dn));
            }

            let weak = weak.clone();
            let connections = connections.clone();
            let active_profile = active_profile.clone();
            let attr_model = attr_model.clone();

            #[allow(clippy::await_holding_refcell_ref)]
            slint::spawn_local(Compat::new(async move {
                let active_idx = match *active_profile.borrow() {
                    Some(a) => a,
                    None => return,
                };

                let entry = {
                    let mut conns = connections.borrow_mut();
                    let state = match conns.get_mut(&active_idx) {
                        Some(s) => s,
                        None => return,
                    };
                    state.conn.search_entry(&dn).await
                };

                match entry {
                    Ok(Some(entry)) => {
                        let mut rows = Vec::new();
                        for (name, values) in &entry.attributes {
                            if values.len() == 1 {
                                rows.push(AttributeRow {
                                    name: SharedString::from(name.as_str()),
                                    value: SharedString::from(values[0].as_str()),
                                });
                            } else {
                                for val in values {
                                    rows.push(AttributeRow {
                                        name: SharedString::from(name.as_str()),
                                        value: SharedString::from(val.as_str()),
                                    });
                                }
                            }
                        }
                        attr_model.set_vec(rows);

                        if let Some(win) = weak.upgrade() {
                            win.set_entry_dn(SharedString::from(&entry.dn));
                        }
                    }
                    Ok(None) => {
                        attr_model.set_vec(vec![]);
                        if let Some(win) = weak.upgrade() {
                            win.set_status_message("Entry not found".into());
                            win.set_status_is_error(true);
                        }
                    }
                    Err(e) => {
                        error!("Failed to fetch entry {}: {}", dn, e);
                        if let Some(win) = weak.upgrade() {
                            win.set_status_message(SharedString::from(format!(
                                "Failed to load entry: {}",
                                e
                            )));
                            win.set_status_is_error(true);
                        }
                    }
                }
            }))
            .unwrap();
        });
    }

    // --- Search cancel ---
    {
        let weak = main_window.as_weak();

        main_window.on_search_cancel(move || {
            if let Some(win) = weak.upgrade() {
                win.set_search_dialog_visible(false);
            }
        });
    }

    // --- Show theme selector ---
    {
        let weak = main_window.as_weak();
        let config = config.clone();

        main_window.on_show_theme_selector(move || {
            if let Some(win) = weak.upgrade() {
                let cfg = config.borrow();
                win.set_current_theme(SharedString::from(&cfg.general.theme));
                win.set_theme_selector_visible(true);
            }
        });
    }

    // --- Change theme ---
    {
        let weak = main_window.as_weak();
        let config = config.clone();

        main_window.on_change_theme(move |theme_name| {
            let name = theme_name.to_string();
            if let Some(win) = weak.upgrade() {
                apply_theme(&win, &name);
                win.set_current_theme(SharedString::from(&name));
                win.set_theme_selector_visible(false);
                win.set_status_message(SharedString::from(format!("Theme changed to {}", &name)));
                win.set_status_is_error(false);
            }
            config.borrow_mut().general.theme = name;
            if let Err(e) = config.borrow().save() {
                error!("Failed to save config: {}", e);
            }
        });
    }

    // --- Menu: about ---
    {
        let weak = main_window.as_weak();
        main_window.on_show_about(move || {
            if let Some(win) = weak.upgrade() {
                win.set_status_message(SharedString::from(format!(
                    "Loom Browser v{} - An LDAP directory browser",
                    env!("CARGO_PKG_VERSION")
                )));
                win.set_status_is_error(false);
            }
        });
    }

    // --- Menu: disconnect ---
    {
        let weak = main_window.as_weak();
        let connections = connections.clone();
        let active_profile = active_profile.clone();
        let config = config.clone();
        let expanded_folders = expanded_folders.clone();
        let sidebar_filter = sidebar_filter.clone();
        let tree_model = tree_model.clone();
        let attr_model = attr_model.clone();
        let sidebar_model = sidebar_model.clone();

        main_window.on_menu_disconnect(move || {
            let active = *active_profile.borrow();
            if let Some(active_idx) = active {
                connections.borrow_mut().remove(&active_idx);
            }

            // Clear UI
            tree_model.set_vec(vec![]);
            attr_model.set_vec(vec![]);

            // Set active to next open connection or None
            let new_active = {
                let conns = connections.borrow();
                conns.keys().next().copied()
            };

            *active_profile.borrow_mut() = new_active;

            // Load next connection's state if any
            if let Some(win) = weak.upgrade() {
                if let Some(next_idx) = new_active {
                    let conns = connections.borrow();
                    if let Some(state) = conns.get(&next_idx) {
                        load_connection_state(state, &tree_model, &attr_model, &win);
                    }
                } else {
                    win.set_entry_dn(SharedString::default());
                    win.set_tree_selected_index(-1);
                }

                win.set_status_message(SharedString::from("Disconnected"));
                win.set_status_is_error(false);

                let cfg = config.borrow();
                let conns = connections.borrow();
                let expanded = expanded_folders.borrow();
                let filter = sidebar_filter.borrow();
                refresh_sidebar(
                    &win,
                    &sidebar_model,
                    &cfg,
                    &conns,
                    new_active,
                    &expanded,
                    &filter,
                );
            }
        });
    }

    main_window.run()
}

#[allow(clippy::too_many_arguments)]
fn spawn_connect(
    weak: slint::Weak<MainWindow>,
    connections: Rc<RefCell<HashMap<usize, ConnectionState>>>,
    active_profile: Rc<RefCell<Option<usize>>>,
    config: Rc<RefCell<AppConfig>>,
    expanded_folders: Rc<RefCell<HashSet<String>>>,
    sidebar_filter: Rc<RefCell<String>>,
    tree_model: Rc<VecModel<TreeNode>>,
    attr_model: Rc<VecModel<AttributeRow>>,
    sidebar_model: Rc<VecModel<SidebarProfile>>,
    settings: ConnectionSettings,
    host: String,
    _profile_name: String,
    base_dn: String,
    bind_dn: Option<String>,
    password: Option<String>,
    profile_index: usize,
) {
    slint::spawn_local(Compat::new(async move {
        // Connect
        let mut conn = match LdapConnection::connect(settings, None).await {
            Ok(c) => c,
            Err(e) => {
                error!("Connection failed: {}", e);
                if let Some(win) = weak.upgrade() {
                    win.set_status_message(SharedString::from(format!("Connection failed: {}", e)));
                    win.set_status_is_error(true);
                }
                return;
            }
        };

        // Bind
        let bind_result = if let (Some(ref dn), Some(ref pw)) = (&bind_dn, &password) {
            conn.simple_bind(dn, pw).await
        } else {
            conn.anonymous_bind().await
        };

        if let Err(e) = bind_result {
            error!("Bind failed: {}", e);
            if let Some(win) = weak.upgrade() {
                win.set_status_message(SharedString::from(format!("Bind failed: {}", e)));
                win.set_status_is_error(true);
            }
            return;
        }

        // Store credentials for reconnection
        if let (Some(dn), Some(pw)) = (bind_dn, password) {
            conn.store_credentials(dn, pw);
        }

        info!("Connected to {}", &host);

        // Populate tree with base DN + children
        let effective_base = if base_dn.is_empty() {
            conn.base_dn.clone()
        } else {
            base_dn.clone()
        };

        let children = match conn.search_children(&effective_base).await {
            Ok(entries) => entries,
            Err(e) => {
                error!("Failed to search base DN: {}", e);
                if let Some(win) = weak.upgrade() {
                    win.set_status_message(SharedString::from(format!(
                        "Connected to {} (search failed: {})",
                        &host, e
                    )));
                    win.set_status_is_error(false);
                }
                // Still store connection even if initial search fails
                // Save current active state first
                if let Some(win) = weak.upgrade() {
                    let current_active = *active_profile.borrow();
                    save_active_state(
                        &mut connections.borrow_mut(),
                        current_active,
                        &tree_model,
                        &attr_model,
                        &win.get_entry_dn(),
                        win.get_tree_selected_index(),
                    );
                }

                connections.borrow_mut().insert(
                    profile_index,
                    ConnectionState {
                        conn,
                        tree_meta: Vec::new(),
                        tree_nodes: Vec::new(),
                        attributes: Vec::new(),
                        entry_dn: SharedString::default(),
                        selected_index: -1,
                    },
                );
                *active_profile.borrow_mut() = Some(profile_index);

                // Refresh sidebar
                if let Some(win) = weak.upgrade() {
                    tree_model.set_vec(vec![]);
                    attr_model.set_vec(vec![]);
                    win.set_entry_dn(SharedString::default());
                    win.set_tree_selected_index(-1);

                    let cfg = config.borrow();
                    let conns = connections.borrow();
                    let expanded = expanded_folders.borrow();
                    let filter = sidebar_filter.borrow();
                    refresh_sidebar(
                        &win,
                        &sidebar_model,
                        &cfg,
                        &conns,
                        Some(profile_index),
                        &expanded,
                        &filter,
                    );
                }
                return;
            }
        };

        // Build flat tree model
        let mut meta = Vec::new();
        let mut nodes = Vec::new();

        // Root node
        meta.push(TreeNodeMeta {
            dn: effective_base.clone(),
            indent_level: 0,
            has_children: !children.is_empty(),
        });
        nodes.push(TreeNode {
            text: SharedString::from(&effective_base),
            indent_level: 0,
            expanded: true,
            has_children: !children.is_empty(),
            is_loading: false,
            is_selected: false,
        });

        // Children
        for child in &children {
            meta.push(TreeNodeMeta {
                dn: child.dn.clone(),
                indent_level: 1,
                has_children: true,
            });
            nodes.push(TreeNode {
                text: SharedString::from(child.rdn()),
                indent_level: 1,
                expanded: false,
                has_children: true,
                is_loading: false,
                is_selected: false,
            });
        }

        // Save current active connection state before switching
        if let Some(win) = weak.upgrade() {
            let current_active = *active_profile.borrow();
            save_active_state(
                &mut connections.borrow_mut(),
                current_active,
                &tree_model,
                &attr_model,
                &win.get_entry_dn(),
                win.get_tree_selected_index(),
            );
        }

        // Update UI with new connection's tree
        tree_model.set_vec(nodes.clone());
        attr_model.set_vec(vec![]);

        // Store connection state
        connections.borrow_mut().insert(
            profile_index,
            ConnectionState {
                conn,
                tree_meta: meta,
                tree_nodes: nodes,
                attributes: Vec::new(),
                entry_dn: SharedString::default(),
                selected_index: -1,
            },
        );
        *active_profile.borrow_mut() = Some(profile_index);

        if let Some(win) = weak.upgrade() {
            win.set_status_message(SharedString::from(format!(
                "Connected to {} ({} entries)",
                &host,
                children.len()
            )));
            win.set_status_is_error(false);
            win.set_entry_dn(SharedString::default());
            win.set_tree_selected_index(-1);

            // Refresh sidebar
            let cfg = config.borrow();
            let conns = connections.borrow();
            let expanded = expanded_folders.borrow();
            let filter = sidebar_filter.borrow();
            refresh_sidebar(
                &win,
                &sidebar_model,
                &cfg,
                &conns,
                Some(profile_index),
                &expanded,
                &filter,
            );
        }
    }))
    .unwrap();
}

fn apply_theme(window: &MainWindow, theme_name: &str) {
    let theme = window.global::<AppTheme>();
    match theme_name {
        "light" => {
            theme.set_bg_primary(slint::Color::from_rgb_u8(0xfa, 0xfa, 0xfa));
            theme.set_bg_secondary(slint::Color::from_rgb_u8(0xf0, 0xf0, 0xf0));
            theme.set_bg_tertiary(slint::Color::from_rgb_u8(0xe8, 0xe8, 0xe8));
            theme.set_bg_hover(slint::Color::from_rgb_u8(0xe0, 0xe0, 0xe0));
            theme.set_bg_selected(slint::Color::from_rgb_u8(0xd8, 0xd8, 0xd8));
            theme.set_fg_primary(slint::Color::from_rgb_u8(0x1a, 0x1a, 0x1a));
            theme.set_fg_secondary(slint::Color::from_rgb_u8(0x55, 0x55, 0x55));
            theme.set_fg_muted(slint::Color::from_rgb_u8(0x99, 0x99, 0x99));
            theme.set_accent(slint::Color::from_rgb_u8(0x22, 0x7c, 0xe6));
            theme.set_border(slint::Color::from_rgb_u8(0xd0, 0xd0, 0xd0));
            theme.set_border_focus(slint::Color::from_rgb_u8(0x22, 0x7c, 0xe6));
        }
        "solarized" => {
            theme.set_bg_primary(slint::Color::from_rgb_u8(0x00, 0x2b, 0x36));
            theme.set_bg_secondary(slint::Color::from_rgb_u8(0x07, 0x36, 0x42));
            theme.set_bg_tertiary(slint::Color::from_rgb_u8(0x0a, 0x40, 0x4d));
            theme.set_fg_primary(slint::Color::from_rgb_u8(0x83, 0x94, 0x96));
            theme.set_fg_secondary(slint::Color::from_rgb_u8(0x65, 0x7b, 0x83));
            theme.set_accent(slint::Color::from_rgb_u8(0x26, 0x8b, 0xd2));
            theme.set_border(slint::Color::from_rgb_u8(0x58, 0x6e, 0x75));
        }
        "nord" => {
            theme.set_bg_primary(slint::Color::from_rgb_u8(0x2e, 0x34, 0x40));
            theme.set_bg_secondary(slint::Color::from_rgb_u8(0x3b, 0x42, 0x52));
            theme.set_bg_tertiary(slint::Color::from_rgb_u8(0x43, 0x4c, 0x5e));
            theme.set_fg_primary(slint::Color::from_rgb_u8(0xec, 0xef, 0xf4));
            theme.set_fg_secondary(slint::Color::from_rgb_u8(0xd8, 0xde, 0xe9));
            theme.set_accent(slint::Color::from_rgb_u8(0x88, 0xc0, 0xd0));
            theme.set_border(slint::Color::from_rgb_u8(0x4c, 0x56, 0x6a));
        }
        "matrix" => {
            theme.set_bg_primary(slint::Color::from_rgb_u8(0x0a, 0x0a, 0x0a));
            theme.set_bg_secondary(slint::Color::from_rgb_u8(0x12, 0x12, 0x12));
            theme.set_bg_tertiary(slint::Color::from_rgb_u8(0x1a, 0x1a, 0x1a));
            theme.set_fg_primary(slint::Color::from_rgb_u8(0x00, 0xff, 0x00));
            theme.set_fg_secondary(slint::Color::from_rgb_u8(0x00, 0xcc, 0x00));
            theme.set_fg_muted(slint::Color::from_rgb_u8(0x00, 0x66, 0x00));
            theme.set_accent(slint::Color::from_rgb_u8(0x00, 0xcc, 0x00));
            theme.set_border(slint::Color::from_rgb_u8(0x00, 0x33, 0x00));
        }
        _ => {} // "dark" is the default from theme.slint
    }
}
