use tedge_actors::Builder;
use tedge_actors::DynSender;
use tedge_actors::LinkError;
use tedge_actors::MessageSink;
use tedge_actors::RuntimeRequest;
use tedge_actors::RuntimeRequestSink;
use tedge_actors::ServiceConsumer;
use tedge_actors::ServiceProvider;
use tedge_actors::SimpleMessageBoxBuilder;
use tedge_mqtt_ext::MqttConfig;
use tedge_mqtt_ext::MqttMessage;
use tedge_mqtt_ext::TopicFilter;

use super::actor::TedgetoTeConverterActor;

pub struct TedgetoTeConverterBuilder {
    service_name: String,
    box_builder: SimpleMessageBoxBuilder<MqttMessage, MqttMessage>,
}

impl TedgetoTeConverterBuilder {
    pub fn new(
        service_name: &str,
        mqtt: &mut (impl ServiceProvider<MqttMessage, MqttMessage, TopicFilter> + AsMut<MqttConfig>),
    ) -> Self {
        // Connect this actor to MQTT
        let subscriptions = vec!["tedge/measurements", "tedge/alarams", "tedge/events"]
            .try_into()
            .expect("Failed to create the tedge to te topic filter");

        let mut box_builder = SimpleMessageBoxBuilder::new(service_name, 16);
        box_builder
            .set_request_sender(mqtt.connect_consumer(subscriptions, box_builder.get_sender()));

        let builder = TedgetoTeConverterBuilder {
            service_name: service_name.to_owned(),
            box_builder,
        };

        builder
    }
}

impl RuntimeRequestSink for TedgetoTeConverterBuilder {
    fn get_signal_sender(&self) -> DynSender<RuntimeRequest> {
        Box::new(self.box_builder.get_signal_sender())
    }
}

impl Builder<TedgetoTeConverterActor> for TedgetoTeConverterBuilder {
    type Error = LinkError;

    fn try_build(self) -> Result<TedgetoTeConverterActor, Self::Error> {
        let message_box = self.box_builder.build();
        let actor = TedgetoTeConverterActor::new(self.service_name, message_box);

        Ok(actor)
    }
}
