use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("connection failed: {0}")]
    ConnectionFailed(String),

    #[error("bind failed: {0}")]
    BindFailed(String),

    #[error("search failed: {0}")]
    SearchFailed(String),

    #[error("modify failed: {0}")]
    ModifyFailed(String),

    #[error("add failed: {0}")]
    AddFailed(String),

    #[error("delete failed: {0}")]
    DeleteFailed(String),

    #[error("schema error: {0}")]
    SchemaError(String),

    #[error("DN parse error: {0}")]
    DnParseError(String),

    #[error("export error: {0}")]
    ExportError(String),

    #[error("import error: {0}")]
    ImportError(String),

    #[error("credential error: {0}")]
    CredentialError(String),

    #[error("timeout")]
    Timeout,

    #[error("ldap error: {0}")]
    Ldap(#[from] ldap3::LdapError),
}
