mod actor;
use actor::HealthMonitorActor;
use actor::HealthMonitorMessageBox;
use tedge_actors::futures::channel::mpsc;
use tedge_actors::Builder;
use tedge_actors::CombinedReceiver;
use tedge_actors::DynSender;
use tedge_actors::LinkError;
use tedge_actors::MessageSink;
use tedge_actors::MessageSource;
use tedge_actors::NoConfig;
use tedge_actors::RuntimeRequest;
use tedge_actors::RuntimeRequestSink;
use tedge_actors::ServiceConsumer;
use tedge_mqtt_ext::MqttActorBuilder;
use tedge_mqtt_ext::MqttMessage;
use tedge_mqtt_ext::TopicFilter;

type HealthInputMessage = MqttMessage;
type HealthOutputMessage = MqttMessage;

pub struct HealthMonitorBuilder {
    input_receiver: CombinedReceiver<HealthInputMessage>,
    input_sender: mpsc::Sender<HealthInputMessage>,
    output_sender: Option<DynSender<HealthOutputMessage>>,
    signal_sender: mpsc::Sender<RuntimeRequest>,
    name: String,
}

impl HealthMonitorBuilder {
    pub fn new(name: String) -> Self {
        let (input_sender, events_receiver) = mpsc::channel(10);
        let (signal_sender, signal_receiver) = mpsc::channel(10);
        let input_receiver = CombinedReceiver::new(events_receiver, signal_receiver);

        Self {
            input_receiver,
            input_sender,
            output_sender: None,
            signal_sender,
            name,
        }
    }

    /// Connect this config manager instance to some mqtt connection provider
    pub fn with_mqtt_connection(&mut self, mqtt: &mut MqttActorBuilder) -> Result<(), LinkError> {
        let subscriptions = vec![
            "tedge/health-check",
            "tedge/health-check/c8y-device-management",
        ]
        .try_into()?;
        //Register peers symmetrically here
        mqtt.register_peer(subscriptions, self.input_sender.clone().into());
        self.register_peer(NoConfig, mqtt.get_sender());
        Ok(())
    }
}

impl MessageSource<MqttMessage, NoConfig> for HealthMonitorBuilder {
    fn register_peer(&mut self, _config: NoConfig, sender: DynSender<MqttMessage>) {
        self.output_sender = Some(sender);
    }
}

impl MessageSink<MqttMessage> for HealthMonitorBuilder {
    fn get_sender(&self) -> DynSender<MqttMessage> {
        self.input_sender.clone().into()
    }
}

impl RuntimeRequestSink for HealthMonitorBuilder {
    fn get_signal_sender(&self) -> DynSender<RuntimeRequest> {
        Box::new(self.signal_sender.clone())
    }
}

impl Builder<(HealthMonitorActor, HealthMonitorMessageBox)> for HealthMonitorBuilder {
    type Error = LinkError;

    fn try_build(self) -> Result<(HealthMonitorActor, HealthMonitorMessageBox), Self::Error> {
        let mqtt_publisher = self.output_sender.ok_or_else(|| LinkError::MissingPeer {
            role: "mqtt".to_string(),
        })?;

        let message_box = HealthMonitorMessageBox::new(self.input_receiver, mqtt_publisher.clone());

        let actor = HealthMonitorActor::new(self.name, mqtt_publisher);

        Ok((actor, message_box))
    }
}

impl ServiceConsumer<MqttMessage, MqttMessage, TopicFilter> for HealthMonitorBuilder {
    fn get_config(&self) -> TopicFilter {
        vec![
            "tedge/health-check",
            "tedge/health-check/c8y-device-management",
        ]
        .try_into()
        .unwrap()
    }

    fn set_request_sender(&mut self, request_sender: DynSender<MqttMessage>) {
        self.output_sender = Some(request_sender)
    }

    fn get_response_sender(&self) -> DynSender<MqttMessage> {
        self.input_sender.clone().into()
    }
}
