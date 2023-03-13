use async_trait::async_trait;
use tedge_actors::Actor;
use tedge_actors::CombinedReceiver;
use tedge_actors::DynSender;
use tedge_actors::MessageBox;
use tedge_actors::ReceiveMessages;
use tedge_actors::RuntimeError;
use tedge_actors::RuntimeRequest;
use tedge_actors::WrappedInput;
use tedge_api::health::health_status_down_message;
use tedge_api::health::health_status_up_message;
use tedge_mqtt_ext::MqttMessage;
use tedge_mqtt_ext::TopicFilter;

type HealthInputMessage = MqttMessage;
type HealthOutputMessage = MqttMessage;

pub struct TedgeHealthMonitorActor {
    health_check_topics: TopicFilter,
    mqtt_publisher: DynSender<MqttMessage>,
    daemon_to_be_monitored: String,
}

impl TedgeHealthMonitorActor {
    pub fn new(daemon_to_be_monitored: String, mqtt_publisher: DynSender<MqttMessage>) -> Self {
        let health_check_topics = vec!["tedge/health-check", "tedge/health-check/+"]
            .try_into()
            .unwrap();
        Self {
            health_check_topics,
            mqtt_publisher,
            daemon_to_be_monitored,
        }
    }

    pub async fn process_mqtt_message(
        &mut self,
        message: MqttMessage,
    ) -> Result<(), anyhow::Error> {
        if self.health_check_topics.accept(&message) {
            //Process the mqtt message and send the reply to the health check request message
            self.mqtt_publisher
                .send(health_status_up_message(&self.daemon_to_be_monitored))
                .await?;
        }
        Ok(())
    }

    pub async fn send_up_health_status(&mut self) -> Result<(), anyhow::Error> {
        Ok(self
            .mqtt_publisher
            .send(health_status_up_message(&self.daemon_to_be_monitored))
            .await?)
    }

    pub async fn send_down_health_status(&mut self) -> Result<(), anyhow::Error> {
        Ok(self
            .mqtt_publisher
            .send(health_status_down_message(&self.daemon_to_be_monitored))
            .await?)
    }
}

// FIXME: Consider to use a SimpleMessageBox<LogInput,MqttMessage>
pub struct HealthMonitorMessageBox {
    input_receiver: CombinedReceiver<HealthInputMessage>,
    #[allow(dead_code)]
    mqtt_requests: DynSender<MqttMessage>,
}

impl HealthMonitorMessageBox {
    pub fn new(
        input_receiver: CombinedReceiver<HealthInputMessage>,
        mqtt_con: DynSender<MqttMessage>,
    ) -> HealthMonitorMessageBox {
        HealthMonitorMessageBox {
            input_receiver,
            mqtt_requests: mqtt_con,
        }
    }
}

impl MessageBox for HealthMonitorMessageBox {
    type Input = HealthInputMessage;
    type Output = HealthOutputMessage;

    fn turn_logging_on(&mut self, _on: bool) {
        todo!()
    }

    fn name(&self) -> &str {
        "Health-Monitor-Manager"
    }

    fn logging_is_on(&self) -> bool {
        // FIXME this mailbox recv and send method are not used making logging ineffective.
        false
    }
}

#[async_trait]
impl ReceiveMessages<HealthInputMessage> for HealthMonitorMessageBox {
    async fn try_recv(&mut self) -> Result<Option<HealthOutputMessage>, RuntimeRequest> {
        self.input_receiver.try_recv().await
    }

    async fn recv_message(&mut self) -> Option<WrappedInput<HealthInputMessage>> {
        self.input_receiver.recv_message().await
    }

    async fn recv(&mut self) -> Option<HealthInputMessage> {
        self.input_receiver.recv().await.map(|message| {
            self.log_input(&message);
            message
        })
    }
}

#[async_trait]
impl Actor for TedgeHealthMonitorActor {
    type MessageBox = HealthMonitorMessageBox;

    fn name(&self) -> &str {
        "HealthMonitorActor"
    }

    async fn run(mut self, mut messages: Self::MessageBox) -> Result<(), RuntimeError> {
        self.send_up_health_status().await.unwrap();
        while let Some(message) = messages.recv().await {
            {
                self.process_mqtt_message(message).await.unwrap();
            }
        }
        self.send_down_health_status().await.unwrap();
        Ok(())
    }
}
