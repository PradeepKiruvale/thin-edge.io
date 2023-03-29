use async_trait::async_trait;
use clock::WallClock;
use tedge_actors::Actor;
use tedge_actors::MessageReceiver;
use tedge_actors::RuntimeError;
use tedge_actors::SimpleMessageBox;
use tedge_mapper_core::size_threshold::SizeThreshold;
use tedge_mqtt_ext::MqttMessage;

use crate::converter::AzureConverter;

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
        let clock = Box::new(WallClock);
        let size_threshold = SizeThreshold(255 * 1024);
        let mut converter = Box::new(AzureConverter::new(
            self.add_time_stamp,
            clock,
            size_threshold,
        ));

        while let Some(message) = self.message_box.recv().await {
            {
                let converted_messages = converter.convert(&message).await;
                for converted_message in converted_messages.into_iter() {
                    let _ = self.message_box.send(converted_message).await;
                }
            }
        }

        Ok(())
    }
}
