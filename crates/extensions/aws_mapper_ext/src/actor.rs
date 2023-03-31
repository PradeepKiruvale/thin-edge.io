use async_trait::async_trait;
use clock::WallClock;
use tedge_actors::Actor;
use tedge_actors::MessageReceiver;
use tedge_actors::RuntimeError;
use tedge_actors::SimpleMessageBox;
use tedge_mqtt_ext::MqttMessage;

pub struct AwsMapperActor {
    add_time_stamp: bool,
    message_box: SimpleMessageBox<MqttMessage, MqttMessage>,
}

impl AwsMapperActor {
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
impl Actor for AwsMapperActor {
    fn name(&self) -> &str {
        "AwsMapperActor"
    }

    async fn run(mut self) -> Result<(), RuntimeError> {
        let clock = Box::new(WallClock);
        // Quotas at: https://docs.aws.amazon.com/general/latest/gr/iot-core.html#limits_iot
        // let size_threshold = SizeThreshold(128 * 1024);
        // let mut converter = Box::new(AwsConverter::new(
        //     self.add_time_stamp,
        //     clock,
        //     size_threshold,
        // ));

        while let Some(message) = self.message_box.recv().await {
            {
                // let converted_messages = converter.convert(&message).await;
                // for converted_message in converted_messages.into_iter() {
                //     let _ = self.message_box.send(converted_message).await;
                // }
            }
        }

        Ok(())
    }
}
