slint::include_modules!();

use std::cell::RefCell;
use std::rc::Rc;

use async_compat::Compat;
use slint::{Model, ModelRc, SharedString, VecModel};
use tracing::{error, info};

use loom_core::config::AppConfig;
use loom_core::connection::LdapConnection;

/// Per-node metadata stored alongside the flat tree model.
#[derive(Clone, Debug)]
#[allow(dead_code)]
struct TreeNodeMeta {
    dn: String,
    indent_level: i32,
    has_children: bool,
}

/// State for a single connection tab.
struct ConnectionState {
    conn: LdapConnection,
    tree_meta: Vec<TreeNodeMeta>,
}

pub fn run() -> Result<(), slint::PlatformError> {
    let config = AppConfig::load();
    let main_window = MainWindow::new()?;

    apply_theme(&main_window, &config.general.theme);

    main_window.set_status_message("Ready".into());

    // Shared state: config and active connection
    let config = Rc::new(RefCell::new(config));
    let conn_state: Rc<RefCell<Option<ConnectionState>>> = Rc::new(RefCell::new(None));

    // Shared tree VecModel so we can mutate it from callbacks
    let tree_model: Rc<VecModel<TreeNode>> = Rc::new(VecModel::default());
    main_window.set_tree_model(ModelRc::from(tree_model.clone()));

    // Shared attributes VecModel
    let attr_model: Rc<VecModel<AttributeRow>> = Rc::new(VecModel::default());
    main_window.set_attributes(ModelRc::from(attr_model.clone()));

    // Shared tabs VecModel
    let tabs_model: Rc<VecModel<TabInfo>> = Rc::new(VecModel::default());
    main_window.set_tabs(ModelRc::from(tabs_model.clone()));

    // --- Task 10 & 11: connect-profile callback ---
    {
        let weak = main_window.as_weak();
        let config = config.clone();
        let conn_state = conn_state.clone();
        let tree_model = tree_model.clone();
        let attr_model = attr_model.clone();
        let tabs_model = tabs_model.clone();

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
            let _bind_dn = profile.bind_dn.clone();
            let _credential_method = profile.credential_method.clone();

            // Update status
            if let Some(win) = weak.upgrade() {
                win.set_status_message(SharedString::from(format!("Connecting to {}...", &host)));
                win.set_status_is_error(false);
            }

            let weak = weak.clone();
            let conn_state = conn_state.clone();
            let tree_model = tree_model.clone();
            let attr_model = attr_model.clone();
            let tabs_model = tabs_model.clone();

            slint::spawn_local(Compat::new(async move {
                // Connect
                let mut conn = match LdapConnection::connect(settings, None).await {
                    Ok(c) => c,
                    Err(e) => {
                        error!("Connection failed: {}", e);
                        if let Some(win) = weak.upgrade() {
                            win.set_status_message(SharedString::from(format!(
                                "Connection failed: {}",
                                e
                            )));
                            win.set_status_is_error(true);
                        }
                        return;
                    }
                };

                // Bind — for now always anonymous; Task 16 adds credential prompts
                let bind_result = conn.anonymous_bind().await;

                if let Err(e) = bind_result {
                    error!("Bind failed: {}", e);
                    if let Some(win) = weak.upgrade() {
                        win.set_status_message(SharedString::from(format!("Bind failed: {}", e)));
                        win.set_status_is_error(true);
                    }
                    return;
                }

                info!("Connected to {}", &host);

                // --- Task 11: Create tab ---
                if let Some(win) = weak.upgrade() {
                    let tab = TabInfo {
                        id: profile_index,
                        title: SharedString::from(&profile_name),
                    };
                    tabs_model.push(tab);
                    let tab_count = tabs_model.row_count() as i32;
                    win.set_active_tab(tab_count - 1);
                }

                // --- Task 12: Populate tree with base DN + children ---
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
                        *conn_state.borrow_mut() = Some(ConnectionState {
                            conn,
                            tree_meta: Vec::new(),
                        });
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
                        has_children: true, // assume children until proven otherwise
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

                // Clear and repopulate tree model
                while tree_model.row_count() > 0 {
                    tree_model.remove(0);
                }
                for node in nodes {
                    tree_model.push(node);
                }

                // Clear attributes
                while attr_model.row_count() > 0 {
                    attr_model.remove(0);
                }

                // Store connection state
                *conn_state.borrow_mut() = Some(ConnectionState {
                    conn,
                    tree_meta: meta,
                });

                if let Some(win) = weak.upgrade() {
                    win.set_status_message(SharedString::from(format!(
                        "Connected to {} ({} entries)",
                        &host,
                        children.len()
                    )));
                    win.set_status_is_error(false);
                    win.set_entry_dn(SharedString::default());
                    win.set_tree_selected_index(-1);
                }
            }))
            .unwrap();
        });
    }

    // --- Task 12: tree-toggle-expand callback ---
    {
        let weak = main_window.as_weak();
        let conn_state = conn_state.clone();
        let tree_model = tree_model.clone();

        main_window.on_tree_toggle_expand(move |index| {
            let idx = index as usize;
            if idx >= tree_model.row_count() {
                return;
            }

            let node = tree_model.row_data(idx).unwrap();

            if node.expanded {
                // Collapse: remove all descendants
                let my_indent = node.indent_level;
                let mut collapsed_node = node.clone();
                collapsed_node.expanded = false;
                tree_model.set_row_data(idx, collapsed_node);

                // Remove children from both tree_model and meta
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

                // Also remove from meta
                if let Some(state) = conn_state.borrow_mut().as_mut() {
                    for _ in 0..remove_count {
                        if idx + 1 < state.tree_meta.len() {
                            state.tree_meta.remove(idx + 1);
                        }
                    }
                }
            } else {
                // Expand: fetch children asynchronously
                let dn = {
                    let state = conn_state.borrow();
                    match state.as_ref() {
                        Some(s) if idx < s.tree_meta.len() => s.tree_meta[idx].dn.clone(),
                        _ => return,
                    }
                };

                // Mark as loading
                let mut loading_node = node.clone();
                loading_node.expanded = true;
                loading_node.is_loading = true;
                tree_model.set_row_data(idx, loading_node);

                let weak = weak.clone();
                let conn_state = conn_state.clone();
                let tree_model = tree_model.clone();
                let indent = node.indent_level + 1;

                #[allow(clippy::await_holding_refcell_ref)]
                slint::spawn_local(Compat::new(async move {
                    // Safety: spawn_local runs on the single-threaded Slint event loop,
                    // so no concurrent borrow can occur while we await.
                    let children = {
                        let mut state_ref = conn_state.borrow_mut();
                        let state = match state_ref.as_mut() {
                            Some(s) => s,
                            None => return,
                        };
                        state.conn.search_children(&dn).await
                    };

                    match children {
                        Ok(entries) => {
                            // Update node: no longer loading, set has_children
                            if idx < tree_model.row_count() {
                                let mut updated = tree_model.row_data(idx).unwrap();
                                updated.is_loading = false;
                                updated.has_children = !entries.is_empty();
                                tree_model.set_row_data(idx, updated);
                            }

                            // Insert children after current node
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

                            // Insert into tree model (in reverse so positions stay stable)
                            for (i, node) in new_nodes.into_iter().enumerate() {
                                tree_model.insert(insert_pos + i, node);
                            }

                            // Insert into meta
                            if let Some(state) = conn_state.borrow_mut().as_mut() {
                                for (i, m) in new_meta.into_iter().enumerate() {
                                    state.tree_meta.insert(insert_pos + i, m);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to expand {}: {}", dn, e);
                            // Revert loading state
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

    // --- Task 13: tree-node-selected callback ---
    {
        let weak = main_window.as_weak();
        let conn_state = conn_state.clone();
        let tree_model = tree_model.clone();
        let attr_model = attr_model.clone();

        main_window.on_tree_node_selected(move |index| {
            let idx = index as usize;
            if idx >= tree_model.row_count() {
                return;
            }

            // Update selected state in tree model
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

            // Get DN for selected node
            let dn = {
                let state = conn_state.borrow();
                match state.as_ref() {
                    Some(s) if idx < s.tree_meta.len() => s.tree_meta[idx].dn.clone(),
                    _ => return,
                }
            };

            if let Some(win) = weak.upgrade() {
                win.set_entry_dn(SharedString::from(&dn));
            }

            // Fetch attributes asynchronously
            let weak = weak.clone();
            let conn_state = conn_state.clone();
            let attr_model = attr_model.clone();

            #[allow(clippy::await_holding_refcell_ref)]
            slint::spawn_local(Compat::new(async move {
                // Safety: spawn_local runs on the single-threaded Slint event loop,
                // so no concurrent borrow can occur while we await.
                let entry = {
                    let mut state_ref = conn_state.borrow_mut();
                    let state = match state_ref.as_mut() {
                        Some(s) => s,
                        None => return,
                    };
                    state.conn.search_entry(&dn).await
                };

                match entry {
                    Ok(Some(entry)) => {
                        // Clear existing attributes
                        while attr_model.row_count() > 0 {
                            attr_model.remove(0);
                        }

                        // Map BTreeMap to AttributeRow list
                        // Multi-valued attributes get one row per value
                        for (name, values) in &entry.attributes {
                            if values.len() == 1 {
                                attr_model.push(AttributeRow {
                                    name: SharedString::from(name.as_str()),
                                    value: SharedString::from(values[0].as_str()),
                                });
                            } else {
                                for val in values {
                                    attr_model.push(AttributeRow {
                                        name: SharedString::from(name.as_str()),
                                        value: SharedString::from(val.as_str()),
                                    });
                                }
                            }
                        }

                        if let Some(win) = weak.upgrade() {
                            win.set_entry_dn(SharedString::from(&entry.dn));
                        }
                    }
                    Ok(None) => {
                        while attr_model.row_count() > 0 {
                            attr_model.remove(0);
                        }
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

    // --- Task 11: Auto-connect to first profile if available ---
    {
        let config = config.borrow();
        if !config.connections.is_empty() {
            main_window.invoke_connect_profile(0);
        }
    }

    main_window.run()
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
