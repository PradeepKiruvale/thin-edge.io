use tedge_actors::RuntimeError;

#[derive(Debug, thiserror::Error)]
#[allow(clippy::enum_variant_names)]
pub enum TedgetoTeConverterError {
    #[error("Unsupported topic: {0}")]
    UnsupportedTopic(String),

    #[error(transparent)]
    FromChannelError(#[from] tedge_actors::ChannelError),
}

impl From<TedgetoTeConverterError> for RuntimeError {
    fn from(error: TedgetoTeConverterError) -> Self {
        RuntimeError::ActorError(Box::new(error))
    }
}
