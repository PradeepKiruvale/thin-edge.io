use async_trait::async_trait;
use tedge_actors::Actor;
use tedge_actors::ReceiveMessages;
use tedge_actors::RuntimeError;
use tedge_actors::SimpleMessageBox;
use tedge_api::health::health_status_down_message;
use tedge_api::health::health_status_up_message;
use tedge_mqtt_ext::MqttMessage;

pub struct HealthMonitorActor {
    daemon_to_be_monitored: String,
}

impl HealthMonitorActor {
    pub fn new(daemon_to_be_monitored: String) -> Self {
        Self {
            daemon_to_be_monitored,
        }
    }

    pub fn up_health_status(&self) -> MqttMessage {
        health_status_up_message(&self.daemon_to_be_monitored)
    }

    pub fn down_health_status(&self) -> MqttMessage {
        health_status_down_message(&self.daemon_to_be_monitored)
    }
}

#[async_trait]
impl Actor for HealthMonitorActor {
    type MessageBox = SimpleMessageBox<MqttMessage, MqttMessage>;

    fn name(&self) -> &str {
        "HealthMonitorActor"
    }

    async fn run(mut self, mut messages: Self::MessageBox) -> Result<(), RuntimeError> {
        messages.send(self.up_health_status()).await?;
        while let Some(_message) = messages.recv().await {
            {
                messages.send(self.up_health_status()).await?;
            }
        }
        Ok(())
    }
}
