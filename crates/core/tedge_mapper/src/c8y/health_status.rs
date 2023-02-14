use crate::core::error::ConversionError;
use c8y_api::smartrest::topic::SMARTREST_PUBLISH_TOPIC;
use mqtt_channel::Message;
use mqtt_channel::Topic;
use serde_json::Value;
use std::collections::HashMap;

pub fn convert_health_status_message(
    message: &Message,
    device_name: String,
) -> Result<Vec<Message>, ConversionError> {
    let mut mqtt_messages: Vec<Message> = Vec::new();
    let topic = message.topic.name.to_owned();

    let payload: HashMap<String, Value> = serde_json::from_str(message.payload_str()?)?;
    if payload.len() > 1 {
        let service_type = get_service_type(&payload);
        let status = get_health_status(&payload);
        let service_name = get_service_name(&topic);
        let service_external_id = get_service_id(device_name, service_name, &topic);
        let monitor_message =
            format!("102,{service_external_id},{service_type},{service_name},{status}");
        let topic = Topic::new_unchecked(&get_c8y_health_topic(&topic)?);
        let alarm_copy = Message::new(&topic, monitor_message.as_bytes().to_owned()).with_retain();
        mqtt_messages.push(alarm_copy);
    }
    Ok(mqtt_messages)
}

pub(crate) fn get_c8y_health_topic(topic: &str) -> Result<String, ConversionError> {
    let topic_split: Vec<&str> = topic.split('/').collect();
    if topic_split.len() == 3 {
        Ok(SMARTREST_PUBLISH_TOPIC.to_string())
    } else if topic_split.len() == 4 {
        Ok(format!("{SMARTREST_PUBLISH_TOPIC}/{}", topic_split[2]))
    } else {
        Err(ConversionError::UnsupportedTopic(topic.to_string()))
    }
}

pub fn get_health_status(payload: &HashMap<String, Value>) -> String {
    let status = &payload["status"];
    status.to_string()
}

pub fn get_service_type(payload: &HashMap<String, Value>) -> String {
    let s_type = payload.get("type");
    match s_type {
        Some(t) => t.to_string(),
        None => "thin-edge.io".to_string(),
    }
}

pub fn get_service_name(topic: &str) -> &str {
    let topic_split: Vec<&str> = topic.split('/').collect();
    if topic_split.len() == 4 {
        topic_split[3]
    } else {
        topic_split[2]
    }
}

pub fn get_service_id(device_name: String, service_name: &str, topic: &str) -> String {
    let topic_split: Vec<&str> = topic.split('/').collect();
    if topic_split.len() == 4 {
        format!("{device_name}_{}_{service_name}", topic_split[2])
    } else {
        format!("{device_name}_{service_name}")
    }
}
