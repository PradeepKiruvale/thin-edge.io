use crate::aws::mapper::AwsMapper;
use crate::az::mapper::AzureMapper;
use crate::c8y::mapper::CumulocityMapper;
use crate::collectd::mapper::CollectdMapper;
use crate::core::component::TEdgeComponent;
use az_mapper_ext::converter::AzureConverter;
use az_mapper_ext::size_threshold::SizeThreshold;
use clap::Parser;
use clock::WallClock;
use flockfile::check_another_instance_is_not_running;
use std::fmt;
use std::path::PathBuf;
use tedge_actors::builders::ServiceConsumer;
use tedge_actors::ConvertingActor;
use tedge_actors::MessageSink;
use tedge_actors::MessageSource;
use tedge_actors::NoConfig;
use tedge_actors::Runtime;
use tedge_config::system_services::get_log_level;
use tedge_config::system_services::set_log_level;
use tedge_config::DEFAULT_TEDGE_CONFIG_PATH;
use tedge_config::*;
use tedge_health_ext::HealthMonitorBuilder;
use tedge_mqtt_ext::MqttActorBuilder;
use tedge_mqtt_ext::MqttConfig;
use tedge_signal_ext::SignalActor;

mod aws;
mod az;
mod c8y;
mod collectd;
mod core;

fn lookup_component(component_name: &MapperName) -> Box<dyn TEdgeComponent> {
    match component_name {
        MapperName::Az => Box::new(AzureMapper::new()),
        MapperName::Aws => Box::new(AwsMapper),
        MapperName::Collectd => Box::new(CollectdMapper::new()),
        MapperName::C8y => Box::new(CumulocityMapper::new()),
    }
}

#[derive(Debug, Parser)]
#[clap(
    name = clap::crate_name!(),
    version = clap::crate_version!(),
    about = clap::crate_description!()
)]
pub struct MapperOpt {
    #[clap(subcommand)]
    pub name: MapperName,

    /// Turn-on the debug log level.
    ///
    /// If off only reports ERROR, WARN, and INFO
    /// If on also reports DEBUG and TRACE
    #[clap(long, global = true)]
    pub debug: bool,

    /// Start the mapper with clean session off, subscribe to the topics, so that no messages are lost
    #[clap(short, long)]
    pub init: bool,

    /// Start the agent with clean session on, drop the previous session and subscriptions
    ///
    /// WARNING: All pending messages will be lost.
    #[clap(short, long)]
    pub clear: bool,

    /// Start the mapper from custom path
    ///
    /// WARNING: This is mostly used in testing.
    #[clap(long = "config-dir", default_value = DEFAULT_TEDGE_CONFIG_PATH)]
    pub config_dir: PathBuf,
}

#[derive(Debug, clap::Subcommand)]
pub enum MapperName {
    Az,
    Aws,
    C8y,
    Collectd,
}

impl fmt::Display for MapperName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MapperName::Az => write!(f, "tedge-mapper-az"),
            MapperName::Aws => write!(f, "tedge-mapper-aws"),
            MapperName::C8y => write!(f, "tedge-mapper-c8y"),
            MapperName::Collectd => write!(f, "tedge-mapper-collectd"),
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mapper_opt = MapperOpt::parse();

    let component = lookup_component(&mapper_opt.name);

    let tedge_config_location =
        tedge_config::TEdgeConfigLocation::from_custom_root(&mapper_opt.config_dir);
    let config = tedge_config::TEdgeConfigRepository::new(tedge_config_location.clone()).load()?;

    let log_level = if mapper_opt.debug {
        tracing::Level::TRACE
    } else {
        get_log_level(
            "tedge-mapper",
            &tedge_config_location.tedge_config_root_path,
        )?
    };
    set_log_level(log_level);

    // Run only one instance of a mapper (if enabled)
    let mut _flock = None;
    if config.query(LockFilesSetting)?.is_set() {
        let run_dir: PathBuf = config.query(RunPathSetting)?.into();
        _flock = Some(check_another_instance_is_not_running(
            &mapper_opt.name.to_string(),
            &run_dir,
        )?);
    }

    let mut mqtt_actor = get_mqtt_actor(component.session_name(), &config).await?;
    let runtime_events_logger = None;
    let mut runtime = Runtime::try_new(runtime_events_logger).await?;
    let mut signal_actor = SignalActor::builder();
    if mapper_opt.init {
        component.init(&mapper_opt.config_dir).await
    } else if mapper_opt.clear {
        component.clear_session().await
    } else if component.session_name().eq("tedge-mapper-az") {
        let add_time_stamp = config.query(AzureMapperTimestamp)?.is_set();
        let clock = Box::new(WallClock);
        let size_threshold = SizeThreshold(255 * 1024);
        let az_converter = AzureConverter::new(add_time_stamp, clock, size_threshold);

        // Instantiate the azure mapper converter
        let mut az_converting_actor = ConvertingActor::builder("AzConverter", az_converter);
        mqtt_actor.register_peer(
            AzureConverter::in_topic_filter(),
            az_converting_actor.get_sender(),
        );
        az_converting_actor.register_peer(NoConfig, mqtt_actor.get_sender());

        //Instantiate health monitor actor
        let health_actor = HealthMonitorBuilder::new(component.session_name());
        mqtt_actor.mqtt_config = health_actor.set_init_and_last_will(mqtt_actor.mqtt_config);
        let health_actor = health_actor.with_connection(&mut mqtt_actor);

        // Shutdown on SIGINT
        signal_actor.register_peer(NoConfig, runtime.get_handle().get_sender());

        runtime.spawn(signal_actor).await?;
        runtime.spawn(mqtt_actor).await?;
        runtime.spawn(az_converting_actor).await?;
        runtime.spawn(health_actor).await?;

        runtime.run_to_completion().await?;
        Ok(())
    } else {
        component.start(config, &mapper_opt.config_dir).await
    }
}

async fn get_mqtt_actor(
    session_name: &str,
    tedge_config: &TEdgeConfig,
) -> Result<MqttActorBuilder, anyhow::Error> {
    let mqtt_port = tedge_config.query(MqttClientPortSetting)?.into();
    let mqtt_host = tedge_config.query(MqttClientHostSetting)?;

    let mqtt_config = MqttConfig::default()
        .with_host(mqtt_host)
        .with_port(mqtt_port);

    Ok(MqttActorBuilder::new(
        mqtt_config.with_session_name(session_name),
    ))
}
