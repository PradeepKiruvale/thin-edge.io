use crate::Config;
use crate::MqttError;
use rumqttc::AsyncClient;
use rumqttc::ConnectReturnCode;
use rumqttc::Event;
use rumqttc::Packet;
use tracing::warn;

/// Create a persistent session on the MQTT server `config.host`.
///
/// The session is named after the `config.session_name`
/// subscribing to all the topics given by the `config.subscriptions`.
///
/// A new `Connection` created with a config with the same session name,
/// will receive all the messages published meantime on the subscribed topics.
///
/// This function can be called multiple times with the same session name,
/// since it consumes no messages.
pub async fn init_session(config: &Config) -> Result<(), MqttError> {
    if config.clean_session || config.session_name.is_none() {
        return Err(MqttError::InvalidSessionConfig);
    }

    let mqtt_options = config.rumqttc_options()?;
    let (mqtt_client, mut event_loop) = AsyncClient::new(mqtt_options, config.queue_capacity);

    loop {
        match event_loop.poll().await {
            Ok(Event::Incoming(Packet::ConnAck(ack))) => {
                if let Some(err) = MqttError::maybe_connection_error(&ack) {
                    return Err(err);
                };
                let subscriptions = config.subscriptions.filters();
                if subscriptions.is_empty() {
                    break;
                }
                mqtt_client.subscribe_many(subscriptions).await?;
            }

            Ok(Event::Incoming(Packet::SubAck(_))) => {
                break;
            }

            Err(err) => {
                match err {
                    rumqttc::ConnectionError::ConnectionRefused(ConnectReturnCode::BadClientId) => {
                        warn!("Failed to initialize the session with MQTT Broker due to bad client id");
                    }
                    rumqttc::ConnectionError::ConnectionRefused(
                        ConnectReturnCode::BadUserNamePassword,
                    ) => {
                        warn!(
                            "Failed to initialize the session with MQTT Broker due to bad user name and password"
                        );
                    }
                    rumqttc::ConnectionError::ConnectionRefused(
                        ConnectReturnCode::NotAuthorized,
                    ) => {
                        warn!("Failed to initialize the session with MQTT Broker due to not authorized");
                    }
                    rumqttc::ConnectionError::ConnectionRefused(
                        ConnectReturnCode::RefusedProtocolVersion,
                    ) => {
                        warn!("Failed to initialize the session with MQTT Broker due to protocol version mismatch");
                    }
                    rumqttc::ConnectionError::ConnectionRefused(
                        ConnectReturnCode::ServiceUnavailable,
                    ) => {
                        warn!("Failed to initialize the session with MQTT Broker due to service not available");
                    }
                    rumqttc::ConnectionError::ConnectionRefused(ConnectReturnCode::Success) => {}
                    e => {
                        warn!(
                            "Failed to initialize the session with MQTT Broker due to {}",
                            e
                        );
                    }
                }

                break;
            }
            _ => (),
        }
    }

    // Errors on disconnect are ignored, since having no impact on the session
    let _ = mqtt_client.disconnect().await;
    Ok(())
}

/// Clear a persistent session on the MQTT server `config.host`.
///
/// The session named after the `config.session_name` is cleared
/// unsubscribing to all the topics given by the `config.subscriptions`.
///
/// All the messages persisted for that session all cleared.
/// and no more messages will be stored till the session is re-created.
///
/// A new `Connection` created with a config with the same session name,
/// will receive no messages that have been published meantime.
pub async fn clear_session(config: &Config) -> Result<(), MqttError> {
    if config.session_name.is_none() {
        return Err(MqttError::InvalidSessionConfig);
    }
    let mut mqtt_options = config.rumqttc_options()?;
    mqtt_options.set_clean_session(true);
    let (mqtt_client, mut event_loop) = AsyncClient::new(mqtt_options, config.queue_capacity);

    loop {
        match event_loop.poll().await {
            Ok(Event::Incoming(Packet::ConnAck(ack))) => {
                if let Some(err) = MqttError::maybe_connection_error(&ack) {
                    return Err(err);
                };
                break;
            }

            Err(err) => {
                eprintln!("Connection Error {}", err);
                break;
            }
            _ => (),
        }
    }

    // Errors on disconnect are ignored, since having no impact on the session
    let _ = mqtt_client.disconnect().await;
    Ok(())
}
