use crate::logconfig::{SystemConfig, SystemServiceError};
use std::path::PathBuf;

/// Initialize a `tracing_subscriber`
///
/// Reports all the log events sent either with the `log` crate or the `tracing` crate.
///
/// If `debug` is `false` then only `error!`, `warn!` and `info!` are reported.
/// If `debug` is `true` then only `debug!` and `trace!` are reported.
pub fn initialise_tracing_subscriber(
    debug: bool,
    config_dir: PathBuf,
) -> Result<(), SystemServiceError> {
    let log_config = SystemConfig::try_new(config_dir)?.log;
    let log_level = if debug || log_config.unwrap_or_default().is_debug {
        tracing::Level::TRACE
    } else {
        tracing::Level::INFO
    };

    tracing_subscriber::fmt()
        .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339())
        .with_max_level(log_level)
        .init();

    Ok(())
}
