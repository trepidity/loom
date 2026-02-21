# loom-ldapbrowser User Manual

## Table of Contents

- [Overview](#overview)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Layouts](#layouts)
- [Browser Layout](#browser-layout)
- [Profiles Layout](#profiles-layout)
- [Connecting](#connecting)
- [Browsing the Directory](#browsing-the-directory)
- [Searching](#searching)
- [Editing Entries](#editing-entries)
- [Creating and Deleting Entries](#creating-and-deleting-entries)
- [Bulk Update](#bulk-update)
- [Export and Import](#export-and-import)
- [Schema Viewer](#schema-viewer)
- [Connection Profiles](#connection-profiles)
- [Configuration](#configuration)
- [Keybindings](#keybindings)
- [Themes](#themes)
- [Credentials](#credentials)
- [TLS Modes](#tls-modes)
- [Offline Mode](#offline-mode)
- [Context Menus](#context-menus)
- [Log Panel](#log-panel)
- [Command-Line Options](#command-line-options)

---

## Overview

loom-ldapbrowser is a terminal-based LDAP browser built with Rust. It provides a full-featured TUI for browsing, searching, editing, and managing LDAP directories. It supports multiple simultaneous connections, vim-style navigation, and works with OpenLDAP, Active Directory, and other LDAP servers.

## Installation

### From source

Requires Rust 1.80 or later.

```bash
git clone https://github.com/trepidity/loom-ldapbrowser.git
cd loom-ldapbrowser
cargo install --path crates/loom-ldapbrowser
```

### Prebuilt binaries

Download from [GitHub Releases](https://github.com/trepidity/loom-ldapbrowser/releases):

| Platform | Architecture |
|----------|-------------|
| Linux | x86_64, aarch64 |
| macOS | Intel (x86_64), Apple Silicon (aarch64) |
| Windows | x86_64 |

## Quick Start

```bash
# Connect with CLI arguments
loom-ldapbrowser -H ldap.example.com -D "cn=admin,dc=example,dc=com" -b "dc=example,dc=com"

# Use saved profiles from config
loom-ldapbrowser
```

On first launch with no configuration, press `Ctrl+T` to open the connection dialog. Press `F5` or `?` at any time for the built-in help overlay.

---

## Layouts

loom-ldapbrowser has two layouts, toggled with `F1` and `F2`:

- **Browser** (`F1`) -- The main working view with the directory tree, detail panel, and command bar.
- **Profiles** (`F2`) -- Manage saved connection profiles organized into folders.

---

## Browser Layout

The browser layout consists of four panels:

### Tab Bar

Displays open connection tabs. Switch between tabs or open new ones. Each tab represents an independent LDAP connection.

### Tree Panel

Displays the directory hierarchy starting from the base DN. Nodes expand lazily as you navigate. Vim-style keys (`h/j/k/l`) or arrow keys move through the tree.

### Detail Panel

Shows all attributes of the currently selected entry. Navigate attributes with `j/k` or arrows. Edit, add, or delete attribute values from here.

### Command Panel

A search/filter input bar at the bottom. Type an LDAP filter (e.g., `(objectClass=person)`) and press `Enter` to search. Results appear in a popup overlay.

### Status Bar

Shows the current connection info, detected server type, and key hints.

---

## Profiles Layout

The profiles layout has two panels:

### Profiles Tree

A tree view of all saved connection profiles, organized by folder. Navigate, connect, edit, create, or delete profiles from here.

### Profile Detail / Folder Detail

When a profile is selected, shows all connection fields with options to edit, connect, or delete. When a folder is selected, shows the folder name and description with an option to edit the description.

---

## Connecting

There are several ways to connect:

1. **CLI arguments** -- Pass `-H`, `-D`, and `-b` flags to connect on startup.
2. **Connection dialog** (`Ctrl+T`) -- Select from saved profiles or create a new connection.
3. **Profiles layout** (`F2`) -- Browse saved profiles, press `c` to connect.
4. **Config file** -- The first profile in `config.toml` connects automatically on startup.

When a profile uses `credential_method = "prompt"`, loom-ldapbrowser will prompt for the bind password. You can also set the `LOOM_PASSWORD` environment variable to skip the prompt.

---

## Browsing the Directory

After connecting, the tree panel shows the directory starting from the base DN.

- Expand a node to load its children from the server.
- Select an entry to view its attributes in the detail panel.
- The tree loads children lazily -- only fetched when a node is expanded.

---

## Searching

Press `F9` or `/` to focus the search input. Type an LDAP filter and press `Enter`.

Examples:
- `(objectClass=person)` -- all person entries
- `(cn=Alice*)` -- entries with cn starting with "Alice"
- `(&(objectClass=inetOrgPerson)(mail=*@example.com))` -- compound filter

Results appear in a popup. Press `Enter` on a result to navigate to that entry in the tree.

---

## Editing Entries

From the detail panel:

- **Edit a value** -- Press `e` or `Enter` on an attribute to open the editor.
- **Add an attribute** -- Press `a` to pick from available attributes (filtered by schema).
- **Add a value** -- Press `+` to add another value to a multi-valued attribute.
- **Delete a value** -- Press `d` or `Delete` to remove an attribute value (with confirmation).

### DN Search Mode

When editing a DN-valued attribute (like `member` or `manager`), the editor provides live DN search. Type a name to search, use `Space` to toggle selections, and `Enter` to add the selected DNs.

---

## Creating and Deleting Entries

### Create

Press `a` in the tree panel or `n` in the detail panel to create a new child entry under the selected node. Fill in:

- **RDN** -- e.g., `cn=NewUser`
- **Object classes** -- comma-separated, e.g., `inetOrgPerson,posixAccount`
- **Extra attributes** -- comma-separated `attr=value` pairs

### Delete

Press `d` or `Delete` on an entry. A confirmation dialog appears before deletion.

---

## Bulk Update

Press `F8` to open the bulk update dialog. This applies a modification to all entries matching a filter.

- **Operation** -- Replace, Add, or Delete (cycle with `F2`)
- **Filter** -- LDAP search filter to match entries
- **Attribute** -- Attribute name to modify
- **Value** -- Value to use

Press `Enter` to execute. Results are reported in the status bar.

---

## Export and Import

### Export

Press `F4` to open the export dialog. Configure:

- **Search filter** -- Which entries to export
- **Attributes** -- Comma-separated list, or `*` for all
- **Format** -- LDIF, JSON, CSV, or XLSX (cycle with `F2`)
- **Filename** -- Output file path

The format is auto-detected from the file extension.

### Import

Import files through the profiles layout or programmatically. Supported formats:

| Format | Extensions | Notes |
|--------|-----------|-------|
| LDIF | `.ldif`, `.ldf` | RFC 2849 compliant |
| JSON | `.json` | Array of entry objects |
| CSV | `.csv` | One row per entry, multi-values joined |
| Excel | `.xlsx`, `.xls` | Spreadsheet with header row |

---

## Schema Viewer

Press `F6` to open the schema viewer. It has two tabs:

### Object Classes

Shows all object classes with their kind (structural, abstract, auxiliary), superior class, and MUST/MAY attributes. Inheritance is resolved -- you see the full attribute set.

### Attribute Types

Shows all attribute types with their syntax, single-value flag, and description.

Navigate with `j/k`, switch tabs with `Tab`, filter with `/`, and close with `q` or `Esc`.

---

## Connection Profiles

Profiles are saved in `~/.config/loom-ldapbrowser/config.toml` under `[[connections]]` blocks.

### Organizing with Folders

Set the `folder` field on a profile to group it:

```toml
[[connections]]
name = "Production"
host = "ldap.prod.example.com"
folder = "Production"

[[connections]]
name = "Staging"
host = "ldap.staging.example.com"
folder = "Production/Staging"
```

Folders appear as expandable nodes in the profiles tree. You can add descriptions to folders:

```toml
[[folders]]
path = "Production"
description = "Production LDAP servers -- handle with care"
```

### Export and Import Profiles

From the profiles layout, press `x` to export selected profiles to a file, or `i` to import profiles from a file. Exported files use the same `[[connections]]` TOML format.

---

## Configuration

loom-ldapbrowser reads `~/.config/loom-ldapbrowser/config.toml`.

### Full Example

```toml
[general]
theme = "dark"               # dark | light | solarized | nord | matrix
tick_rate_ms = 250
log_level = "info"

[keybindings]
quit = "Ctrl+q"
force_quit = "Ctrl+c"
focus_next = "Tab"
focus_prev = "Shift+Tab"
show_connect_dialog = "Ctrl+t"
search = "F9"
show_export_dialog = "F4"
show_bulk_update = "F8"
show_schema_viewer = "F6"
show_help = "F5"
toggle_log_panel = "F7"
save_connection = "F10"
switch_to_browser = "F1"
switch_to_profiles = "F2"

[[connections]]
name = "Production"
host = "ldap.example.com"
port = 389
tls_mode = "auto"            # auto | ldaps | starttls | none
bind_dn = "cn=admin,dc=example,dc=com"
base_dn = "dc=example,dc=com"
credential_method = "prompt"  # prompt | command | keychain
page_size = 500
timeout_secs = 30
relax_rules = false
read_only = false
folder = "Production"

[[connections]]
name = "Staging"
host = "ldap-staging.internal"
port = 636
tls_mode = "ldaps"
bind_dn = "cn=readonly,dc=staging,dc=com"
base_dn = "dc=staging,dc=com"
credential_method = "keychain"
folder = "Production/Staging"

[[folders]]
path = "Production"
description = "Production LDAP servers"
```

### Connection Profile Fields

| Field | Default | Description |
|-------|---------|-------------|
| `name` | *required* | Display name for the profile |
| `host` | *required* | LDAP server hostname |
| `port` | `389` | LDAP port |
| `tls_mode` | `auto` | TLS mode (see below) |
| `bind_dn` | | DN to bind as |
| `base_dn` | | Base DN for browsing and search |
| `credential_method` | `prompt` | How to obtain the password |
| `password_command` | | Shell command for `command` method |
| `page_size` | `500` | LDAP paged results size |
| `timeout_secs` | `30` | Connection timeout in seconds |
| `relax_rules` | `false` | Relax LDAP protocol rules |
| `read_only` | `false` | Prevent modifications |
| `folder` | | Folder path for organization |
| `offline` | `false` | Use offline demo directory |

---

## Keybindings

All global keybindings are configurable via the `[keybindings]` section. Override only the keys you want to change; everything else keeps its default.

### Global (configurable)

| Default Key | Action |
|-------------|--------|
| `F1` | Browser layout |
| `F2` | Profiles layout |
| `F3` | About |
| `Ctrl+T` | Connection dialog |
| `F4` | Export dialog |
| `F5` / `?` | Help |
| `F6` | Schema viewer |
| `F7` | Toggle log panel |
| `F8` | Bulk update |
| `F9` | Focus search input |
| `F10` | Save connection |
| `Tab` | Focus next panel |
| `Shift+Tab` | Focus previous panel |
| `Ctrl+Q` | Quit |
| `Ctrl+C` | Force quit |

### Tree Panel

| Key | Action |
|-----|--------|
| `j` / `k` / arrows | Navigate up/down |
| `l` / `Right` / `Enter` | Expand or select node |
| `h` / `Left` | Collapse node |
| `a` | Create child entry |
| `d` / `Delete` | Delete entry |
| `Space` | Context menu |

### Detail Panel

| Key | Action |
|-----|--------|
| `j` / `k` / arrows | Navigate attributes |
| `e` / `Enter` | Edit attribute value |
| `a` | Add new attribute |
| `+` | Add value to multi-valued attribute |
| `d` / `Delete` | Delete attribute value |
| `n` | Create child entry |
| `x` | Delete entry |
| `r` | Refresh entry |
| `Space` | Context menu |

### Profiles Tree

| Key | Action |
|-----|--------|
| `j` / `k` / arrows | Navigate |
| `l` / `Right` | Expand folder or view |
| `h` / `Left` | Collapse folder |
| `e` | Edit or view profile |
| `c` | Connect to profile |
| `n` | New profile |
| `d` / `Delete` | Delete profile |
| `x` | Export profiles |
| `i` | Import profiles |

### Connection Form

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next / previous field |
| `e` | Enter edit mode (view mode) |
| `c` | Connect (view mode) |
| `F2` | Cycle TLS mode (edit mode) |
| `F3` | Cycle credential method (edit mode) |
| `F10` / `Ctrl+Enter` | Save profile (edit mode) |
| `Esc` | Cancel editing |

### Search Results

| Key | Action |
|-----|--------|
| `j` / `k` / arrows | Navigate results |
| `Enter` | Go to selected entry |
| `Esc` / `q` | Close |

### Export Dialog

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next / previous field |
| `F2` | Cycle export format |
| `Enter` | Execute export |
| `Esc` | Cancel |

### Bulk Update Dialog

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next / previous field |
| `F2` | Cycle operation type |
| `Enter` | Execute |
| `Esc` | Cancel |

### Schema Viewer

| Key | Action |
|-----|--------|
| `j` / `k` / arrows | Scroll |
| `Tab` | Switch Object Classes / Attribute Types |
| `/` | Filter by name |
| `Esc` / `q` | Close |

### Log Panel

| Key | Action |
|-----|--------|
| `j` / `k` / arrows | Scroll |
| `g` / `Home` | Jump to top |
| `G` / `End` | Jump to bottom |
| `Esc` / `q` | Close |

### Confirm Dialog

| Key | Action |
|-----|--------|
| `y` | Yes |
| `n` / `Esc` | No |
| `h` / `l` / arrows | Select Yes / No |
| `Enter` | Execute selection |

---

## Themes

loom-ldapbrowser includes five built-in themes. Set the theme in `config.toml`:

```toml
[general]
theme = "dark"
```

| Theme | Description |
|-------|-------------|
| `dark` | Catppuccin Mocha palette (default) |
| `light` | Light backgrounds with blue accents |
| `solarized` | Classic Solarized dark |
| `nord` | Nordic color scheme |
| `matrix` | Green on black |

### Custom Themes

Place custom theme files in `~/.config/loom-ldapbrowser/themes/`. A theme TOML file defines colors for borders, text, selections, headers, and other elements using hex colors (`#RRGGBB`) or named colors.

---

## Credentials

| Method | Description |
|--------|-------------|
| `prompt` | Interactive password prompt in the TUI. Also reads the `LOOM_PASSWORD` environment variable if set. |
| `command` | Executes `password_command` and reads stdout. Works with `pass`, `op`, `gpg`, `security`, and any command that prints a password. |
| `keychain` | Uses the OS keychain: macOS Keychain, Linux Secret Service (GNOME Keyring), or Windows Credential Manager. |

### Command examples

```toml
# pass (password store)
credential_method = "command"
password_command = "pass show ldap/prod"

# 1Password CLI
credential_method = "command"
password_command = "op read 'op://Vault/LDAP/password'"

# GPG-encrypted file
credential_method = "command"
password_command = "gpg --quiet --decrypt ~/.ldap-password.gpg"
```

---

## TLS Modes

| Mode | Behavior |
|------|----------|
| `auto` | Try LDAPS on port 636, fall back to StartTLS, then plaintext |
| `ldaps` | LDAPS (TLS on connect) on port 636 |
| `starttls` | StartTLS upgrade on port 389 |
| `none` | Plaintext, no encryption |

---

## Offline Mode

Set `offline = true` on a connection profile to use an in-memory demo directory without connecting to a server. This is useful for testing loom-ldapbrowser's features or demonstrating the UI.

```toml
[[connections]]
name = "Demo"
host = "localhost"
offline = true
```

---

## Context Menus

Press `Space` on a tree node or detail attribute to open a context menu with relevant actions (edit, copy, create, delete, etc.). Mouse right-click also works.

---

## Log Panel

Press `F7` to toggle the log panel. It shows a scrollable history of log messages including connection events, LDAP operations, errors, and search results.

---

## Command-Line Options

```
loom-ldapbrowser [OPTIONS]

Options:
  -c, --config <PATH>     Path to config file (default: ~/.config/loom-ldapbrowser/config.toml)
  -H, --host <HOST>       LDAP host to connect to (overrides config)
  -p, --port <PORT>       LDAP port (overrides config)
  -D, --bind-dn <DN>      Bind DN (overrides config)
  -b, --base-dn <DN>      Base DN (overrides config)
  -h, --help              Print help
  -V, --version           Print version
```

CLI arguments override the first connection profile in the config file. If `-H` is specified, loom-ldapbrowser connects to that host on startup.

---

## Architecture

loom-ldapbrowser is organized as a Cargo workspace:

```
crates/
  loom-ldapbrowser/  Binary -- CLI parsing and entry point
  loom-core/         Library -- LDAP operations, export/import, schema, DN utilities
  loom-tui/          Library -- TUI framework, components, themes, keybindings
```

All state changes flow through an `Action` enum dispatched via an async channel. LDAP operations run in background Tokio tasks, keeping the UI responsive.
