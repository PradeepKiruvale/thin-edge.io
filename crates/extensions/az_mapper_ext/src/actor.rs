use async_trait::async_trait;
use clock::WallClock;
use tedge_actors::Actor;
use tedge_actors::MessageReceiver;
use tedge_actors::RuntimeError;
use tedge_actors::SimpleMessageBox;
use tedge_mqtt_ext::MqttMessage;
use tedge_actors::Sender;

pub struct AzMapperActor {
    add_time_stamp: bool,
    message_box: SimpleMessageBox<MqttMessage, MqttMessage>,
}

impl AzMapperActor {
    pub fn new(
        add_time_stamp: bool,
        message_box: SimpleMessageBox<MqttMessage, MqttMessage>,
    ) -> Self {
        Self {
            add_time_stamp,
            message_box,
        }
    }
}

#[async_trait]
impl Actor for AzMapperActor {
    fn name(&self) -> &str {
        "AzMapperActor"
    }

    async fn run(mut self) -> Result<(), RuntimeError> {
        while let Some(message) = self.message_box.recv().await {
            {
                let _ = self.message_box.send(message).await;
            }
        }

        Ok(())
    }
}
