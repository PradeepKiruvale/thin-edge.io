use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

pub const SERVICE_CONFIG_FILE: &str = "system.toml";

#[derive(thiserror::Error, Debug)]
pub enum SystemServiceError {
    #[error("Service command <{service_command:?}> failed with code: {code:?}.")]
    ServiceCommandFailedWithCode { service_command: String, code: i32 },

    #[error("Service command <{service_command:?}> terminated by a signal.")]
    ServiceCommandFailedBySignal { service_command: String },

    #[error(
        "Service command <{service_command:?}> not found.\n\
    Check '{path}' file."
    )]
    ServiceCommandNotFound {
        service_command: String,
        path: String,
    },

    #[error("Failed to execute '{cmd}' to check the service manager availability.\n\
     Service manager '{name}' is not available on the system or elevated permissions have not been granted.")]
    ServiceManagerUnavailable { cmd: String, name: String },

    #[error("Toml syntax error in the system config file '{path}': {reason}")]
    SystemConfigInvalidToml { path: String, reason: String },

    #[error(
        "Syntax error in the system config file for '{cmd}': {reason}\n\
    Check '{path}' file."
    )]
    SystemConfigInvalidSyntax {
        reason: String,
        cmd: String,
        path: String,
    },
}

#[derive(Deserialize, Debug, Default, Eq, PartialEq)]
pub struct SystemConfig {
    pub(crate) init: Option<InitConfig>,
    pub(crate) log: Option<LogConfig>,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LogConfig {
    pub is_debug: bool,
}

#[derive(Deserialize, Debug, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct InitConfig {
    pub name: String,
    pub is_available: Vec<String>,
    pub restart: Vec<String>,
    pub stop: Vec<String>,
    pub enable: Vec<String>,
    pub disable: Vec<String>,
    pub is_active: Vec<String>,
}

impl Default for InitConfig {
    fn default() -> Self {
        Self {
            name: "systemd".to_string(),
            is_available: vec!["/bin/systemctl".into(), "--version".into()],
            restart: vec!["/bin/systemctl".into(), "restart".into(), "{}".into()],
            stop: vec!["/bin/systemctl".into(), "stop".into(), "{}".into()],
            enable: vec!["/bin/systemctl".into(), "enable".into(), "{}".into()],
            disable: vec!["/bin/systemctl".into(), "disable".into(), "{}".into()],
            is_active: vec!["/bin/systemctl".into(), "is-active".into(), "{}".into()],
        }
    }
}

impl Default for LogConfig {
    fn default() -> Self {
        Self { is_debug: true }
    }
}

impl SystemConfig {
    pub fn try_new(config_root: PathBuf) -> Result<Self, SystemServiceError> {
        let config_path = config_root.join(SERVICE_CONFIG_FILE);
        let config_path_str = config_path.to_str().unwrap_or(SERVICE_CONFIG_FILE);

        match fs::read_to_string(config_path.clone()) {
            Ok(contents) => {
                let config: SystemConfig = toml::from_str(contents.as_str()).map_err(|e| {
                    SystemServiceError::SystemConfigInvalidToml {
                        path: config_path_str.to_string(),
                        reason: format!("{}", e),
                    }
                })?;
                Ok(config)
            }
            Err(_) => {
                println!("The system config file '{}' doesn't exist. Use '/bin/systemctl' as a service manager.\n", config_path_str);
                Ok(Self::default())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn deserialize_system_config() {
        let config: SystemConfig = toml::from_str(
            r#"
            [init]
            name = "systemd"
            is_available = ["/bin/systemctl", "--version"]
            restart = ["/bin/systemctl", "restart", "{}"]
            stop =  ["/bin/systemctl", "stop", "{}"]
            enable =  ["/bin/systemctl", "enable", "{}"]
            disable =  ["/bin/systemctl", "disable", "{}"]
            is_active = ["/bin/systemctl", "is-active", "{}"]
          
        "#,
        )
        .unwrap();

        assert_eq!(config.init.name, "systemd");
        assert_eq!(
            config.init.is_available,
            vec!["/bin/systemctl", "--version"]
        );
        assert_eq!(config.init.restart, vec!["/bin/systemctl", "restart", "{}"]);
        assert_eq!(config.init.stop, vec!["/bin/systemctl", "stop", "{}"]);
        assert_eq!(config.init.enable, vec!["/bin/systemctl", "enable", "{}"]);
        assert_eq!(config.init.disable, vec!["/bin/systemctl", "disable", "{}"]);
        assert_eq!(
            config.init.is_active,
            vec!["/bin/systemctl", "is-active", "{}"]
        );
        assert_eq!(config.init.is_debug, false);
    }

    #[test]
    fn read_system_config_file() -> anyhow::Result<()> {
        let toml_conf = r#"
            [init]
            name = "systemd"
            is_available = ["/bin/systemctl", "--version"]
            restart = ["/bin/systemctl", "restart", "{}"]
            stop =  ["/bin/systemctl", "stop", "{}"]
            enable =  ["/bin/systemctl", "enable", "{}"]
            disable =  ["/bin/systemctl", "disable", "{}"]
            is_active = ["/bin/systemctl", "is-active", "{}"]
           
        "#;
        let expected_config: SystemConfig = toml::from_str(toml_conf)?;

        let (_dir, config_root_path) = create_temp_system_config(toml_conf)?;
        let config = SystemConfig::try_new(config_root_path).unwrap();

        assert_eq!(config, expected_config);

        Ok(())
    }

    // Need to return TempDir, otherwise the dir will be deleted when this function ends.
    fn create_temp_system_config(content: &str) -> std::io::Result<(TempDir, PathBuf)> {
        let temp_dir = TempDir::new()?;
        let config_root = temp_dir.path().to_path_buf();
        let config_file_path = config_root.join(SERVICE_CONFIG_FILE);
        let mut file = std::fs::File::create(config_file_path.as_path())?;
        file.write_all(content.as_bytes())?;
        Ok((temp_dir, config_root))
    }
}
