use anyhow::bail;
use async_trait::async_trait;
use tedge_actors::Actor;
use tedge_actors::DynSender;
use tedge_actors::ReceiveMessages;
use tedge_actors::RuntimeError;
use tedge_actors::SimpleMessageBox;
use tedge_api::health::health_status_down_message;
use tedge_api::health::health_status_up_message;
use tedge_mqtt_ext::MqttMessage;
use tedge_mqtt_ext::TopicFilter;

pub struct HealthMonitorActor {
    health_check_topics: TopicFilter,
    mqtt_publisher: DynSender<MqttMessage>,
    daemon_to_be_monitored: String,
}

impl HealthMonitorActor {
    pub fn new(
        daemon_to_be_monitored: String,
        mqtt_publisher: DynSender<MqttMessage>,
        health_check_topics: TopicFilter,
    ) -> Self {
        Self {
            health_check_topics,
            mqtt_publisher,
            daemon_to_be_monitored,
        }
    }

    pub async fn process_mqtt_message(
        &mut self,
        message: MqttMessage,
    ) -> Result<(), anyhow::Error> {
        if self.health_check_topics.accept(&message) {
            //Process the mqtt message and send the reply to the health check request message
            self.mqtt_publisher
                .send(health_status_up_message(&self.daemon_to_be_monitored))
                .await?;
        } else {
            bail!("Failed to receive any message");
        }
        Ok(())
    }

    pub async fn send_up_health_status(&mut self) -> Result<(), anyhow::Error> {
        Ok(self
            .mqtt_publisher
            .send(health_status_up_message(&self.daemon_to_be_monitored))
            .await?)
    }

    pub async fn send_down_health_status(&mut self) -> Result<(), anyhow::Error> {
        Ok(self
            .mqtt_publisher
            .send(health_status_down_message(&self.daemon_to_be_monitored))
            .await?)
    }
}

#[async_trait]
impl Actor for HealthMonitorActor {
    type MessageBox = SimpleMessageBox<MqttMessage, MqttMessage>;

    fn name(&self) -> &str {
        "HealthMonitorActor"
    }

    async fn run(mut self, mut messages: Self::MessageBox) -> Result<(), RuntimeError> {
        self.send_up_health_status().await?;
        while let Some(message) = messages.recv().await {
            {
                self.process_mqtt_message(message).await?;
            }
        }
        Ok(())
    }
}
