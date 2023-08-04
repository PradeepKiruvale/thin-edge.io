use crate::tedge_to_te_converter::builder::TedgetoTeConverterBuilder;
use serde_json::json;
use std::time::Duration;
use tedge_actors::test_helpers::MessageReceiverExt;
use tedge_actors::test_helpers::TimedMessageBox;
use tedge_actors::Actor;
use tedge_actors::Builder;
use tedge_actors::DynError;
use tedge_actors::Sender;
use tedge_actors::SimpleMessageBox;
use tedge_actors::SimpleMessageBoxBuilder;
use tedge_mqtt_ext::MqttMessage;
use tedge_mqtt_ext::Topic;

const TEST_TIMEOUT_MS: Duration = Duration::from_millis(5000);

#[tokio::test]
async fn convert_incoming_main_device_measurement_topic() -> Result<(), DynError> {
    // Spawn incoming mqtt message converter
    let mut mqtt_box = spawn_tedge_to_te_converter().await?;

    // Simulate SoftwareList MQTT message received.
    let mqtt_message = MqttMessage::new(
        &Topic::new_unchecked("tedge/measurements"),
        r#"{"temperature": 2500 }"#,
    );
    let expected_mqtt_message = MqttMessage::new(
        &Topic::new_unchecked("te/device/main///m/"),
        r#"{"temperature": 2500 }"#,
    );
    mqtt_box.send(mqtt_message).await?;

    // Assert SoftwareListRequest
    mqtt_box.assert_received([expected_mqtt_message]).await;
    Ok(())
}

#[tokio::test]
async fn convert_incoming_child_device_measurement_topic() -> Result<(), DynError> {
    // Spawn incoming mqtt message converter
    let mut mqtt_box = spawn_tedge_to_te_converter().await?;

    // Simulate SoftwareList MQTT message received.
    let mqtt_message = MqttMessage::new(
        &Topic::new_unchecked("tedge/measurements/child1"),
        r#"{"temperature": 2500 }"#,
    );
    let expected_mqtt_message = MqttMessage::new(
        &Topic::new_unchecked("te/device/child1///m/"),
        r#"{"temperature": 2500 }"#,
    );
    mqtt_box.send(mqtt_message).await?;

    // Assert SoftwareListRequest
    mqtt_box.assert_received([expected_mqtt_message]).await;
    Ok(())
}

#[tokio::test]
async fn convert_incoming_main_device_alarm_topic() -> Result<(), DynError> {
    // Spawn incoming mqtt message converter
    let mut mqtt_box = spawn_tedge_to_te_converter().await?;

    // Simulate SoftwareList MQTT message received.
    let mqtt_message = MqttMessage::new(
        &Topic::new_unchecked("tedge/alarms/critical/MyCustomAlarm"),
        r#"{
            "text": "I raised it",
            "time": "2021-04-23T19:00:00+05:00"
        }"#,
    );

    let expected_mqtt_message = MqttMessage::new(
        &Topic::new_unchecked("te/device/main///a/MyCustomAlarm"),
        r#"{"text":"I raised it","time":"2021-04-23T19:00:00+05:00","severity":"critical"}"#,
    );

    mqtt_box.send(mqtt_message).await?;

    // Assert SoftwareListRequest
    mqtt_box.assert_received([expected_mqtt_message]).await;
    Ok(())
}

#[tokio::test]
async fn convert_incoming_custom_main_device_alarm_topic() -> Result<(), DynError> {
    // Spawn incoming mqtt message converter
    let mut mqtt_box = spawn_tedge_to_te_converter().await?;

    // Simulate SoftwareList MQTT message received.
    let mqtt_message = MqttMessage::new(
        &Topic::new_unchecked("tedge/alarms/critical/MyCustomAlarm"),
        r#"{
            "text": "I raised it",
            "time": "2021-04-23T19:00:00+05:00",
            "someOtherCustomFragment": {"nested":{"value": "extra info"}}
        }"#,
    );

    let expected_mqtt_message = MqttMessage::new(
        &Topic::new_unchecked("te/device/main///a/MyCustomAlarm"),
        r#"{"text":"I raised it","time":"2021-04-23T19:00:00+05:00","severity":"critical","someOtherCustomFragment":{"nested":{"value":"extra info"}}}"#,
    );

    mqtt_box.send(mqtt_message).await?;

    // Assert SoftwareListRequest
    mqtt_box.assert_received([expected_mqtt_message]).await;
    Ok(())
}

#[tokio::test]
async fn convert_incoming_child_device_alarm_topic() -> Result<(), DynError> {
   // Spawn incoming mqtt message converter
   let mut mqtt_box = spawn_tedge_to_te_converter().await?;

   // Simulate SoftwareList MQTT message received.
   let mqtt_message = MqttMessage::new(
       &Topic::new_unchecked("tedge/alarms/critical/child/MyCustomAlarm"),
       r#"{
           "text": "I raised it",
           "time": "2021-04-23T19:00:00+05:00"
       }"#,
   );

   let expected_mqtt_message = MqttMessage::new(
       &Topic::new_unchecked("te/device/child///a/MyCustomAlarm"),
       r#"{"text":"I raised it","time":"2021-04-23T19:00:00+05:00","severity":"critical"}"#,
   );

   mqtt_box.send(mqtt_message).await?;

   // Assert SoftwareListRequest
   mqtt_box.assert_received([expected_mqtt_message]).await;
   Ok(())
}


#[tokio::test]
async fn convert_incoming_main_device_event_topic() -> Result<(), DynError> {
    // Spawn incoming mqtt message converter
    let mut mqtt_box = spawn_tedge_to_te_converter().await?;

    // Simulate SoftwareList MQTT message received.
    let mqtt_message = MqttMessage::new(
        &Topic::new_unchecked("tedge/events/MyEvent"),
        r#"{"text":"Some test event","time":"2021-04-23T19:00:00+05:00"}"#,
    );

    let expected_mqtt_message = MqttMessage::new(
        &Topic::new_unchecked("te/device/main///e/MyEvent"),
        r#"{"text":"Some test event","time":"2021-04-23T19:00:00+05:00"}"#,
    );

    mqtt_box.send(mqtt_message).await?;

    // Assert SoftwareListRequest
    mqtt_box.assert_received([expected_mqtt_message]).await;
    Ok(())
}

#[tokio::test]
async fn convert_custom_incoming_main_device_event_topic() -> Result<(), DynError> {
    // Spawn incoming mqtt message converter
    let mut mqtt_box = spawn_tedge_to_te_converter().await?;

    // Simulate SoftwareList MQTT message received.
    let mqtt_message = MqttMessage::new(
        &Topic::new_unchecked("tedge/events/MyEvent"),
        r#"{"text":"Some test event","time":"2021-04-23T19:00:00+05:00","someOtherCustomFragment":{"nested":{"value":"extra info"}}}"#,
    );

    let expected_mqtt_message = MqttMessage::new(
        &Topic::new_unchecked("te/device/main///e/MyEvent"),
        r#"{"text":"Some test event","time":"2021-04-23T19:00:00+05:00","someOtherCustomFragment":{"nested":{"value":"extra info"}}}"#,
    );

    mqtt_box.send(mqtt_message).await?;

    // Assert SoftwareListRequest
    mqtt_box.assert_received([expected_mqtt_message]).await;
    Ok(())
}

#[tokio::test]
async fn convert_incoming_child_device_event_topic() -> Result<(), DynError> {
    // Spawn incoming mqtt message converter
    let mut mqtt_box = spawn_tedge_to_te_converter().await?;

    // Simulate SoftwareList MQTT message received.
    let mqtt_message = MqttMessage::new(
        &Topic::new_unchecked("tedge/events/child/MyEvent"),
        r#"{"text":"Some test event","time":"2021-04-23T19:00:00+05:00"}"#,
    );

    let expected_mqtt_message = MqttMessage::new(
        &Topic::new_unchecked("te/device/child///e/MyEvent"),
        r#"{"text":"Some test event","time":"2021-04-23T19:00:00+05:00"}"#,
    );

    mqtt_box.send(mqtt_message).await?;

    // Assert SoftwareListRequest
    mqtt_box.assert_received([expected_mqtt_message]).await;
    Ok(())
}

async fn spawn_tedge_to_te_converter(
) -> Result<TimedMessageBox<SimpleMessageBox<MqttMessage, MqttMessage>>, DynError> {
    let mut mqtt_builder: SimpleMessageBoxBuilder<MqttMessage, MqttMessage> =
        SimpleMessageBoxBuilder::new("MQTT", 5);

    let converter_actor_builder = TedgetoTeConverterBuilder::new("Test builder", &mut mqtt_builder);
    let mqtt_message_box = mqtt_builder.build().with_timeout(TEST_TIMEOUT_MS);

    let mut converter_actor = converter_actor_builder.build();
    tokio::spawn(async move { converter_actor.run().await });

    Ok(mqtt_message_box)
}
