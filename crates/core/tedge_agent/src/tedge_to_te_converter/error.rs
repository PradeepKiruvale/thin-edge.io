use tedge_actors::RuntimeError;
use tedge_api::alarm::ThinEdgeJsonDeserializerError;

#[derive(Debug, thiserror::Error)]
#[allow(clippy::enum_variant_names)]
pub enum TedgetoTeConverterError {
    #[error("Unsupported topic: {0}")]
    UnsupportedTopic(String),

    #[error(transparent)]
    FromChannelError(#[from] tedge_actors::ChannelError),

    #[error(transparent)]
    MqttError(#[from] mqtt_channel::MqttError),

    #[error(transparent)]
    SerdeJsonError(#[from] serde_json::Error),

    #[error(transparent)]
    AlarmEr(#[from] ThinEdgeJsonDeserializerError),
}

impl From<TedgetoTeConverterError> for RuntimeError {
    fn from(error: TedgetoTeConverterError) -> Self {
        RuntimeError::ActorError(Box::new(error))
    }
}
