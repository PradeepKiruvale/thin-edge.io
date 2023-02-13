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
        let status = get_health_status(&payload)?;
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

fn get_c8y_health_topic(topic: &str) -> Result<String, ConversionError> {
    let topic_split: Vec<&str> = topic.split('/').collect();
    if topic_split.len() == 3 {
        Ok(SMARTREST_PUBLISH_TOPIC.to_string())
    } else if topic_split.len() == 4 {
        Ok(format!("{SMARTREST_PUBLISH_TOPIC}/{}", topic_split[2]))
    } else {
        Err(ConversionError::UnsupportedTopic(topic.to_string()))
    }
}

fn get_health_status(payload: &HashMap<String, Value>) -> Result<String, ConversionError> {
    let status = payload.get("status");
    match status {
        Some(s) => Ok(s.to_string()),
        None => Err(ConversionError::HealthStatus),
    }
}

fn get_service_type(payload: &HashMap<String, Value>) -> String {
    let s_type = payload.get("type");
    match s_type {
        Some(t) => t.to_string(),
        None => "thin-edge.io".to_string(),
    }
}

fn get_service_name(topic: &str) -> &str {
    let topic_split: Vec<&str> = topic.split('/').collect();
    if topic_split.len() == 4 {
        topic_split[3]
    } else {
        topic_split[2]
    }
}

fn get_service_id(device_name: String, service_name: &str, topic: &str) -> String {
    let topic_split: Vec<&str> = topic.split('/').collect();
    if topic_split.len() == 4 {
        format!("{device_name}_{}_{service_name}", topic_split[2])
    } else {
        format!("{device_name}_{service_name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_case::test_case;
    #[test_case(
        "test_device",
        "tedge/health/tedge-mapper-c8y",
        r#"{"pid":"1234","type":"systemd","status":"up"}"#,
        "c8y/s/us",
        r#"102,test_device_tedge-mapper-c8y,"systemd",tedge-mapper-c8y,"up""#;        
        "service-monitoring-thin-edge-device"
    )]
    #[test_case(
        "test_device",
        "tedge/health/child/tedge-mapper-c8y",
        r#"{"pid":"1234","type":"systemd","status":"up"}"#,
        "c8y/s/us/child",
        r#"102,test_device_child_tedge-mapper-c8y,"systemd",tedge-mapper-c8y,"up""#;        
        "service-monitoring-thin-edge-child-device"
    )]
    #[test_case(
        "test_device",
        "tedge/health/tedge-mapper-c8y",
        r#"{"pid":"1234","type":"systemd"}"#,
        "c8y/s/us",
        r#"102,test_device_tedge-mapper-c8y,"systemd",tedge-mapper-c8y,"up""#;        
        "service-monitoring-thin-edge-no-status"
    )]
    fn translate_health_status_to_c8y_service_monitoring_message(
        device_name: &str,
        health_topic: &str,
        health_payload: &str,
        c8y_monitor_topic: &str,
        c8y_monitor_payload: &str,
    ) {
        let topic = Topic::new_unchecked(&health_topic);
        let health_message = Message::new(&topic, health_payload.as_bytes().to_owned());
        let expected_message = Message::new(
            &Topic::new_unchecked(&c8y_monitor_topic),
            c8y_monitor_payload.as_bytes(),
        )
        .with_retain();
        match convert_health_status_message(&health_message, device_name.into()) {
            Ok(msg) => {
                dbg!(msg[0].payload_str());
                dbg!(expected_message.payload_str());
                assert_eq!(msg[0], expected_message);
            }
            Err(e) => {
                assert_eq!(e.to_string(), "Failed to extract the health status")
            }
        }
    }
}
