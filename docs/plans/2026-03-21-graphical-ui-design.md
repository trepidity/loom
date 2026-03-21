# Graphical UI Design

## Overview

Add an optional Slint-based GUI alongside the existing TUI. Users choose which interface to use. Both share the same core library, config, and vault.

## Binary Names

- `loom` — TUI (terminal, short to type)
- `loom-browser` — GUI (graphical, launched from app launcher or terminal)

## Workspace Architecture

```
crates/
  loom-core/          # Shared: LDAP client, vault, config, export (no UI deps)
  loom-tui/           # ratatui-based terminal UI
  loom-ldapbrowser/   # TUI binary → rename to "loom"
  loom-gui/           # (new) Slint-based graphical UI
  loom-browser/       # (new) GUI binary
```

- `loom-core` stays untouched — both UIs depend on it
- Config types move from `loom-tui::config` to `loom-core::config` (both UIs need them)
- Both binaries read the same `~/.config/loom-ldapbrowser/config.toml` and `vault.dat`
- The TUI gains no Slint dependency; the GUI gains no ratatui dependency

## Framework: Slint

- Declarative `.slint` markup language with Rust backend
- Cross-platform: macOS, Linux, Windows, web (wasm)
- GPU-accelerated (Skia or software renderer)
- Built-in widgets: tree views, tables, tabs, split panes
- Live preview during development (edit `.slint` files without recompiling Rust)
- VS Code extension with autocomplete and visual preview
- License: GPLv3 (compatible with this project's GPL license)

## GUI Layout

```
+-----------------------------------------------------+
|  Profiles v | Edit | View | Help                     |
+--------------+--------------------------------------+
|  Tab Bar (Connection1 | Connection2 | +)             |
+------------------------------------------------------+
|  +------------------+---------------------------+    |
|  | DIT Tree         | Attribute Detail          |    |
|  |                  |                           |    |
|  | dc=example       | dn: cn=user1,dc=...      |    |
|  |  +- ou=users     | cn: user1                |    |
|  |  |  +- cn=user1  | mail: user1@ex.com       |    |
|  |  |  +- cn=user2  | objectClass: person       |    |
|  |  +- ou=groups    |                           |    |
|  |                  |                           |    |
|  +------------------+---------------------------+    |
+------------------------------------------------------+
|  Status Bar / Log Panel (collapsible)                |
+------------------------------------------------------+
```

- Profiles are a dropdown in the menu bar, not a sidebar
- Connection actions (connect, disconnect, reconnect) live under Profiles
- Resizable splitter between tree and detail panel
- Right-click context menus for tree nodes and attribute rows

## Menu Structure

- **Profiles** — Connect, Disconnect, Reconnect, ---, saved profiles by folder, ---, New Profile, Edit Profiles, Import/Export Profiles, ---, Quit
- **Edit** — Copy DN, Copy Attribute, ---, New Entry, Delete Entry, ---, Bulk Update, ---, Search (Ctrl+F)
- **View** — Schema Viewer, Toggle Log Panel, ---, Theme > (Dark, Light, Solarized, Nord, Matrix, custom...), ---, Zoom In, Zoom Out
- **Help** — Keyboard Shortcuts, User Manual, ---, About

## Dialogs (Modal Overlays)

- Connection form (new/edit profile)
- Credential prompt (password entry)
- Vault password (on startup if vault enabled)
- Search (LDAP filter builder)
- Export/Import
- Schema viewer
- Certificate trust prompt

## Data Flow & Core Integration

```
loom-gui (Slint UI)
  +-- calls loom-core::LdapConnection for connect/bind/search/modify
  +-- calls loom-core::vault::Vault for password storage
  +-- calls loom-core::config::AppConfig for load/save
  +-- calls loom-core::export for LDIF/JSON/CSV/XLSX
  +-- calls loom-core::credentials for password resolution
```

- Slint runs its event loop on the main thread
- LDAP operations run on a tokio runtime in a background thread
- Results sent back via `slint::invoke_from_event_loop()`

## Theming

**Approach**: Option A — same theme name, separate definitions per renderer.

- `theme = "nord"` in config loads a ratatui Nord palette for TUI, a Slint Nord palette for GUI
- 5 built-in themes: dark, light, solarized, nord, matrix
- Custom themes are GUI-only via `[[themes]]` in config.toml
- TUI falls back to "dark" for unknown theme names

**Aesthetic**: Dark, minimal, terminal-inspired.

- Monospace font (Geist Mono or similar) for values, DNs, filters
- Proportional font for labels, menus, buttons
- Zinc/neutral tones, single accent color, subtle 1px borders, small radius (4px)
- Tree uses monospace with connector lines (like the TUI)
- Attribute table: alternating row shading, monospace values
- No heavy shadows, gradients, or animations

**Custom themes in config**:

```toml
[[themes]]
name = "my-custom"
bg_primary = "#0d1117"
bg_secondary = "#161b22"
fg_primary = "#c9d1d9"
accent = "#58a6ff"
border = "#30363d"
```

**Theme as Slint global**: All widgets bind to `Theme.*` properties. Switching themes swaps property values at runtime — no restart needed.

## Startup & Error Handling

The GUI fixes the TUI's startup issue (connecting before UI appears):

1. Launch -> Slint window appears immediately (empty state or profiles loaded)
2. If vault enabled -> in-window password dialog (not TTY prompt)
3. If auto-connect -> connection happens in background, status bar shows progress
4. Connection failure -> error in status bar, UI always responsive
5. No blank screen, no silent exit

## User Stories & Test Scenarios

### Story 1: First Launch (No Config)

**As a** new user, **I want** the app to open immediately with a welcome state **so that** I know the app is working and I can create my first profile.

**Scenario:**

1. User launches `loom-browser` with no `config.toml`
2. Window appears within 1 second
3. Empty state shows: "No connections. Use Profiles > New Profile to get started."
4. Profiles menu contains: New Profile, Edit Profiles (disabled), Import Profiles
5. Tab bar is empty, DIT tree and detail panel show placeholder text

**Tests:**

- `test_first_launch_shows_empty_state` — no config file -> window renders, status message visible
- `test_first_launch_profiles_menu` — New Profile enabled, Edit Profiles disabled
- `test_first_launch_no_tabs` — tab bar is empty

### Story 2: Connect to Profile

**As a** user with saved profiles, **I want** to pick a profile from the menu and connect **so that** I can browse the directory.

**Scenario:**

1. User clicks Profiles menu
2. Sees profiles grouped by folder
3. Clicks a profile
4. Status bar shows "Connecting to ldap.example.com..."
5. New tab appears with spinner
6. On success: DIT tree populates, status bar shows "Connected"
7. On failure: error in status bar, no crash

**Tests:**

- `test_profiles_menu_shows_saved_profiles` — config with 3 profiles -> all 3 appear
- `test_profiles_menu_grouped_by_folder` — profiles with folders appear under folder submenus
- `test_connect_shows_loading_state` — clicking profile -> status bar text + tab spinner
- `test_connect_success_populates_tree` — mock LDAP response -> tree has base DN node
- `test_connect_failure_shows_error` — connection error -> error in status bar
- `test_connect_timeout_shows_error` — unreachable host -> error after timeout, UI never freezes

### Story 3: Browse Directory Tree

**As a** connected user, **I want** to expand tree nodes and see child entries **so that** I can navigate the hierarchy.

**Scenario:**

1. User clicks expand arrow on base DN
2. Loading indicator while children fetch
3. Children appear
4. User clicks a leaf node -> detail panel shows attributes

**Tests:**

- `test_expand_node_fetches_children` — click expand -> LDAP search issued
- `test_expand_node_shows_loading` — during fetch, node shows loading state
- `test_expand_node_populates_children` — mock response -> child nodes appear
- `test_select_node_shows_attributes` — click leaf -> detail panel shows attribute table
- `test_expand_empty_node` — no children -> shows as leaf

### Story 4: View & Edit Attributes

**As a** connected user, **I want** to view and edit attributes inline **so that** I can modify entries.

**Scenario:**

1. User selects entry in tree
2. Detail panel shows attribute table
3. Double-click value -> editable
4. Enter submits, Escape cancels
5. Read-only profile blocks editing

**Tests:**

- `test_attribute_table_shows_all_attributes` — entry with 5 attrs -> 5 rows
- `test_attribute_table_monospace_values` — values render in monospace
- `test_double_click_enables_edit` — double-click -> editable state
- `test_edit_submit_sends_modify` — Enter -> LDAP modify issued
- `test_edit_cancel_on_escape` — Escape -> reverts to original
- `test_read_only_blocks_edit` — read-only profile -> no edit
- `test_modify_failure_shows_error` — LDAP error -> error message, value reverts

### Story 5: Create New Profile

**As a** user, **I want** to create a new connection profile **so that** I can connect to a new server.

**Scenario:**

1. Profiles > New Profile -> dialog opens
2. Form: Name, Host, Port, TLS Mode, Bind DN, Base DN, Credential Method, Folder
3. Save -> profile in menu, config updated
4. "Connect now?" offered

**Tests:**

- `test_new_profile_dialog_opens` — menu click -> dialog visible
- `test_new_profile_requires_name_and_host` — empty name -> validation error
- `test_new_profile_saves_to_config` — fill + save -> config file updated
- `test_new_profile_appears_in_menu` — after save -> listed in Profiles menu
- `test_new_profile_default_values` — port=389, tls_mode=auto, credential_method=prompt
- `test_new_profile_cancel` — Cancel -> no changes

### Story 6: Search / LDAP Filter

**As a** connected user, **I want** to search with an LDAP filter **so that** I can find specific entries.

**Scenario:**

1. Ctrl+F -> search dialog with pre-filled base DN
2. Enter filter and scope, click Search
3. Results displayed
4. Click result -> detail panel shows attributes
5. Escape returns to tree

**Tests:**

- `test_search_dialog_opens` — Ctrl+F -> dialog visible
- `test_search_executes_filter` — submit -> LDAP search issued
- `test_search_results_displayed` — mock results -> entries listed
- `test_search_result_click_shows_detail` — click -> attributes shown
- `test_search_invalid_filter` — malformed filter -> validation error
- `test_search_no_results` — empty set -> "No entries found"
- `test_search_escape_returns_to_tree` — Escape -> tree restored

### Story 7: Multi-Tab Connections

**As a** user, **I want** multiple connections in tabs **so that** I can compare directories.

**Scenario:**

1. Connect to profile A -> tab 1
2. Connect to profile B -> tab 2
3. Click tabs to switch
4. Ctrl+W closes active tab
5. Last tab closed -> empty state

**Tests:**

- `test_second_connection_opens_new_tab` — connect twice -> 2 tabs
- `test_tab_switch_changes_tree` — click tab 2 -> tab 2's directory shown
- `test_close_tab` — Ctrl+W -> tab removed
- `test_close_last_tab_shows_empty_state` — close only tab -> empty state
- `test_tab_shows_profile_name` — tab label matches profile name

### Story 8: Vault Password on Startup

**As a** user with vault enabled, **I want** to enter my vault password in the GUI **so that** I don't need a TTY prompt.

**Scenario:**

1. Launch with vault enabled + vault.dat exists
2. Window appears immediately
3. Modal dialog: password field + Unlock/Skip
4. Correct password -> vault loaded
5. Wrong password -> error, retry
6. Skip -> continues without vault

**Tests:**

- `test_vault_prompt_shown_on_startup` — vault enabled + file exists -> dialog
- `test_vault_correct_password_unlocks` — correct password -> vault loaded
- `test_vault_wrong_password_shows_error` — wrong password -> error, dialog stays
- `test_vault_skip_continues_without` — Skip -> no vault, app works
- `test_vault_not_shown_when_disabled` — vault_enabled=false -> no dialog

### Story 9: Export Entries

**As a** connected user, **I want** to export entries **so that** I can save data locally.

**Scenario:**

1. Right-click tree node -> "Export subtree..."
2. Dialog: format (LDIF/JSON/CSV/XLSX), file path, scope
3. Export -> status bar shows progress then confirmation

**Tests:**

- `test_export_dialog_opens` — right-click export -> dialog visible
- `test_export_format_options` — dropdown has LDIF, JSON, CSV, XLSX
- `test_export_writes_file` — mock entries + export -> file written
- `test_export_progress_shown` — during export -> status bar progress
- `test_export_error_shown` — write failure -> error message

### Story 10: Theme Switching

**As a** user, **I want** to switch themes without restarting.

**Scenario:**

1. View > Theme > Nord
2. Colors update immediately
3. Config file updated

**Tests:**

- `test_theme_menu_lists_all_themes` — 5 built-in + custom themes
- `test_theme_switch_updates_colors` — select nord -> colors change
- `test_theme_switch_persists` — after switch -> config updated
- `test_custom_theme_loaded` — `[[themes]]` in config -> appears in menu

## Summary of Decisions

| Decision | Choice |
|----------|--------|
| Framework | Slint |
| Binary names | `loom` (TUI), `loom-browser` (GUI) |
| Workspace | 2 new crates: `loom-gui`, `loom-browser` |
| Config sharing | Same `config.toml` + `vault.dat` for both |
| Config refactor | Move config types from `loom-tui` to `loom-core` |
| Layout | Menu bar + tabs + tree/detail split |
| Profiles | Dropdown in menu bar, not sidebar |
| Async model | Slint main thread + tokio background thread |
| Theming | Same name, separate definitions per renderer |
| Custom themes | GUI-only via `[[themes]]` in config |
| Startup | Window immediately, connections in background |
| Vault prompt | In-window dialog, not TTY prompt |
| Test stories | 10 stories, 58 tests |
