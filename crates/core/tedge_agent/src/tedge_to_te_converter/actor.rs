use crate::tedge_to_te_converter::error::TedgetoTeConverterError;
use async_trait::async_trait;
use clock::Timestamp;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use tedge_actors::Actor;
use tedge_actors::MessageReceiver;
use tedge_actors::RuntimeError;
use tedge_actors::Sender;
use tedge_actors::SimpleMessageBox;
use tedge_mqtt_ext::MqttMessage;
use tedge_mqtt_ext::Topic;

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct ThinEdgeAlarmData {
    pub text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    #[serde(with = "time::serde::rfc3339::option")]
    pub time: Option<Timestamp>,

    #[serde(default)]
    pub severity: String,

    #[serde(flatten)]
    pub alarm_data: HashMap<String, Value>,
}

pub struct TedgetoTeConverterActor {
    messages: SimpleMessageBox<MqttMessage, MqttMessage>,
}

#[async_trait]
impl Actor for TedgetoTeConverterActor {
    fn name(&self) -> &str {
        "TedgetoTeConverterActor"
    }

    async fn run(&mut self) -> Result<(), RuntimeError> {
        while let Some(message) = self.messages.recv().await {
            {
                self.process_mqtt_message(message).await?;
            }
        }
        Ok(())
    }
}

impl TedgetoTeConverterActor {
    pub fn new(messages: SimpleMessageBox<MqttMessage, MqttMessage>) -> Self {
        Self { messages }
    }

    // Todo: Convert command topics
    //tedge/commands/res/software/list	te/<identifier>/cmd/software_list/<cmd_id>
    //tedge/commands/res/software/update	te/<identifier>/cmd/software_update/<cmd_id>
    //tedge/+/commands/res/config_snapshot	te/<identifier>/cmd/config_snapshot/<cmd_id>
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
                self.convert_alarm(message).await?;
            }
            topic if topic.name.starts_with("tedge/health") => {
                self.convert_health_status_message(message).await?;
            }
            _ => {}
        }
        Ok(())
    }

    // tedge/measurements -> te/device/main///m/
    // tedge/measurements/child -> te/device/child///m/
    async fn convert_measurement(
        &mut self,
        message: MqttMessage,
    ) -> Result<(), TedgetoTeConverterError> {
        let cid = get_child_id_from_measurement_topic(&message.topic.name)?;
        let te_topic = match cid {
            Some(cid) => Topic::new_unchecked(format!("te/device/{cid}///m/").as_str()),
            None => Topic::new_unchecked("te/device/main///m/"),
        };

        let msg = MqttMessage::new(&te_topic, message.payload_str()?).with_qos(message.qos);
        self.messages.send(msg).await?;
        Ok(())
    }

    // tedge/alarms/severity/alarm_type -> te/device/main///a/alarm_type, put severity in payload
    // tedge/alarms/severity/child/alarm_type ->  te/device/child///a/alarm_type, put severity in payload
    async fn convert_alarm(&mut self, message: MqttMessage) -> Result<(), TedgetoTeConverterError> {
        let (cid, alarm_type, severity) =
            get_child_id_and_alarm_type_and_severity(&message.topic.name)?;
        let mut alarm: ThinEdgeAlarmData = serde_json::from_str(message.payload_str()?)?;
        alarm.severity = severity.into();

        let te_topic = match cid {
            Some(cid) => Topic::new_unchecked(format!("te/device/{cid}///a/{alarm_type}").as_str()),
            None => Topic::new_unchecked(format!("te/device/main///a/{alarm_type}").as_str()),
        };

        let msg = MqttMessage::new(&te_topic, serde_json::to_string(&alarm)?).with_qos(message.qos);
        self.messages.send(msg).await?;
        Ok(())
    }

    // tedge/events/event_type -> te/device/main///e/event_type
    // tedge/events/child/event_type -> te/device/child///e/event_type
    async fn convert_event(&mut self, message: MqttMessage) -> Result<(), TedgetoTeConverterError> {
        let (cid, event_type) = get_child_id_and_event_type(&message.topic.name)?;
        let te_topic = match cid {
            Some(cid) => Topic::new_unchecked(format!("te/device/{cid}///e/{event_type}").as_str()),
            None => Topic::new_unchecked(format!("te/device/main///e/{event_type}").as_str()),
        };

        let msg = MqttMessage::new(&te_topic, message.payload_str().unwrap()).with_qos(message.qos);
        self.messages.send(msg).await?;
        Ok(())
    }

    // tedge/health/service-name -> te/device/main/service/<service-name>/status/health
    // tedge/health/child/service-name -> te/device/child/service/<service-name>/status/health
    async fn convert_health_status_message(
        &mut self,
        message: MqttMessage,
    ) -> Result<(), TedgetoTeConverterError> {
        let (cid, service_name) = get_child_id_and_service_name(&message.topic.name)?;
        let te_topic = match cid {
            Some(cid) => Topic::new_unchecked(
                format!("te/device/{cid}/service/{service_name}/status/health").as_str(),
            ),
            None => Topic::new_unchecked(
                format!("te/device/main/service/{service_name}/status/health").as_str(),
            ),
        };
        let msg = MqttMessage::new(&te_topic, message.payload_str()?).with_qos(message.qos);

        self.messages.send(msg).await?;
        Ok(())
    }
}

fn get_child_id_and_event_type(
    topic: &str,
) -> Result<(Option<&str>, &str), TedgetoTeConverterError> {
    let ts = topic.split('/').collect::<Vec<_>>();
    if ts.len() == 3 {
        if ts[2].is_empty() {
            Err(TedgetoTeConverterError::UnsupportedTopic(topic.into()))
        } else {
            Ok((None, ts[2]))
        }
    } else if ts.len() == 4 {
        if ts[2].is_empty() || ts[3].is_empty() {
            return Err(TedgetoTeConverterError::UnsupportedTopic(topic.into()));
        } else {
            Ok((Some(ts[2]), ts[3]))
        }
    } else {
        return Err(TedgetoTeConverterError::UnsupportedTopic(topic.into()));
    }
}

fn get_child_id_and_alarm_type_and_severity(
    topic: &str,
) -> Result<(Option<&str>, &str, &str), TedgetoTeConverterError> {
    let ts = topic.split('/').collect::<Vec<_>>();
    if ts.len() == 4 {
        if ts[3].is_empty() || ts[2].is_empty() {
            Err(TedgetoTeConverterError::UnsupportedTopic(topic.into()))
        } else {
            Ok((None, ts[3], ts[2]))
        }
    } else if ts.len() == 5 {
        if ts[3].is_empty() || ts[4].is_empty() || ts[2].is_empty() {
            Err(TedgetoTeConverterError::UnsupportedTopic(topic.into()))
        } else {
            Ok((Some(ts[3]), ts[4], ts[2]))
        }
    } else {
        return Err(TedgetoTeConverterError::UnsupportedTopic(topic.into()));
    }
}

fn get_child_id_from_measurement_topic(
    topic: &str,
) -> Result<Option<&str>, TedgetoTeConverterError> {
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

fn get_child_id_and_service_name(
    topic: &str,
) -> Result<(Option<&str>, &str), TedgetoTeConverterError> {
    let ts = topic.split('/').collect::<Vec<_>>();

    if ts.len() == 3 {
        if ts[2].is_empty() {
            Err(TedgetoTeConverterError::UnsupportedTopic(topic.into()))
        } else {
            Ok((None, ts[2]))
        }
    } else if ts.len() == 4 {
        if ts[3].is_empty() {
            Err(TedgetoTeConverterError::UnsupportedTopic(topic.into()))
        } else {
            Ok((Some(ts[2]), ts[3]))
        }
    } else {
        Err(TedgetoTeConverterError::UnsupportedTopic(topic.into()))
    }
}
