use std::path::Path;

use crate::converter::AzureConverter;
use async_trait::async_trait;
use tedge_config::TEdgeConfig;
use tedge_mapper_core::component::TEdgeComponent;



use tedge_utils::file::create_directory_with_user_group;
use tracing::info;

const AZURE_MAPPER_NAME: &str = "tedge-mapper-az";

pub struct AzureMapper {}

impl AzureMapper {
    pub fn new() -> AzureMapper {
        AzureMapper {}
    }
}

impl Default for AzureMapper {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TEdgeComponent for AzureMapper {
    fn session_name(&self) -> &str {
        AZURE_MAPPER_NAME
    }

    async fn init(&self, config_dir: &Path) -> Result<(), anyhow::Error> {
        info!("Initialize tedge mapper az");
        create_directory_with_user_group(
            format!("{}/operations/az", config_dir.display()),
            "tedge",
            "tedge",
            0o775,
        )?;

        self.init_session(AzureConverter::in_topic_filter()).await?;
        Ok(())
    }

    async fn start(
        &self,
        _tedge_config: TEdgeConfig,
        _config_dir: &Path,
    ) -> Result<(), anyhow::Error> {
        Ok(())
    }
}
