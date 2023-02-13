use crate::core::error::ConversionError;
use c8y_api::smartrest::topic::SMARTREST_PUBLISH_TOPIC;
use mqtt_channel::Message;
use mqtt_channel::Topic;

pub fn convert_health_status_message(message: &Message) -> Result<Vec<Message>, ConversionError> {
    let mut mqtt_messages: Vec<Message> = Vec::new();
    let topic = message.topic.name.to_owned();
    dbg!(&topic);

    //form a smartrest message
    //get device id
    // get status
    // get service type
    // get service name
    // form a new payload

    let service_type = "debian";
    let dev_name = "childops_test";

    if let Some(status) = get_health_status(message.payload_str()?) {
        let service_name = get_service_name(&topic);
        let monitor_message =
            format!("102,{dev_name}_{service_name},{service_type},{service_name},{status}");

        let topic = Topic::new_unchecked(&get_c8y_health_topic(&topic)?);

        dbg!(&topic);

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

pub fn get_health_status(payload: &str) -> Option<String> {
    let payload_split: Vec<&str> = payload.split([',', ':']).collect();
    if payload_split.len() >= 2 {
        let status = payload_split[3];
        dbg!(&status.len());
        let s = if status.len() == 7 {
            &status[1..status.len() - 2]
        } else {
            &status[1..status.len() - 1]
        };
        Some(s.to_string())
    } else {
        None
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
