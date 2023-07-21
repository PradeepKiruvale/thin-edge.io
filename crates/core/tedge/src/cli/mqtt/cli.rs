use crate::cli::mqtt::publish::MqttPublishCommand;
use crate::cli::mqtt::subscribe::MqttSubscribeCommand;
use crate::cli::mqtt::MqttError;
use crate::command::BuildCommand;
use crate::command::BuildContext;
use crate::command::Command;
use camino::Utf8PathBuf;
use rumqttc::QoS;
use log::info;
use std::time::Duration;

const PUB_CLIENT_PREFIX: &str = "tedge-pub";
const SUB_CLIENT_PREFIX: &str = "tedge-sub";
const DISCONNECT_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(clap::Subcommand, Debug)]
pub enum TEdgeMqttCli {
    /// Publish a MQTT message on a topic.
    Pub {
        /// Topic to publish
        topic: String,
        /// Message to publish
        message: String,
        /// QoS level (0, 1, 2)
        #[clap(short, long, parse(try_from_str = parse_qos), default_value = "0")]
        qos: QoS,
        /// Retain flag
        #[clap(short, long = "retain")]
        retain: bool,
    },

    /// Subscribe a MQTT topic.
    Sub {
        /// Topic to subscribe to
        topic: String,
        /// QoS level (0, 1, 2)
        #[clap(short, long, parse(try_from_str = parse_qos), default_value = "0")]
        qos: QoS,
        /// Avoid printing the message topics on the console
        #[clap(long = "no-topic")]
        hide_topic: bool,
    },
}

impl BuildCommand for TEdgeMqttCli {
    fn build_command(self, context: BuildContext) -> Result<Box<dyn Command>, crate::ConfigError> {
        let config = context.config_repository.load_new()?;
        info!("++++in build cmd+++++++++++++++++++++");
        let auth_config = config.mqtt_client_auth_configs()?;

        let client_auth_config =
            if auth_config.cert_file.is_none() && auth_config.key_file.is_none() {
                info!("...............None....................");
                None
            } else {
                info!("&&&&&&&&&&&&&& with certs &&&&&&&&&&&&&&&&&&");
                Some(ClientAuthConfig {
                    cert_file: auth_config.cert_file.clone().unwrap_or_default(),
                    key_file: auth_config.key_file.unwrap_or_default(),
                })
            };

        let cmd = {
            match self {
                TEdgeMqttCli::Pub {
                    topic,
                    message,
                    qos,
                    retain,
                } => MqttPublishCommand {
                    host: config.mqtt.client.host.clone(),
                    port: config.mqtt.client.port.into(),
                    topic,
                    message,
                    qos,
                    client_id: format!("{}-{}", PUB_CLIENT_PREFIX, std::process::id()),
                    disconnect_timeout: DISCONNECT_TIMEOUT,
                    retain,
                    ca_file: auth_config.ca_file.clone(),
                    ca_dir: auth_config.ca_dir,
                    client_auth_config,
                }
                .into_boxed(),
                TEdgeMqttCli::Sub {
                    topic,
                    qos,
                    hide_topic,
                } => MqttSubscribeCommand {
                    host: config.mqtt.client.host.clone(),
                    port: config.mqtt.client.port.into(),
                    topic,
                    qos,
                    hide_topic,
                    client_id: format!("{}-{}", SUB_CLIENT_PREFIX, std::process::id()),
                    ca_file: auth_config.ca_file,
                    ca_dir: auth_config.ca_dir,
                    client_auth_config,
                }
                .into_boxed(),
            }
        };

        Ok(cmd)
    }
}

fn parse_qos(src: &str) -> Result<QoS, MqttError> {
    let int_val: u8 = src.parse().map_err(|_| MqttError::InvalidQoS)?;
    match int_val {
        0 => Ok(QoS::AtMostOnce),
        1 => Ok(QoS::AtLeastOnce),
        2 => Ok(QoS::ExactlyOnce),
        _ => Err(MqttError::InvalidQoS),
    }
}

pub struct ClientAuthConfig {
    pub cert_file: Utf8PathBuf,
    pub key_file: Utf8PathBuf,
}

#[cfg(test)]
mod tests {
    use super::parse_qos;
    use rumqttc::QoS;

    #[test]
    fn test_parse_qos_at_most_once() {
        let input_qos = "0";
        let expected_qos = QoS::AtMostOnce;
        assert_eq!(parse_qos(input_qos).unwrap(), expected_qos);
    }

    #[test]
    fn test_parse_qos_at_least_once() {
        let input_qos = "1";
        let expected_qos = QoS::AtLeastOnce;
        assert_eq!(parse_qos(input_qos).unwrap(), expected_qos);
    }

    #[test]
    fn test_parse_qos_exactly_once() {
        let input_qos = "2";
        let expected_qos = QoS::ExactlyOnce;
        assert_eq!(parse_qos(input_qos).unwrap(), expected_qos);
    }
}
