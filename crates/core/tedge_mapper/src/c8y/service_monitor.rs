use c8y_api::smartrest::topic::SMARTREST_PUBLISH_TOPIC;
use mqtt_channel::Message;
use mqtt_channel::Topic;
use serde::Deserialize;
use serde::Serialize;

#[derive(Deserialize, Serialize, Debug)]
pub struct HealthStatus {
    #[serde(rename = "type", default = "default_type")]
    pub service_type: String,

    #[serde(default = "default_status")]
    pub status: String,
}

fn default_type() -> String {
    "thin-edge.io".to_string()
}

fn default_status() -> String {
    "down".to_string()
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TopicInfo {
    pub service_name: String,
    pub child_id: Option<String>,
}

impl TopicInfo {
    fn parse_topic_info(topic: &str) -> Self {
        let topic_split: Vec<&str> = topic.split('/').collect();
        let service_name = if topic_split.len() == 4 {
            topic_split[3]
        } else {
            topic_split[2]
        }
        .to_string();

        let child_id = if topic_split.len() == 4 {
            Some(topic_split[2].to_owned())
        } else {
            None
        };

        Self {
            service_name,
            child_id,
        }
    }
}
pub fn convert_health_status_message(message: &Message, device_name: String) -> Vec<Message> {
    let mut mqtt_messages: Vec<Message> = Vec::new();
    let topic = message.topic.name.to_owned();
    let topic_info = TopicInfo::parse_topic_info(&topic);

    // If not Bridge health status
    if !topic_info.service_name.contains("bridge") {
        let payload_str = message
            .payload_str()
            .unwrap_or(r#""type":"thin-edge.io","status":"down""#);

        let health_status = serde_json::from_str(payload_str).unwrap_or_else(|_| HealthStatus {
            service_type: "unknown".to_string(),
            status: "down".to_string(),
        });

        let status_message = service_monitor_status_message(
            &device_name,
            &topic_info.service_name,
            &health_status.status,
            &health_status.service_type,
            topic_info.child_id,
        );

        mqtt_messages.push(status_message);
    }

    mqtt_messages
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
        r#"102,test_device_tedge-mapper-c8y,systemd,tedge-mapper-c8y,up"#;        
        "service-monitoring-thin-edge-device"
    )]
    #[test_case(
        "test_device",
        "tedge/health/child/tedge-mapper-c8y",
        r#"{"pid":"1234","type":"systemd","status":"up"}"#,
        "c8y/s/us/child",
        r#"102,test_device_child_tedge-mapper-c8y,systemd,tedge-mapper-c8y,up"#;
        "service-monitoring-thin-edge-child-device"
    )]
    #[test_case(
        "test_device",
        "tedge/health/tedge-mapper-c8y",
        r#"{"pid":"123456","type":"systemd"}"#,
        "c8y/s/us",
        r#"102,test_device_tedge-mapper-c8y,systemd,tedge-mapper-c8y,down"#;
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

        assert_eq!(msg[0], expected_message);
    }
}
