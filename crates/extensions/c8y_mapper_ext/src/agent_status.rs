use tedge_mqtt_ext::Message;

pub const TEDGE_AGENT_HEALTH_TOPIC: &str = "tedge/health/tedge-agent";
const TEDGE_HEALTH_UP_PAYLOAD: &str = r#""status":"up""#;
const TEDGE_HEALTH_DOWN_PAYLOAD: &str = r#""status":"down""#;

pub fn check_tedge_agent_status(message: &Message) -> bool {
    if message.topic.name.eq(TEDGE_AGENT_HEALTH_TOPIC) {
        match message.payload_str() {
            Ok(payload) => {
                payload.contains(TEDGE_HEALTH_UP_PAYLOAD)
                    || payload.contains(TEDGE_HEALTH_DOWN_PAYLOAD)
            }
            Err(_err) => false,
        }
    } else {
        false
    }
}
