use tedge_api::serialize::ThinEdgeJsonSerializationError;
use tedge_config::TEdgeConfigError;

// allowing enum_variant_names due to a False positive where it is
// detected that "all variants have the same prefix: `From`"
#[allow(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
pub enum MapperError {
    #[cfg(test)] // this error is only used in a test so far
    #[error("Home directory is not found.")]
    HomeDirNotFound,

    #[error(transparent)]
    FromTEdgeConfig(#[from] TEdgeConfigError),

    #[error(transparent)]
    FromConfigSetting(#[from] tedge_config::ConfigSettingError),

    #[error(transparent)]
    FromStdIo(#[from] std::io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ConversionError {
    #[error(transparent)]
    FromMapper(#[from] MapperError),

    #[error(transparent)]
    FromThinEdgeJsonSerialization(#[from] ThinEdgeJsonSerializationError),

    #[error(transparent)]
    FromThinEdgeJsonAlarmDeserialization(#[from] tedge_api::alarm::ThinEdgeJsonDeserializerError),

    #[error(transparent)]
    FromThinEdgeJsonEventDeserialization(
        #[from] tedge_api::event::error::ThinEdgeJsonDeserializerError,
    ),

    #[error(transparent)]
    FromThinEdgeJsonParser(#[from] tedge_api::parser::ThinEdgeJsonParserError),

    #[error("The size of the message received on {topic} is {actual_size} which is greater than the threshold size of {threshold}.")]
    SizeThresholdExceeded {
        topic: String,
        actual_size: usize,
        threshold: usize,
    },

    #[error(transparent)]
    FromSerdeJson(#[from] serde_json::Error),

    #[error(transparent)]
    FromStdIo(#[from] std::io::Error),

    #[error(transparent)]
    FromUtf8Error(#[from] std::string::FromUtf8Error),

    #[error(transparent)]
    FromTimeFormatError(#[from] time::error::Format),

    #[error("The payload {payload} received on {topic} after translation is {actual_size} greater than the threshold size of {threshold}.")]
    TranslatedSizeExceededThreshold {
        payload: String,
        topic: String,
        actual_size: usize,
        threshold: usize,
    },
}
