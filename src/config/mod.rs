use crate::adapter_config::common::{ConfigField, ConfigProcessor, FieldValue, InteractiveSetup};

use dbt_common::FsResult;
use dbt_schemas::schemas::profiles::SqlserverDbConfig;

// Authentication methods supported by the Columnar ADBC mssql driver.
// Values match the `authentication` field accepted by SqlserverDbConfig / dbt-auth.
// Order determines the order shown in the select prompt.
const AUTH_METHODS: &[(&str, &str)] = &[
    ("windows", "Windows Authentication (Kerberos / SSPI — on-prem, gMSA)"),
    ("sql", "SQL Authentication (username + password)"),
    ("ad_password", "Entra ID Password (username + password + client ID)"),
    ("ad_integrated", "Entra ID Integrated (DefaultAzureCredential chain)"),
    ("ad_msi", "Entra ID Managed Identity (MSI / workload identity)"),
    ("env", "Environment Credential (DefaultAzureCredential env vars)"),
];

impl InteractiveSetup for SqlserverDbConfig {
    fn get_fields() -> Vec<ConfigField> {
        let auth_labels = auth_label_options();
        // Default to Windows auth — most common for on-prem SQL Server deployments.
        let auth_default = 0;

        // Index lookups for auth-dependent fields.
        let sql_idx = auth_index("sql").unwrap_or(1);
        let adpw_idx = auth_index("ad_password").unwrap_or(2);

        vec![
            // Core connection settings
            ConfigField::input(
                "host",
                "Host (SQL Server hostname or IP, e.g. sql.mycompany.com)",
            ),
            ConfigField::optional_input("port", "Port", Some("1433")),
            ConfigField::input("database", "Database"),
            ConfigField::optional_input("schema", "Schema", Some("dbo")),
            ConfigField::optional_input("threads", "Threads", Some("4")),
            // Authentication
            ConfigField::select(
                "auth_method",
                "Which authentication method would you like to use?",
                auth_labels,
                auth_default,
            ),
            // SQL auth fields
            ConfigField::input("user", "Username")
                .when_field_equals("auth_method", FieldValue::Integer(sql_idx)),
            ConfigField::password("password", "Password")
                .when_field_equals("auth_method", FieldValue::Integer(sql_idx)),
            // Entra ID password fields
            ConfigField::input(
                "user_adpw",
                "Username (Entra account, e.g. user@tenant.onmicrosoft.com)",
            )
            .when_field_equals("auth_method", FieldValue::Integer(adpw_idx)),
            ConfigField::password("password_adpw", "Password")
                .when_field_equals("auth_method", FieldValue::Integer(adpw_idx)),
            // Optional: instance name
            ConfigField::optional_input(
                "instance",
                "Named instance (optional, leave blank for default)",
                None,
            ),
            // TLS options
            ConfigField::confirm("encrypt", "Encrypt the connection? (recommended)", true),
            ConfigField::confirm(
                "trust_cert",
                "Trust server certificate without validation? (Yes only for dev/self-signed certs)",
                false,
            ),
        ]
    }

    fn set_field(&mut self, field_name: &str, value: FieldValue) -> FsResult<()> {
        match field_name {
            "host" => {
                if let FieldValue::String(s) = value {
                    self.host = Some(s);
                }
            }
            "port" => {
                if let FieldValue::String(s) = value && !s.is_empty() {
                    self.port = Some(s.into());
                }
            }
            "database" => {
                if let FieldValue::String(s) = value {
                    self.database = Some(s);
                }
            }
            "schema" => {
                if let FieldValue::String(s) = value {
                    self.schema = Some(s);
                }
            }
            "threads" => {
                if let FieldValue::String(s) = value && !s.is_empty() {
                    self.threads = Some(s.into());
                }
            }
            "auth_method" => {
                if let FieldValue::Integer(i) = value
                    && let Some((val, _)) = AUTH_METHODS.get(i as usize)
                {
                    self.authentication = Some((*val).to_string());
                }
            }
            "user" => {
                if let FieldValue::String(s) = value && !s.is_empty() {
                    self.user = Some(s);
                }
            }
            "password" => {
                if let FieldValue::String(s) = value {
                    self.password = Some(s);
                }
            }
            "user_adpw" => {
                if let FieldValue::String(s) = value && !s.is_empty() {
                    self.user = Some(s);
                }
            }
            "password_adpw" => {
                if let FieldValue::String(s) = value {
                    self.password = Some(s);
                }
            }
            "instance" => {
                if let FieldValue::String(s) = value && !s.is_empty() {
                    self.instance = Some(s);
                }
            }
            "encrypt" => {
                if let FieldValue::Boolean(b) = value {
                    self.encrypt = Some(b);
                }
            }
            "trust_cert" => {
                if let FieldValue::Boolean(b) = value {
                    self.trust_cert = Some(b);
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn get_field(&self, field_name: &str) -> Option<FieldValue> {
        match field_name {
            "host" => self.host.as_ref().map(|s| FieldValue::String(s.clone())),
            "port" => self
                .port
                .as_ref()
                .map(|p| FieldValue::String(p.to_string())),
            "database" => self
                .database
                .as_ref()
                .map(|s| FieldValue::String(s.clone())),
            "schema" => self.schema.as_ref().map(|s| FieldValue::String(s.clone())),
            "threads" => self
                .threads
                .as_ref()
                .map(|t| FieldValue::String(t.to_string())),
            "auth_method" => self
                .authentication
                .as_deref()
                .and_then(auth_index)
                .map(FieldValue::Integer),
            "user" | "user_adpw" => self.user.as_ref().map(|s| FieldValue::String(s.clone())),
            "password" | "password_adpw" => self
                .password
                .as_ref()
                .map(|s| FieldValue::String(s.clone())),
            "instance" => self
                .instance
                .as_ref()
                .map(|s| FieldValue::String(s.clone())),
            "encrypt" => self.encrypt.map(FieldValue::Boolean),
            "trust_cert" => self.trust_cert.map(FieldValue::Boolean),
            _ => None,
        }
    }

    fn is_field_set(&self, field_name: &str) -> bool {
        match field_name {
            "host" => self.host.is_some(),
            "port" => self.port.is_some(),
            "database" => self.database.is_some(),
            "schema" => self.schema.is_some(),
            "threads" => self.threads.is_some(),
            "auth_method" => self
                .authentication
                .as_deref()
                .map(auth_index)
                .is_some_and(|o| o.is_some()),
            "user" | "user_adpw" => self.user.is_some(),
            "password" | "password_adpw" => self.password.is_some(),
            "instance" => self.instance.is_some(),
            "encrypt" => self.encrypt.is_some(),
            "trust_cert" => self.trust_cert.is_some(),
            _ => false,
        }
    }
}

fn auth_index(value: &str) -> Option<i64> {
    AUTH_METHODS
        .iter()
        .position(|(v, _)| v.eq_ignore_ascii_case(value))
        .map(|i| i as i64)
}

fn auth_label_options() -> Vec<&'static str> {
    AUTH_METHODS.iter().map(|(_, label)| *label).collect()
}

fn default_sqlserver_config() -> SqlserverDbConfig {
    SqlserverDbConfig {
        host: None,
        port: Some("1433".to_string().into()),
        database: None,
        schema: Some("dbo".to_string()),
        user: None,
        password: None,
        authentication: Some("windows".to_string()),
        instance: None,
        encrypt: Some(true),
        trust_cert: Some(false),
        connection_timeout: None,
        threads: Some("4".to_string().into()),
    }
}

pub fn setup_sqlserver_profile(
    existing_config: Option<&SqlserverDbConfig>,
) -> FsResult<Box<SqlserverDbConfig>> {
    let default_config = default_sqlserver_config();
    let config = ConfigProcessor::process_config(existing_config.or(Some(&default_config)))?;
    Ok(Box::new(config))
}
