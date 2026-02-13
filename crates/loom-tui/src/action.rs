use crate::components::attribute_editor::EditResult;
use crate::components::bulk_update_dialog::BulkOp;
use crate::config::ConnectionProfile;
use loom_core::entry::LdapEntry;
use loom_core::schema::SchemaCache;
use loom_core::server_detect::ServerType;
use loom_core::tree::TreeNode;

/// Unique identifier for a connection tab.
pub type ConnectionId = usize;

/// All actions that can flow through the application.
#[derive(Debug, Clone)]
pub enum Action {
    // System
    Tick,
    Render,
    Quit,
    Resize(u16, u16),

    // Focus & Navigation
    FocusNext,
    FocusPrev,
    FocusPanel(FocusTarget),

    // Tab Management
    NextTab,
    PrevTab,
    NewTab,
    CloseTab(ConnectionId),
    SwitchTab(ConnectionId),

    // Connection
    ShowConnectDialog,
    ShowNewConnectionForm,
    ConnectByIndex(usize),
    ConnectAdHoc(ConnectionProfile, String), // profile + password (never saved)
    PromptCredentials(ConnectionProfile),    // show credential prompt for profile
    ConnectWithCredentials(ConnectionProfile, String), // retry with user-provided credentials
    Connected(ConnectionId, String, ServerType),
    Disconnected(ConnectionId),
    ConnectionError(String),
    SaveCurrentConnection,

    // Tree Navigation
    TreeExpand(String),
    TreeCollapse(String),
    TreeSelect(String),
    TreeChildrenLoaded(ConnectionId, String, Vec<TreeNode>),
    TreeUp,
    TreeDown,
    TreeToggle,

    // Entry Detail
    EntryLoaded(ConnectionId, LdapEntry),
    EntryRefresh,

    // Search
    SearchExecute(String),
    SearchResults(ConnectionId, Vec<LdapEntry>),
    SearchClear,
    SearchFocusInput,

    // Attribute Editing
    EditAttribute(String, String, String), // dn, attr_name, current_value
    AddAttribute(String, String),          // dn, attr_name
    SaveAttribute(EditResult),
    AttributeSaved(String), // dn that was updated

    // Export / Import
    ShowExportDialog,
    ExportExecute {
        path: String,
        filter: String,
        attributes: Vec<String>,
    },
    ExportComplete(String), // success message

    // Bulk Update
    ShowBulkUpdateDialog,
    BulkUpdateExecute {
        filter: String,
        attribute: String,
        value: String,
        op: BulkOp,
    },
    BulkUpdateComplete(String), // result message

    // Schema
    ShowSchemaViewer,
    SchemaLoaded(ConnectionId, Box<SchemaCache>),

    // Log Panel
    ToggleLogPanel,

    // Popup / Modal
    ShowConfirm(String, Box<Action>),
    PopupConfirm,
    PopupCancel,
    ClosePopup,

    // Status
    StatusMessage(String),
    ErrorMessage(String),

    // No-op
    None,
}

/// Which panel is focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTarget {
    TreePanel,
    DetailPanel,
    CommandPanel,
}
