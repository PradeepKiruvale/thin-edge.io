use crate::tedge_to_te_converter::error::TedgetoTeConverterError;
use async_trait::async_trait;
use tedge_actors::Actor;
use tedge_actors::MessageReceiver;
use tedge_actors::RuntimeError;
use tedge_actors::SimpleMessageBox;
use tedge_mqtt_ext::MqttMessage;

pub struct TedgetoTeConverterActor {
    daemon_name: String,
    messages: SimpleMessageBox<MqttMessage, MqttMessage>,
}

impl TedgetoTeConverterActor {
    pub fn new(daemon_name: String, messages: SimpleMessageBox<MqttMessage, MqttMessage>) -> Self {
        Self {
            daemon_name,
            messages,
        }
    }

    async fn process_mqtt_message(
        &mut self,
        message: MqttMessage,
    ) -> Result<(), TedgetoTeConverterError> {
        match message.topic {
            topic if topic.name.starts_with("tedge/measurements") => Self::convert_measurement(),
            _ => unreachable!(),
        }
        Ok(())
    }

    fn convert_measurement() {
        println!("inside the convert measurement");
    }
}

#[async_trait]
impl Actor for TedgetoTeConverterActor {
    fn name(&self) -> &str {
        "TedgetoTeConverterActor"
    }

    async fn run(&mut self) -> Result<(), RuntimeError> {
        while let Some(message) = self.messages.recv().await {
            {
                println!("inside process message");
                self.process_mqtt_message(message).await;
            }
        }
        Ok(())
    }
}
