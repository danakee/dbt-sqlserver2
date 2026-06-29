// crates/dbt-xdbc/src/mssql.rs
//
// URI query-parameter constants for the Columnar `adbc-drivers/mssql` ADBC driver v1.3.1.
//
// These constants name the query-string keys accepted by the driver when a connection
// is established via `DatabaseBuilder::with_parse_uri()` with a `mssql://` URI.
//
// URI format:
//   mssql://[host[:port]][/instance][?key=value&key=value...]
//   (credentials supplied separately via `with_username()` / `with_password()`)
//
// Auth variant examples:
//   SQL auth:           mssql://host:1433?database=db         (+ with_username / with_password)
//   Windows / gMSA:     mssql://host:1433?database=db&integrated+security=sspi
//   EntraID password:   mssql://host:1433?database=db&fedauth=ActiveDirectoryPassword
//   EntraID integrated: mssql://host:1433?database=db&fedauth=ActiveDirectoryIntegrated
//   EntraID MSI:        mssql://host:1433?database=db&fedauth=ActiveDirectoryMSI
//
// Platform support: Windows x86_64, Linux x86_64, Linux aarch64, macOS aarch64.
// No Intel Mac (x86_64-apple-darwin) driver — enforced in install.rs checksum tests.
//
// Driver version declared in lib.rs: MSSQLSERVER_DRIVER_VERSION = "1.3.1"

// ---------------------------------------------------------------------------
// Core connection parameters
// ---------------------------------------------------------------------------

/// Target database name.
/// URI key: `database`
/// Example: `?database=SimulationsAnalytics`
pub const DATABASE: &str = "database";

/// Connection timeout in seconds.
/// URI key: `connection timeout`
/// Space encoded as `+` in query string: `?connection+timeout=30`
pub const CONNECTION_TIMEOUT: &str = "connection timeout";

/// Application name reported to SQL Server (visible in `sys.dm_exec_sessions`).
/// Useful for monitoring and auditing dbt-initiated connections.
/// URI key: `app`
/// Example: `?app=dbt`
pub const APP: &str = "app";

// ---------------------------------------------------------------------------
// TLS / encryption parameters
// ---------------------------------------------------------------------------

/// Whether to encrypt the connection.
/// URI key: `encrypt`
/// Accepted values: `true` | `false`
/// Default in the Columnar driver (ODBC Driver 18 behavior): `true`
/// Omit when `true` (default) — only emit when explicitly disabling encryption.
pub const ENCRYPT: &str = "encrypt";

/// Whether to skip TLS server certificate validation.
/// URI key: `trustservercertificate`
/// Accepted values: `true` | `false`
/// Default: `false` (certificate is validated by the driver).
/// Set to `true` only in dev/test environments with self-signed certificates.
pub const TRUST_SERVER_CERTIFICATE: &str = "trustservercertificate";

// ---------------------------------------------------------------------------
// Windows Integrated Security (NTLM / Kerberos / gMSA)
// ---------------------------------------------------------------------------

/// Enables Windows Authentication via SSPI (NTLM, Kerberos, or gMSA token).
/// URI key: `integrated security`
/// Required value: `sspi` (see `SSPI` constant below).
/// Space encoded as `+` in query string: `?integrated+security=sspi`
/// When set, omit user/password — the driver uses the process identity.
///
/// FSI use case: gMSA service account in non-interactive CI/CD context.
/// OQ-02: functional validation against gMSA service account pending.
pub const INTEGRATED_SECURITY: &str = "integrated security";

/// The only accepted value for `INTEGRATED_SECURITY`.
pub const SSPI: &str = "sspi";

// ---------------------------------------------------------------------------
// Microsoft Entra ID (Azure Active Directory) federated auth
// ---------------------------------------------------------------------------

/// Selects the Entra ID / AAD federated authentication method.
/// URI key: `fedauth`
/// See the `fedauth` sub-module constants for accepted values.
pub const FEDAUTH: &str = "fedauth";

pub mod fedauth {
    /// Username + password credential, non-interactive (service principal).
    pub const ACTIVE_DIRECTORY_PASSWORD: &str = "ActiveDirectoryPassword";

    /// Integrated / device-flow authentication (interactive — developer workstations only).
    pub const ACTIVE_DIRECTORY_INTEGRATED: &str = "ActiveDirectoryIntegrated";

    /// Managed Service Identity / Workload Identity (Azure-hosted environments).
    pub const ACTIVE_DIRECTORY_MSI: &str = "ActiveDirectoryMSI";

    /// Ambient environment credential chain (`AZURE_CLIENT_ID` / `AZURE_CLIENT_SECRET` /
    /// `AZURE_TENANT_ID` env vars). Driver support pending confirmation — see OQ-03.
    pub const ACTIVE_DIRECTORY_ENVIRONMENT: &str = "ActiveDirectoryEnvironment";
}
