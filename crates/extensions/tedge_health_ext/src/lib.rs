mod actor;
use actor::HealthMonitorActor;
use tedge_actors::Builder;
use tedge_actors::DynSender;
use tedge_actors::LinkError;
use tedge_actors::MessageSink;
use tedge_actors::MessageSource;
use tedge_actors::NoConfig;
use tedge_actors::RuntimeRequest;
use tedge_actors::RuntimeRequestSink;
use tedge_actors::ServiceConsumer;
use tedge_actors::SimpleMessageBox;
use tedge_actors::SimpleMessageBoxBuilder;
use tedge_mqtt_ext::MqttMessage;
use tedge_mqtt_ext::TopicFilter;

type HealthInputMessage = MqttMessage;
type HealthOutputMessage = MqttMessage;

type HealthMonitorMessageBox = SimpleMessageBox<HealthInputMessage, HealthOutputMessage>;

pub struct HealthMonitorBuilder {
    subscriptions: TopicFilter,
    box_builder: SimpleMessageBoxBuilder<MqttMessage, MqttMessage>,
}

impl HealthMonitorBuilder {
    pub fn new(name: &str) -> Self {
        let subscriptions = vec!["tedge/health-check", &format!("tedge/health-check/{name}")]
            .try_into()
            .expect("Failed to create the HealthMonitorActor topicfilter");
        HealthMonitorBuilder {
            subscriptions,
            box_builder: SimpleMessageBoxBuilder::new(name, 16),
        }
    }
}

impl MessageSource<MqttMessage, NoConfig> for HealthMonitorBuilder {
    fn register_peer(&mut self, _config: NoConfig, sender: DynSender<MqttMessage>) {
        self.box_builder.set_request_sender(sender);
    }
}

impl MessageSink<MqttMessage> for HealthMonitorBuilder {
    fn get_sender(&self) -> DynSender<MqttMessage> {
        self.box_builder.get_response_sender()
    }
}

impl RuntimeRequestSink for HealthMonitorBuilder {
    fn get_signal_sender(&self) -> DynSender<RuntimeRequest> {
        Box::new(self.box_builder.get_signal_sender())
    }
}

impl Builder<(HealthMonitorActor, HealthMonitorMessageBox)> for HealthMonitorBuilder {
    type Error = LinkError;

    fn try_build(self) -> Result<(HealthMonitorActor, HealthMonitorMessageBox), Self::Error> {
        let message_box = HealthMonitorMessageBox::new(
            self.box_builder.name.clone(),
            self.box_builder.input_receiver,
            self.box_builder.output_sender.clone(),
        );

        let actor = HealthMonitorActor::new(
            self.box_builder.name,
            self.box_builder.output_sender,
            self.subscriptions,
        );

        Ok((actor, message_box))
    }
}

impl ServiceConsumer<MqttMessage, MqttMessage, TopicFilter> for HealthMonitorBuilder {
    fn get_config(&self) -> TopicFilter {
        self.subscriptions.clone()
    }

    fn set_request_sender(&mut self, request_sender: DynSender<MqttMessage>) {
        self.box_builder.set_request_sender(request_sender);
    }

    fn get_response_sender(&self) -> DynSender<MqttMessage> {
        self.box_builder.get_sender()
    }
}
