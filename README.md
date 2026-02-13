# Loom

A terminal-based LDAP browser built with Rust.

Browse, search, edit, and manage LDAP directories from the comfort of your terminal. Loom provides a full-featured TUI with vim-style navigation, multi-tab connections, and bulk operations.

## Features

- **Tree browser** -- Navigate the directory hierarchy with vim keybindings or arrow keys
- **Attribute viewer** -- Inspect and edit entry attributes in a detail panel
- **Multi-tab** -- Open multiple connections side by side
- **Search** -- LDAP filter search with attribute selection
- **Create & delete entries** -- Add children or remove entries directly from the tree
- **Bulk update** -- Apply modifications across entries matching a filter
- **Export / Import** -- LDIF, JSON, CSV, and XLSX formats
- **Schema viewer** -- Browse object classes and attribute type definitions
- **Themes** -- Dark (Catppuccin Mocha), light, solarized, nord, and matrix built-in themes
- **Credentials** -- Interactive prompt, shell command (`pass`, `op`, etc.), or OS keychain
- **Server detection** -- Identifies OpenLDAP, Active Directory, and other vendors
- **TLS** -- Auto-negotiation, LDAPS, StartTLS, or plaintext
- **Mouse support** -- Click to focus panels
- **Reconnect** -- Automatic reconnection with cached credentials

## Screenshots

### Main Browsing View

Tab bar, tree panel, detail panel with attributes, command panel, and status bar.

```
 [localhost:389]  dev-ldap  [+]
┌ Tree ──────────────┐┌ Details ─────────────────────────────────────────────┐
│▼ dc=example,dc=com ││ DN: cn=Alice Smith,ou=People,dc=example,dc=com      │
│  ▼ ou=People       ││                                                     │
│    ► cn=Alice Smith ││ Attribute          │ Value                          │
│      cn=Bob Jones  ││────────────────────┼────────────────────────────────│
│  ▼ ou=Groups       ││ cn                 │ Alice Smith                    │
│      cn=admins     ││ sn                 │ Smith                          │
│      cn=developers ││ givenName          │ Alice                          │
│  ► ou=Services     ││ mail               │ alice@example.com              │
│                    ││ uid                │ asmith                         │
│                    ││ uidNumber          │ 1001                           │
│                    ││ gidNumber          │ 500                            │
│                    ││ objectClass        │ inetOrgPerson                  │
│                    ││                    │ posixAccount                   │
│                    ││ memberOf           │ cn=admins,ou=Groups,dc=exam... │
│                    ││                    │ cn=developers,ou=Groups,dc=... │
│                    │├──────────────────────────────────────────────────────┤
│                    │┊ Command                                             ┊
│                    │┊ Loaded 8 entries                                    ┊
│                    │┊ / (objectClass=*)                                   ┊
└────────────────────┘└──────────────────────────────────────────────────────┘
 Connected: localhost:389 | OpenLDAP | q:quit Tab:focus /:search C-t:connect
```

### Search Results

Search results dialog overlaid on the main view, showing matching entries.

```
┌ Search: (objectClass=person) (4 results) ────────────────────────────┐
│  j/k: navigate  Enter: select  Esc: close                           │
│                                                                      │
│► cn=Alice Smith,ou=People,dc=example,dc=com  [inetOrgPerson]        │
│  cn=Bob Jones,ou=People,dc=example,dc=com    [inetOrgPerson]        │
│  cn=Carol Lee,ou=People,dc=example,dc=com    [inetOrgPerson]        │
│  cn=Dave Patel,ou=People,dc=example,dc=com   [inetOrgPerson]        │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

### Attribute Editor (Plain)

Simple text editing popup for a non-DN attribute.

```
┌ Edit: mail ──────────────────────────────────────────────┐
│ DN: cn=Alice Smith,ou=People,dc=example,dc=com           │
│                                                          │
│ alice.smith@example.com_                                 │
│                                                          │
│ Enter: save  Esc: cancel  Ctrl+Space: DN search          │
└──────────────────────────────────────────────────────────┘
```

### Connection Dialog

Popup listing saved connection profiles.

```
┌ Connect ─────────────────────────────────────────┐
│  j/k: navigate  Enter: connect  Esc: cancel     │
│                                                  │
│  + New Connection...                             │
│► Production LDAP    ldap.example.com:389         │
│  Staging            ldap-staging.internal:636    │
│                                                  │
└──────────────────────────────────────────────────┘
```

### New Connection Form

Form for entering connection details.

```
┌ New Connection ──────────────────────────────────────┐
│ TLS Mode: Auto  (F2 to cycle)                       │
│                                                      │
│ Name:                                                │
│ Production LDAP_                                     │
│ Host:                                                │
│ ldap.example.com_                                    │
│ Port:                                                │
│ 389_                                                 │
│ Bind DN:                                             │
│ cn=admin,dc=example,dc=com_                          │
│ Base DN:                                             │
│ dc=example,dc=com_                                   │
│ Password:                                            │
│ ********_                                            │
│                                                      │
│ Tab:next  Shift+Tab:prev  F2:TLS  Enter:connect      │
└──────────────────────────────────────────────────────┘
```

### Credential Prompt

Popup requesting bind credentials for reconnection.

```
┌ Credentials ─────────────────────────────────────┐
│ Server: ldap.example.com:389                     │
│                                                  │
│ Bind DN:                                         │
│ cn=admin,dc=example,dc=com_                      │
│ Password:                                        │
│ ********_                                        │
│                                                  │
│ Tab:switch field  Enter:connect  Esc:cancel      │
└──────────────────────────────────────────────────┘
```

### Attribute Editor (DN Search)

DN search mode with live results and multi-select checkboxes.

```
┌ Edit: member ────────────────────────────────────────────────────┐
│ DN: cn=admins,ou=Groups,dc=example,dc=com                       │
│                                                                  │
│ alice_                                          Searching...     │
│                                                                  │
│ [x] Alice Smith  (cn=Alice Smith,ou=People,dc=example,dc=com)   │
│ [ ] Alice Wang   (cn=Alice Wang,ou=People,dc=example,dc=com)    │
│                                                                  │
│ Space: toggle  Enter: add 1  Tab: results  Esc: cancel           │
└──────────────────────────────────────────────────────────────────┘
```

### Create Entry Dialog

Form for creating a new LDAP entry under the selected node.

```
┌ Create Entry ────────────────────────────────────────┐
│ Parent DN:                                           │
│ ou=People,dc=example,dc=com                          │
│ RDN (e.g. cn=NewUser):                               │
│ cn=JohnDoe_                                          │
│ Object Classes (comma-separated):                    │
│ inetOrgPerson,posixAccount_                          │
│ Extra Attributes (attr=val, ...):                    │
│ sn=Doe, givenName=John_                              │
│                                                      │
│ Full DN:                                             │
│ cn=JohnDoe,ou=People,dc=example,dc=com               │
│                                                      │
│ Tab:next field  Enter:create  Esc:cancel             │
└──────────────────────────────────────────────────────┘
```

### Delete Confirmation

Small confirmation dialog before deleting an entry.

```
┌ Confirm ─────────────────────────────────────┐
│                                              │
│ Delete entry?                                │
│ cn=Bob Jones,ou=People,dc=example,dc=com     │
│                                              │
│           [Y]es      [N]o                    │
└──────────────────────────────────────────────┘
```

### Export Dialog

Form for exporting entries with filter, format, and filename.

```
┌ Export ──────────────────────────────────────────┐
│ Search Filter:                                  │
│ (objectClass=person)_                           │
│ Attributes:                                     │
│ cn,mail,uid_                                    │
│ Format:                                         │
│ > LDIF (.ldif)                                  │
│   JSON (.json)                                  │
│   CSV (.csv)                                    │
│   Excel (.xlsx)                                 │
│ Filename:                                       │
│ export.ldif_                                    │
│                                                 │
│ Tab:next  j/k:select  F2:cycle  Enter:export    │
└─────────────────────────────────────────────────┘
```

### Bulk Update Dialog

Form for applying bulk modifications to matching entries.

```
┌ Bulk Update ────────────────────────────────────┐
│ Operation: Replace  (F2 to cycle)               │
│                                                 │
│ Filter:                                         │
│ (objectClass=person)_                           │
│ Attribute:                                      │
│ description_                                    │
│ Value:                                          │
│ Updated by bulk operation_                      │
│                                                 │
│ Tab:next  F2:operation  Enter:execute  Esc:cancel│
└─────────────────────────────────────────────────┘
```

### Schema Viewer (Object Classes)

Browse object class definitions with MUST/MAY attributes.

```
┌ Schema ──────────────────────────────────────────────────────────────────┐
│ [Object Classes]  Attribute Types                                       │
│                                                                         │
│ Name              │ Kind        │ Superior │ MUST          │ MAY        │
│───────────────────┼─────────────┼──────────┼───────────────┼────────────│
│ top               │ ABSTRACT    │          │ objectClass   │            │
│ person            │ STRUCTURAL  │ top      │ cn, sn        │ description│
│ organizationalPer │ STRUCTURAL  │ person   │               │ title, ou  │
│ inetOrgPerson     │ STRUCTURAL  │ organiz..│               │ mail, uid  │
│ groupOfNames      │ STRUCTURAL  │ top      │ cn, member    │ o, ou, desc│
│ organizationalUnit│ STRUCTURAL  │ top      │ ou            │ description│
│ posixAccount      │ AUXILIARY   │ top      │ cn, uid, uid..│ gecos, home│
│                                                                         │
│ Tab:switch  j/k:navigate  /:filter  q:close                            │
└─────────────────────────────────────────────────────────────────────────┘
```

### Schema Viewer (Attribute Types)

Browse attribute type definitions with syntax and constraints.

```
┌ Schema ──────────────────────────────────────────────────────────────────┐
│  Object Classes  [Attribute Types]                                      │
│                                                                         │
│ Name              │ Syntax          │ Single? │ Description              │
│───────────────────┼─────────────────┼─────────┼──────────────────────────│
│ cn                │ Directory String│ No      │ Common name              │
│ sn                │ Directory String│ No      │ Surname                  │
│ mail              │ IA5 String      │ No      │ Email address            │
│ uid               │ Directory String│ Yes     │ User ID                  │
│ uidNumber         │ INTEGER         │ Yes     │ Numeric user ID          │
│ member            │ DN              │ No      │ Group member DN          │
│ objectClass       │ OID             │ No      │ Object class membership  │
│                                                                         │
│ Tab:switch  j/k:navigate  /:filter  q:close                            │
└─────────────────────────────────────────────────────────────────────────┘
```

### Log Panel

Overlay showing recent log messages.

```
┌ Logs (6) ────────────────────────────────────────────────────────┐
│                                                                  │
│ [INFO] Connected to ldap.example.com:389                         │
│ [INFO] Detected server type: OpenLDAP                            │
│ [INFO] Loaded base DN: dc=example,dc=com                         │
│ [INFO] Expanded ou=People,dc=example,dc=com (4 children)        │
│ [INFO] Search: (objectClass=person) returned 4 results           │
│ [ERR]  Modify failed: insufficient access rights                 │
│                                                                  │
│ j/k:scroll  g/G:top/bottom  q:close                             │
└──────────────────────────────────────────────────────────────────┘
```

## Installation

### From source

Requires Rust 1.80 or later.

```bash
cargo install --path crates/loom
```

### From release binaries

Download a prebuilt binary from [GitHub Releases](https://github.com/your-org/loom/releases) for:

| Platform | Architecture |
|----------|-------------|
| Linux | x86_64, aarch64 |
| macOS | x86_64 (Intel), aarch64 (Apple Silicon) |
| Windows | x86_64 |

### Build from source

```bash
git clone https://github.com/your-org/loom.git
cd loom
cargo build --release
# Binary at target/release/loom
```

## Quick Start

```bash
# Connect with CLI arguments
loom -H ldap.example.com -D "cn=admin,dc=example,dc=com" -b "dc=example,dc=com"

# Or configure a profile and just run:
loom
```

On first launch with no config, Loom shows the connection dialog (`Ctrl+T`).

## Configuration

Loom reads `~/.config/loom/config.toml`. Create it manually or press `Ctrl+T` to connect ad-hoc, then `Ctrl+W` to save the connection.

```toml
[general]
theme = "dark"               # dark | light | solarized | nord | matrix
tick_rate_ms = 250
log_level = "info"

[keybindings]
# Override just what you want; everything else keeps Ctrl defaults
# quit = "Alt+q"
# show_connect_dialog = "Alt+t"
# toggle_layout = "F1"

[[connections]]
name = "Production"
host = "ldap.example.com"
port = 389
tls_mode = "auto"
bind_dn = "cn=admin,dc=example,dc=com"
base_dn = "dc=example,dc=com"
credential_method = "prompt"    # prompt | command | keychain
# password_command = "pass show ldap/prod"  # for credential_method = "command"
page_size = 500
timeout_secs = 30
relax_rules = false

[[connections]]
name = "Staging"
host = "ldap-staging.internal"
port = 636
tls_mode = "ldaps"
bind_dn = "cn=readonly,dc=staging,dc=com"
base_dn = "dc=staging,dc=com"
credential_method = "keychain"
```

### Credential methods

| Method | Description |
|--------|------------|
| `prompt` | Password prompt in the TUI. Also reads `LOOM_PASSWORD` env var if set. |
| `command` | Runs `password_command` and reads stdout. Works with `pass`, `op`, `gpg`, etc. |
| `keychain` | Uses the OS keychain (macOS Keychain, Linux Secret Service, Windows Credential Manager) via the `keyring` crate. |

### TLS modes

| Mode | Behavior |
|------|----------|
| `auto` | Try LDAPS (636), fall back to StartTLS, then plaintext |
| `ldaps` | LDAPS on port 636 |
| `starttls` | StartTLS upgrade on port 389 |
| `none` | Plaintext (no encryption) |

## Keybindings

All global keybindings are customizable via the `[keybindings]` section in `config.toml`.

### Global

| Key | Action |
|-----|--------|
| `q` | Quit |
| `Ctrl+C` | Force quit |
| `Tab` | Focus next panel |
| `Shift+Tab` | Focus previous panel |
| `/` | Focus search input |
| `Ctrl+T` | Open connection dialog |
| `Ctrl+N` | Next tab |
| `Ctrl+P` | Previous tab |
| `Ctrl+E` | Export dialog |
| `Ctrl+B` | Bulk update dialog |
| `Ctrl+S` | Schema viewer |
| `Ctrl+L` | Toggle log panel |
| `Ctrl+W` | Save ad-hoc connection to config |
| `Ctrl+G` | Toggle layout (Browser / Connections) |

### Tree panel

| Key | Action |
|-----|--------|
| `j` / `Up` | Move up |
| `k` / `Down` | Move down |
| `l` / `Right` / `Enter` | Expand / select entry |
| `h` / `Left` | Collapse |
| `a` | Create new entry (under selected node) |
| `d` / `Delete` | Delete selected entry (with confirmation) |

### Detail panel

| Key | Action |
|-----|--------|
| `r` | Refresh entry |

### Dialogs

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Cycle fields |
| `Enter` | Submit |
| `Esc` | Cancel / close |
| `y` / `n` | Confirm / deny (confirmation dialogs) |

## Export & Import

Loom supports four formats, auto-detected by file extension:

| Format | Extensions | Notes |
|--------|-----------|-------|
| LDIF | `.ldif`, `.ldf` | RFC 2849 compliant |
| JSON | `.json` | Array of entry objects |
| CSV | `.csv` | One row per entry, multivalues joined by separator |
| Excel | `.xlsx`, `.xls` | Spreadsheet with header row |

Open the export dialog with `Ctrl+E`, set a search filter and attributes, choose format and filename, then press Enter.

## Architecture

Loom is organized as a Cargo workspace with three crates:

```
crates/
  loom/          Binary -- CLI parsing + entry point
  loom-core/     Library -- LDAP operations, export/import, schema, DN utilities
  loom-tui/      Library -- TUI framework, components, themes, keybindings
```

All state changes flow through an `Action` enum dispatched via an async mpsc channel. LDAP operations run in background Tokio tasks and send results back as actions, keeping the UI responsive.

```
User Input --> KeyEvent --> Action --> process_action() --> spawn async task
                                                               |
                                                               v
                                                         LDAP operation
                                                               |
                                                               v
                                                     Action (result) --> UI update
```

## Development

```bash
# Check all crates
cargo check --workspace

# Run tests (125 tests: 57 core + 7 integration + 61 TUI)
cargo test --workspace

# Lint
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --all
```

### Integration tests with Docker

Start a local OpenLDAP server, then run the ignored integration tests:

```bash
docker compose -f tests/integration/docker-compose.yml up -d
cargo test --test ldap_integration -- --ignored
```

The Docker container provides:
- **Host:** localhost:3389 (LDAP), localhost:3636 (LDAPS)
- **Admin:** `cn=admin,dc=example,dc=com` / `admin`
- **Sample data:** Users (Alice, Bob), Groups (admins), OUs

### Project structure

```
.
├── Cargo.toml                    Workspace root
├── crates/
│   ├── loom/                     Binary crate
│   ├── loom-core/                LDAP client library
│   │   └── src/
│   │       ├── connection.rs     Connect, TLS, bind
│   │       ├── search.rs         Search operations
│   │       ├── modify.rs         Add, modify, delete entries
│   │       ├── bulk.rs           Bulk modifications
│   │       ├── schema.rs         Schema parsing
│   │       ├── tree.rs           Directory tree model
│   │       ├── entry.rs          LDAP entry model
│   │       ├── dn.rs             DN parsing utilities
│   │       ├── filter.rs         LDAP filter validation
│   │       ├── export/           LDIF, JSON, CSV, XLSX writers
│   │       └── import/           LDIF, JSON, CSV, XLSX readers
│   └── loom-tui/                 TUI framework
│       └── src/
│           ├── app.rs            Main app loop + action dispatch
│           ├── keymap.rs         Keybinding resolution
│           ├── theme.rs          Theme system
│           ├── config.rs         Configuration loading
│           └── components/       UI components (panels, dialogs)
├── config/
│   ├── default.toml              Default settings
│   └── themes/                   Built-in theme files
├── tests/
│   ├── fixtures/                 Sample LDIF/JSON/CSV data
│   └── integration/              Docker compose for OpenLDAP
└── .github/workflows/
    ├── ci.yml                    Check, test, clippy, fmt, MSRV
    └── release.yml               Multi-platform release builds
```

## License

Licensed under either of:

- [Apache License, Version 2.0](http://www.apache.org/licenses/LICENSE-2.0)
- [MIT License](http://opensource.org/licenses/MIT)

at your option.
