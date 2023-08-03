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
    // tedge/health-check/service-name -> te/device/main/service/<service-name>/cmd/health/check
    // tedge/health-check/child/service-name -> te/device/child/service/<service-name>/cmd/health/check
    // tedge/commands/res/software/list	te/<identifier>/cmd/software_list/<cmd_id>
    // tedge/commands/res/software/update	te/<identifier>/cmd/software_update/<cmd_id>
    // tedge/+/commands/res/config_snapshot	te/<identifier>/cmd/config_snapshot/<cmd_id>
    //tedge/+/commands/res/config_update	te/<identifier>/cmd/config_update/<cmd_id>
    //tedge/+/commands/res/firmware_update	te/<identifier>/cmd/firmware_update/<cmd_id>
    //tedge/commands/res/control/restart	te/<identifier>/cmd/restart/<cmd_id>

    async fn process_mqtt_message(
        &mut self,
        message: MqttMessage,
    ) -> Result<(), TedgetoTeConverterError> {
        match message.topic.clone() {
            topic if topic.name.starts_with("tedge/measurements") => {
                self.convert_measurement(message).await?;
            }
            topic if topic.name.starts_with("tedge/events") => {
                self.convert_event(message).await?;
            }
            topic if topic.name.starts_with("tedge/alarms") => {
                self.convert_measurement(message).await;
            }
            topic if topic.name.starts_with("tedge/health") => {
                self.convert_measurement(message).await;
            }
            _ => unreachable!(),
        }
        Ok(())
    }

    async fn convert_measurement(
        &mut self,
        message: MqttMessage,
    ) -> Result<(), TedgetoTeConverterError> {
        println!("inside the convert measurement");
        let cid = get_child_id(&message.topic.name)?;
        let te_topic = match cid {
            Some(cid) => Topic::new_unchecked(format!("te/device/{cid}///m/").as_str()),
            None => Topic::new_unchecked(format!("te/device/main///m/").as_str()),
        };

        let msg = MqttMessage::new(&te_topic, message.payload_str().unwrap()).with_qos(message.qos);

        self.messages.send(msg).await?;
        Ok(())
    }

    async fn convert_alarm(&mut self, message: MqttMessage) -> Result<(), TedgetoTeConverterError> {
        println!("inside the convert alarm");
        let te_topic = Topic::new_unchecked(format!("te/device/main///a/").as_str());
        let msg = MqttMessage::new(&te_topic, message.payload_str().unwrap()).with_qos(message.qos);
        self.messages.send(msg).await?;
        Ok(())
    }

    async fn convert_event(&mut self, message: MqttMessage) -> Result<(), TedgetoTeConverterError> {
        println!("inside the convert event");
        let (cid, event_type) = get_child_id_and_event_type(&message.topic.name)?;
        let te_topic = match cid {
            Some(cid) => Topic::new_unchecked(format!("te/device/{cid}///e/{event_type}").as_str()),
            None => Topic::new_unchecked(format!("te/device/main///e/{event_type}").as_str()),
        };

        let msg = MqttMessage::new(&te_topic, message.payload_str().unwrap()).with_qos(message.qos);

        self.messages.send(msg).await?;
        Ok(())
    }

    async fn convert_health_status_message(&mut self, message: MqttMessage) {
        println!("inside the convert measurement");

        let te_topic = Topic::new_unchecked(format!("te/device/main///m/").as_str());

        let msg = MqttMessage::new(&te_topic, message.payload_str().unwrap()).with_qos(message.qos);

        self.messages.send(msg).await;
    }

    async fn convert_cmd(&mut self, message: MqttMessage) {
        println!("inside the convert measurement");

        let te_topic =
            Topic::new_unchecked(format!("te/device/main///cmd/cmd-type/cmd-id/").as_str());

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

fn get_child_id_and_event_type(
    topic: &str,
) -> Result<(Option<&str>, &str), TedgetoTeConverterError> {
    let ts = topic.split('/').collect::<Vec<_>>();
    if ts.len() == 3 {
        if ts[2].is_empty() {
            return Err(TedgetoTeConverterError::UnsupportedTopic(topic.into()));
        } else {
            Ok((None, ts[2]))
        }
    } else if ts.len() == 4 {
        if ts[2].is_empty() || ts[3].is_empty() {
            return Err(TedgetoTeConverterError::UnsupportedTopic(topic.into()));
        } else {
            Ok((Some(ts[3]), ts[2]))
        }
    } else {
        return Err(TedgetoTeConverterError::UnsupportedTopic(topic.into()));
    }
}

fn get_child_id(topic: &str) -> Result<Option<&str>, TedgetoTeConverterError> {
    let ts = topic.split('/').collect::<Vec<_>>();
    if ts.len() == 2 {
        Ok(None)
    } else if ts.len() == 3 {
        if ts[2].is_empty() {
            return Err(TedgetoTeConverterError::UnsupportedTopic(topic.into()));
        } else {
            Ok(Some(ts[2]))
        }
    } else {
        return Err(TedgetoTeConverterError::UnsupportedTopic(topic.into()));
    }
}
