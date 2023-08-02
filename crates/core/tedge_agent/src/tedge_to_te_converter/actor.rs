use crate::tedge_to_te_converter::error::TedgetoTeConverterError;
use async_trait::async_trait;
use tedge_actors::Actor;
use tedge_actors::MessageReceiver;
use tedge_actors::RuntimeError;
use tedge_actors::Sender;
use tedge_actors::SimpleMessageBox;
use tedge_mqtt_ext::MqttMessage;
use tedge_mqtt_ext::Topic;

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

    // Must map tedge/topic-> te/topic
    // tedge/measurements -> te/device/main///m/
    // tedge/measurements/child -> te/device/child///m/
    // tedge/alarms/severity/alarm_type -> te/device/main///a/alarm_type, put severity in payload
    // tedge/alarms/severity/child/alarm_type ->  te/device/child///a/alarm_type, put severity in payload
    // tedge/events/event_type -> te/device/main///e/event_type
    // tedge/events/child/event_type -> te/device/child///e/event_type
    // tedge/health/service-name -> te/device/main/service/<service-name>/status/health
    // tedge/health/child/service-name -> te/device/child/service/<service-name>/status/health
    // tedge/health-check/service-name -> te/device/child/service/<service-name>/status/health/check
    // tedge/health-check/child/service-name -> te/device/child/service/<service-name>/status/health/check
    async fn process_mqtt_message(
        &mut self,
        message: MqttMessage,
    ) -> Result<(), TedgetoTeConverterError> {
        match message.topic.clone() {
            topic if topic.name.starts_with("tedge/measurements") => {
                self.convert_measurement(message).await;
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    
    async fn convert_measurement(&mut self, message: MqttMessage) {
        println!("inside the convert measurement");

        let te_topic = Topic::new_unchecked(format!("te/device/main///m/").as_str());

        let msg = MqttMessage::new(&te_topic, message.payload_str().unwrap()).with_qos(message.qos);

        self.messages.send(msg).await;
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
