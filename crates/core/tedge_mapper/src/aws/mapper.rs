use std::path::Path;

use crate::core::component::TEdgeComponent;
use aws_mapper_ext::converter::AwsConverter;

use async_trait::async_trait;
use tedge_config::TEdgeConfig;
use tracing::info;

const AWS_MAPPER_NAME: &str = "tedge-mapper-aws";

pub struct AwsMapper;

#[async_trait]
impl TEdgeComponent for AwsMapper {
    fn session_name(&self) -> &str {
        AWS_MAPPER_NAME
    }

    async fn init(&self, _config_dir: &Path) -> Result<(), anyhow::Error> {
        info!("Initialize tedge mapper aws");
        self.init_session(AwsConverter::in_topic_filter()).await?;

        Ok(())
    }

    async fn start(
        &self,
        _tedge_config: TEdgeConfig,
        _config_dir: &Path,
    ) -> Result<(), anyhow::Error> {
        panic!("This method should no more be used");
    }
}
