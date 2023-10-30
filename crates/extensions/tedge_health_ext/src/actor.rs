use async_trait::async_trait;
use tedge_actors::Actor;
use tedge_actors::MessageReceiver;
use tedge_actors::RuntimeError;
use tedge_actors::Sender;
use tedge_actors::SimpleMessageBox;
use tedge_api::health::ServiceHealthTopic;
use tedge_mqtt_ext::Message;
use tedge_mqtt_ext::MqttMessage;

pub struct HealthMonitorActor {
    // TODO(marcel): move this
    service_registration_message: Option<Message>,
    health_topic: ServiceHealthTopic,
    messages: SimpleMessageBox<MqttMessage, MqttMessage>,
    topic_root: String,
}

impl HealthMonitorActor {
    pub fn new(
        service_registration_message: Option<Message>,
        health_topic: ServiceHealthTopic,
        messages: SimpleMessageBox<MqttMessage, MqttMessage>,
        topic_root: String,
    ) -> Self {
        Self {
            service_registration_message,
            health_topic,
            messages,
            topic_root,
        }
    }

    pub fn up_health_status(&self) -> MqttMessage {
        self.health_topic.up_message(&self.topic_root)
    }

    pub fn down_health_status(&self) -> MqttMessage {
        self.health_topic.down_message()
    }
}

#[async_trait]
impl Actor for HealthMonitorActor {
    fn name(&self) -> &str {
        "HealthMonitorActor"
    }

    async fn run(mut self) -> Result<(), RuntimeError> {
        if let Some(registration_message) = &self.service_registration_message {
            self.messages.send(registration_message.clone()).await?;
        }

        self.messages.send(self.up_health_status()).await?;

        while let Some(_message) = self.messages.recv().await {
            self.messages.send(self.up_health_status()).await?;
        }
        Ok(())
    }
}
