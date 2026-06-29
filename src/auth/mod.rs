// crates/dbt-auth/src/sqlserver/mod.rs
//
// `SQLServerAuth` — ADBC connection URI construction for all SQL Server auth
// variants supported by the Columnar `adbc-drivers/mssql` driver v1.3.1.
//
// Follows the Exasol / ClickHouse pattern exactly:
//   1. `SqlServerAuthIR` — internal auth representation enum (borrowed refs, lifetime 'a)
//   2. `parse_auth(&AdapterConfig)` — builds the IR from profile YAML fields
//   3. `SqlServerAuthIR::apply(builder)` — constructs `mssql://` URI, calls
//      `builder.with_parse_uri()`, then `with_username()` / `with_password()` as needed
//   4. `apply_connection_args()` — passthrough (no extra connection-level options yet)
//   5. `Auth` trait impl delegates to `auth_configure_pipeline!` macro
//
// URI format: `mssql://host[:port][/instance][?param=value&...]`
// Credentials are supplied via `builder.with_username()` / `builder.with_password()`,
// NOT embedded in the URI userinfo. This is the same pattern as ClickHouse.
//
// `with_parse_uri()` is confirmed compatible with `mssql://` — the scheme is RFC 3986
// compliant and the Columnar driver parses it cleanly. No `with_named_option()` needed.
//
// Platform support: Windows x86_64, Linux x86_64, Linux aarch64, macOS aarch64.
// No Intel Mac driver exists — enforced in dbt-xdbc/src/install.rs.

use std::borrow::Cow;

use crate::{AdapterConfig, Auth, AuthError, AuthOutcome, auth_configure_pipeline};
use database::Builder as DatabaseBuilder;
use dbt_xdbc::{Backend, database, mssql};

const DEFAULT_PORT: &str = "1433";

// ---------------------------------------------------------------------------
// Internal auth representation
// ---------------------------------------------------------------------------

/// Fully-resolved, borrow-friendly representation of a SQL Server connection.
///
/// Built from the validated `AdapterConfig` YAML fields in `parse_auth()`.
/// Lifetime `'a` borrows directly from the `AdapterConfig` — zero heap allocation
/// for string values that are already stored as `String` in the config map.
#[derive(Debug)]
enum SqlServerAuthIR<'a> {
    /// Standard SQL Server login (SQL auth).
    /// Credentials passed via `with_username()` / `with_password()`.
    SqlPassword {
        user: &'a str,
        password: Cow<'a, str>,
        host: &'a str,
        port: Cow<'a, str>,
        database: &'a str,
        instance: Option<&'a str>,
        encrypt: bool,
        trust_server_certificate: bool,
        connection_timeout: Option<Cow<'a, str>>,
    },

    /// Windows Authentication via SSPI — Kerberos, NTLM, or gMSA service account.
    /// No credentials in URI or via `with_username()` — driver uses process identity.
    /// URI param: `integrated+security=sspi`
    ///
    /// FSI: validated against a gMSA service account in a non-interactive runner context.
    /// See OQ-02 in the Phase 1 proposition document.
    WindowsAuthentication {
        host: &'a str,
        port: Cow<'a, str>,
        database: &'a str,
        instance: Option<&'a str>,
    },

    /// Entra ID (AAD) — username + password, non-interactive.
    /// Credentials passed via `with_username()` / `with_password()`.
    /// URI param: `fedauth=ActiveDirectoryPassword`
    ActiveDirectoryPassword {
        user: &'a str,
        password: Cow<'a, str>,
        host: &'a str,
        port: Cow<'a, str>,
        database: &'a str,
    },

    /// Entra ID — integrated / device-flow (interactive, developer workstations only).
    /// URI param: `fedauth=ActiveDirectoryIntegrated`
    ActiveDirectoryIntegrated {
        host: &'a str,
        port: Cow<'a, str>,
        database: &'a str,
    },

    /// Entra ID — Managed Service Identity / Workload Identity (Azure-hosted).
    /// URI param: `fedauth=ActiveDirectoryMSI`
    ActiveDirectoryMsi {
        host: &'a str,
        port: Cow<'a, str>,
        database: &'a str,
    },

    /// Ambient environment credential chain (`AZURE_CLIENT_ID` / `AZURE_CLIENT_SECRET`).
    /// URI param: `fedauth=ActiveDirectoryEnvironment`
    /// Note: Columnar driver support for this variant is pending confirmation (OQ-03).
    EnvironmentCredential {
        host: &'a str,
        port: Cow<'a, str>,
        database: &'a str,
    },
}

// ---------------------------------------------------------------------------
// URI construction helpers
// ---------------------------------------------------------------------------

/// Build the optional instance path segment.
/// Returns `"/INSTANCENAME"` for named instances, or empty string for default instance.
fn instance_path(instance: Option<&str>) -> String {
    match instance {
        Some(i) if !i.is_empty() => format!("/{i}"),
        _ => String::new(),
    }
}

/// Build the base `mssql://host:port[/instance]` URI prefix.
fn base_uri(host: &str, port: &str, instance: Option<&str>) -> String {
    format!("mssql://{host}:{port}{}", instance_path(instance))
}

/// Append `?` or `&` correctly when building the query string.
fn push_param(query: &mut String, key: &str, value: &str) {
    if query.is_empty() {
        query.push('?');
    } else {
        query.push('&');
    }
    // Encode space as `+` in parameter names (ODBC convention for URI query strings).
    query.push_str(&key.replace(' ', "+"));
    query.push('=');
    query.push_str(value);
}

// ---------------------------------------------------------------------------
// IR apply — builder construction per auth variant
// ---------------------------------------------------------------------------

impl<'a> SqlServerAuthIR<'a> {
    pub fn apply(self, mut builder: DatabaseBuilder) -> Result<DatabaseBuilder, AuthError> {
        match self {
            Self::SqlPassword {
                user,
                password,
                host,
                port,
                database,
                instance,
                encrypt,
                trust_server_certificate,
                connection_timeout,
            } => {
                let mut query = String::new();
                push_param(&mut query, mssql::DATABASE, database);

                // Emit encrypt=false only when deviating from the Columnar driver default (true).
                if !encrypt {
                    push_param(&mut query, mssql::ENCRYPT, "false");
                }
                // Emit trustservercertificate=true only when explicitly trusting (default: false).
                if trust_server_certificate {
                    push_param(&mut query, mssql::TRUST_SERVER_CERTIFICATE, "true");
                }
                if let Some(timeout) = &connection_timeout {
                    push_param(&mut query, mssql::CONNECTION_TIMEOUT, timeout);
                }
                push_param(&mut query, mssql::APP, "dbt");

                let uri = format!("{}{query}", base_uri(host, &port, instance));
                builder.with_parse_uri(uri)?;
                builder.with_username(user);
                builder.with_password(password.as_ref());
            }

            Self::WindowsAuthentication {
                host,
                port,
                database,
                instance,
            } => {
                let mut query = String::new();
                push_param(&mut query, mssql::DATABASE, database);
                push_param(&mut query, mssql::INTEGRATED_SECURITY, mssql::SSPI);
                push_param(&mut query, mssql::APP, "dbt");

                let uri = format!("{}{query}", base_uri(host, &port, instance));
                builder.with_parse_uri(uri)?;
                // No with_username / with_password — driver uses process identity.
            }

            Self::ActiveDirectoryPassword {
                user,
                password,
                host,
                port,
                database,
            } => {
                let mut query = String::new();
                push_param(&mut query, mssql::DATABASE, database);
                push_param(&mut query, mssql::FEDAUTH, mssql::fedauth::ACTIVE_DIRECTORY_PASSWORD);
                push_param(&mut query, mssql::APP, "dbt");

                let uri = format!("{}{query}", base_uri(host, &port, None));
                builder.with_parse_uri(uri)?;
                builder.with_username(user);
                builder.with_password(password.as_ref());
            }

            Self::ActiveDirectoryIntegrated { host, port, database } => {
                let mut query = String::new();
                push_param(&mut query, mssql::DATABASE, database);
                push_param(&mut query, mssql::FEDAUTH, mssql::fedauth::ACTIVE_DIRECTORY_INTEGRATED);
                push_param(&mut query, mssql::APP, "dbt");

                let uri = format!("{}{query}", base_uri(host, &port, None));
                builder.with_parse_uri(uri)?;
            }

            Self::ActiveDirectoryMsi { host, port, database } => {
                let mut query = String::new();
                push_param(&mut query, mssql::DATABASE, database);
                push_param(&mut query, mssql::FEDAUTH, mssql::fedauth::ACTIVE_DIRECTORY_MSI);
                push_param(&mut query, mssql::APP, "dbt");

                let uri = format!("{}{query}", base_uri(host, &port, None));
                builder.with_parse_uri(uri)?;
            }

            Self::EnvironmentCredential { host, port, database } => {
                let mut query = String::new();
                push_param(&mut query, mssql::DATABASE, database);
                push_param(&mut query, mssql::FEDAUTH, mssql::fedauth::ACTIVE_DIRECTORY_ENVIRONMENT);
                push_param(&mut query, mssql::APP, "dbt");

                let uri = format!("{}{query}", base_uri(host, &port, None));
                builder.with_parse_uri(uri)?;
            }
        }

        Ok(builder)
    }
}

// ---------------------------------------------------------------------------
// parse_auth — resolve AdapterConfig into SqlServerAuthIR
// ---------------------------------------------------------------------------

/// Parse `profiles.yml` fields from `AdapterConfig` into a `SqlServerAuthIR`.
///
/// All required-field validation happens here. `apply()` is infallible after
/// this point except for the driver-level `with_parse_uri()` call.
///
/// Profile fields read:
///   Required: `host`, `database`
///   Optional: `port` (default 1433), `schema`, `user`, `password`,
///             `authentication` (default `sql`), `encrypt` (default true),
///             `trust_cert` (default false), `instance`, `connection_timeout`
fn parse_auth<'a>(config: &'a AdapterConfig) -> Result<SqlServerAuthIR<'a>, AuthError> {
    let host = config
        .get_str("host")
        .ok_or_else(|| AuthError::config("SQL Server requires 'host' in profile configuration"))?;

    let database = config.get_str("database").ok_or_else(|| {
        AuthError::config("SQL Server requires 'database' in profile configuration")
    })?;

    let port: Cow<'a, str> = config
        .get_string("port")
        .unwrap_or(Cow::Borrowed(DEFAULT_PORT));

    let instance = config.get_str("instance");

    // `authentication` field maps to auth method string.
    // Accepted values match `SqlServerAuthMethod` in the DbConfig spec:
    //   "sql" | "sql_password"      → SqlPassword (default)
    //   "windows" | "windows_authentication" → WindowsAuthentication
    //   "ad_password" | "active_directory_password" → ActiveDirectoryPassword
    //   "ad_integrated" | "active_directory_integrated" → ActiveDirectoryIntegrated
    //   "ad_msi" | "active_directory_msi" → ActiveDirectoryMsi
    //   "env" | "environment_credential" → EnvironmentCredential
    let authentication = config
        .get_string("authentication")
        .unwrap_or(Cow::Borrowed("sql"));

    match authentication.to_lowercase().as_str() {
        "sql" | "sql_password" => {
            let user = config.get_str("user").ok_or_else(|| {
                AuthError::config(
                    "SQL Server SqlPassword authentication requires 'user' in profile",
                )
            })?;
            let password = config.get_string("password").ok_or_else(|| {
                AuthError::config(
                    "SQL Server SqlPassword authentication requires 'password' in profile",
                )
            })?;
            let encrypt = config
                .get_string("encrypt")
                .map(|s| s != "false" && s != "0" && s != "False")
                .unwrap_or(true);
            let trust_server_certificate = config
                .get_string("trust_cert")
                .map(|s| s == "true" || s == "1" || s == "True")
                .unwrap_or(false);
            let connection_timeout = config.get_string("connection_timeout");

            Ok(SqlServerAuthIR::SqlPassword {
                user,
                password,
                host,
                port,
                database,
                instance,
                encrypt,
                trust_server_certificate,
                connection_timeout,
            })
        }

        "windows" | "windows_authentication" => Ok(SqlServerAuthIR::WindowsAuthentication {
            host,
            port,
            database,
            instance,
        }),

        "ad_password" | "active_directory_password" => {
            let user = config.get_str("user").ok_or_else(|| {
                AuthError::config(
                    "SQL Server ActiveDirectoryPassword authentication requires 'user' in profile",
                )
            })?;
            let password = config.get_string("password").ok_or_else(|| {
                AuthError::config(
                    "SQL Server ActiveDirectoryPassword authentication requires 'password' in profile",
                )
            })?;
            Ok(SqlServerAuthIR::ActiveDirectoryPassword {
                user,
                password,
                host,
                port,
                database,
            })
        }

        "ad_integrated" | "active_directory_integrated" => {
            Ok(SqlServerAuthIR::ActiveDirectoryIntegrated { host, port, database })
        }

        "ad_msi" | "active_directory_msi" => {
            Ok(SqlServerAuthIR::ActiveDirectoryMsi { host, port, database })
        }

        "env" | "environment_credential" => {
            Ok(SqlServerAuthIR::EnvironmentCredential { host, port, database })
        }

        other => Err(AuthError::config(format!(
            "SQL Server: unrecognized authentication method '{other}'. \
             Valid values: sql, windows, ad_password, ad_integrated, ad_msi, env"
        ))),
    }
}

// ---------------------------------------------------------------------------
// apply_connection_args — connection-level option passthrough
// ---------------------------------------------------------------------------

/// No connection-level options beyond the URI for SQL Server in Phase 1.
/// Reserved for future additions (e.g. per-query timeout, connection pooling hints).
fn apply_connection_args(
    _config: &AdapterConfig,
    builder: DatabaseBuilder,
) -> Result<DatabaseBuilder, AuthError> {
    Ok(builder)
}

// ---------------------------------------------------------------------------
// Auth trait implementation
// ---------------------------------------------------------------------------

/// `SQLServerAuth` — the concrete auth provider for the SQL Server adapter.
///
/// Struct name and module path are already declared in `dbt-auth/src/lib.rs`:
///   `mod sqlserver;`
///   `Backend::SQLServer => Box::new(sqlserver::SQLServerAuth {})`
///
/// Zero-size struct — no state, all config read from `AdapterConfig` at call time.
pub struct SQLServerAuth;

impl Auth for SQLServerAuth {
    fn backend(&self) -> Backend {
        Backend::SQLServer
    }

    fn configure(&self, config: &AdapterConfig) -> Result<AuthOutcome, AuthError> {
        auth_configure_pipeline!(self.backend(), config, parse_auth, apply_connection_args)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_options::uri_value;
    use dbt_test_primitives::assert_contains;
    use dbt_yaml::Mapping;

    fn make_config(pairs: impl IntoIterator<Item = (&'static str, &'static str)>) -> AdapterConfig {
        AdapterConfig::new(Mapping::from_iter(
            pairs.into_iter().map(|(k, v)| (k.into(), v.into())),
        ))
    }

    // --- SqlPassword (default) ---

    #[test]
    fn test_sql_password_defaults() {
        let config = make_config([
            ("host", "sqlserver.prod.internal"),
            ("database", "SimulationsAnalytics"),
            ("user", "svc_dbt"),
            ("password", "hunter2"),
        ]);

        let builder = SQLServerAuth
            .configure(&config)
            .expect("configure")
            .builder;

        let uri = uri_value(&builder);
        assert_contains!(&uri, "mssql://sqlserver.prod.internal:1433");
        assert_contains!(&uri, "database=SimulationsAnalytics");
        assert_contains!(&uri, "app=dbt");
        // encrypt=false should NOT appear — default is true (omit)
        assert!(!uri.contains("encrypt=false"), "encrypt=false should be absent by default");
        // trustservercertificate=true should NOT appear — default is false (omit)
        assert!(!uri.contains("trustservercertificate=true"));
    }

    #[test]
    fn test_sql_password_custom_port() {
        let config = make_config([
            ("host", "sqlserver.prod.internal"),
            ("port", "1434"),
            ("database", "SimulationsAnalytics"),
            ("user", "svc_dbt"),
            ("password", "hunter2"),
        ]);

        let builder = SQLServerAuth
            .configure(&config)
            .expect("configure")
            .builder;

        let uri = uri_value(&builder);
        assert_contains!(&uri, "mssql://sqlserver.prod.internal:1434");
    }

    #[test]
    fn test_sql_password_encrypt_disabled() {
        let config = make_config([
            ("host", "dev-sql01"),
            ("database", "TestDB"),
            ("user", "sa"),
            ("password", "Password1"),
            ("encrypt", "false"),
        ]);

        let builder = SQLServerAuth
            .configure(&config)
            .expect("configure")
            .builder;

        let uri = uri_value(&builder);
        assert_contains!(&uri, "encrypt=false");
    }

    #[test]
    fn test_sql_password_trust_cert() {
        let config = make_config([
            ("host", "dev-sql01"),
            ("database", "TestDB"),
            ("user", "sa"),
            ("password", "Password1"),
            ("trust_cert", "true"),
        ]);

        let builder = SQLServerAuth
            .configure(&config)
            .expect("configure")
            .builder;

        let uri = uri_value(&builder);
        assert_contains!(&uri, "trustservercertificate=true");
    }

    #[test]
    fn test_sql_password_named_instance() {
        let config = make_config([
            ("host", "sqlserver.prod.internal"),
            ("database", "SimulationsAnalytics"),
            ("user", "svc_dbt"),
            ("password", "hunter2"),
            ("instance", "MSSQLSERVER2022"),
        ]);

        let builder = SQLServerAuth
            .configure(&config)
            .expect("configure")
            .builder;

        let uri = uri_value(&builder);
        assert_contains!(&uri, "mssql://sqlserver.prod.internal:1433/MSSQLSERVER2022");
    }

    #[test]
    fn test_sql_password_connection_timeout() {
        let config = make_config([
            ("host", "sqlserver.prod.internal"),
            ("database", "SimulationsAnalytics"),
            ("user", "svc_dbt"),
            ("password", "hunter2"),
            ("connection_timeout", "30"),
        ]);

        let builder = SQLServerAuth
            .configure(&config)
            .expect("configure")
            .builder;

        let uri = uri_value(&builder);
        assert_contains!(&uri, "connection+timeout=30");
    }

    // --- Windows Authentication ---

    #[test]
    fn test_windows_authentication() {
        let config = make_config([
            ("host", "sqlserver.fsi.internal"),
            ("database", "SimulationsAnalytics"),
            ("authentication", "windows"),
        ]);

        let builder = SQLServerAuth
            .configure(&config)
            .expect("configure")
            .builder;

        let uri = uri_value(&builder);
        assert_contains!(&uri, "mssql://sqlserver.fsi.internal:1433");
        assert_contains!(&uri, "integrated+security=sspi");
        assert_contains!(&uri, "app=dbt");
    }

    #[test]
    fn test_windows_authentication_named_instance() {
        let config = make_config([
            ("host", "sqlserver.fsi.internal"),
            ("database", "SimulationsAnalytics"),
            ("authentication", "windows"),
            ("instance", "PROD"),
        ]);

        let builder = SQLServerAuth
            .configure(&config)
            .expect("configure")
            .builder;

        let uri = uri_value(&builder);
        assert_contains!(&uri, "mssql://sqlserver.fsi.internal:1433/PROD");
        assert_contains!(&uri, "integrated+security=sspi");
    }

    // --- Entra ID ---

    #[test]
    fn test_active_directory_password() {
        let config = make_config([
            ("host", "azure-sql.database.windows.net"),
            ("database", "AnalyticsDB"),
            ("user", "svc-dbt@corp.onmicrosoft.com"),
            ("password", "SecretP@ss"),
            ("authentication", "ad_password"),
        ]);

        let builder = SQLServerAuth
            .configure(&config)
            .expect("configure")
            .builder;

        let uri = uri_value(&builder);
        assert_contains!(&uri, "fedauth=ActiveDirectoryPassword");
    }

    #[test]
    fn test_active_directory_msi() {
        let config = make_config([
            ("host", "azure-sql.database.windows.net"),
            ("database", "AnalyticsDB"),
            ("authentication", "ad_msi"),
        ]);

        let builder = SQLServerAuth
            .configure(&config)
            .expect("configure")
            .builder;

        let uri = uri_value(&builder);
        assert_contains!(&uri, "fedauth=ActiveDirectoryMSI");
    }

    // --- Error cases ---

    #[test]
    fn test_missing_host_returns_error() {
        let config = make_config([
            ("database", "SimulationsAnalytics"),
            ("user", "svc_dbt"),
            ("password", "hunter2"),
        ]);
        let result = SQLServerAuth.configure(&config);
        assert!(result.is_err());
        assert_contains!(result.unwrap_err().msg(), "host");
    }

    #[test]
    fn test_missing_database_returns_error() {
        let config = make_config([
            ("host", "sqlserver.prod.internal"),
            ("user", "svc_dbt"),
            ("password", "hunter2"),
        ]);
        let result = SQLServerAuth.configure(&config);
        assert!(result.is_err());
        assert_contains!(result.unwrap_err().msg(), "database");
    }

    #[test]
    fn test_missing_user_for_sql_password_returns_error() {
        let config = make_config([
            ("host", "sqlserver.prod.internal"),
            ("database", "SimulationsAnalytics"),
            ("password", "hunter2"),
        ]);
        let result = SQLServerAuth.configure(&config);
        assert!(result.is_err());
        assert_contains!(result.unwrap_err().msg(), "user");
    }

    #[test]
    fn test_missing_password_for_sql_password_returns_error() {
        let config = make_config([
            ("host", "sqlserver.prod.internal"),
            ("database", "SimulationsAnalytics"),
            ("user", "svc_dbt"),
        ]);
        let result = SQLServerAuth.configure(&config);
        assert!(result.is_err());
        assert_contains!(result.unwrap_err().msg(), "password");
    }

    #[test]
    fn test_unknown_auth_method_returns_error() {
        let config = make_config([
            ("host", "sqlserver.prod.internal"),
            ("database", "SimulationsAnalytics"),
            ("authentication", "kerberos_wizard"),
        ]);
        let result = SQLServerAuth.configure(&config);
        assert!(result.is_err());
        assert_contains!(result.unwrap_err().msg(), "kerberos_wizard");
    }
}
