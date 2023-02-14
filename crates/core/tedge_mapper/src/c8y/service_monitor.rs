use c8y_api::smartrest::topic::SMARTREST_PUBLISH_TOPIC;
use mqtt_channel::Message;
use mqtt_channel::Topic;
use serde_json::json;
use serde_json::Value;
use std::collections::HashMap;

pub fn convert_health_status_message(message: &Message, device_name: String) -> Vec<Message> {
    let mut mqtt_messages: Vec<Message> = Vec::new();
    let topic = message.topic.name.to_owned();
    let service_name = get_service_name(&topic);
    let child_id = get_child_id(&topic);

    // If not Bridge health status
    if !service_name.contains("mosquitto-c8y-bridge") {
        let payload_str = message
            .payload_str()
            .unwrap_or(r#""type":"thin-edge.io","status":"down""#);

        let payload: HashMap<String, Value> =
            serde_json::from_str(payload_str).unwrap_or_else(|_| {
                HashMap::from([
                    ("type".into(), json!("thin-edge.io")),
                    ("status".into(), json!("down")),
                ])
            });

        let service_type = get_service_type(&payload);
        let status = get_health_status(&payload);

        let status_message = service_monitor_status_message(
            &device_name,
            service_name,
            &status,
            &service_type,
            child_id,
        );

        mqtt_messages.push(status_message);
    }

    mqtt_messages
}

fn get_health_status(payload: &HashMap<String, Value>) -> String {
    let status = payload.get("status");
    match status {
        Some(s) => s.to_string(),
        None => r#""down""#.to_string(),
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

fn get_child_id(topic: &str) -> Option<String> {
    let topic_split: Vec<&str> = topic.split('/').collect();
    if topic_split.len() == 4 {
        Some(topic_split[2].to_owned())
    } else {
        None
    }
}

pub fn service_monitor_status_message(
    device_name: &str,
    daemon_name: &str,
    status: &str,
    service_type: &str,
    child_id: Option<String>,
) -> Message {
    match child_id {
        Some(cid) => Message {
            topic: Topic::new_unchecked(&format!("{SMARTREST_PUBLISH_TOPIC}/{cid}")),
            payload: format!(
                "102,{device_name}_{cid}_{daemon_name},{service_type},{daemon_name},{status}"
            )
            .into_bytes(),
            qos: mqtt_channel::QoS::AtLeastOnce,
            retain: true,
        },
        None => Message {
            topic: Topic::new_unchecked(SMARTREST_PUBLISH_TOPIC),
            payload: format!(
                "102,{device_name}_{daemon_name},{service_type},{daemon_name},{status}"
            )
            .into_bytes(),
            qos: mqtt_channel::QoS::AtLeastOnce,
            retain: true,
        },
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
        r#"102,test_device_tedge-mapper-c8y,"systemd",tedge-mapper-c8y,"down""#;        
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

        let msg = convert_health_status_message(&health_message, device_name.into());

        dbg!(&msg[0].payload_str().unwrap());
        dbg!(&expected_message.payload_str().unwrap());
        assert_eq!(msg[0], expected_message);
    }
}
