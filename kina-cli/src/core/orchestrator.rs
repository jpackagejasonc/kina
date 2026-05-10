use anyhow::Result;
use async_trait::async_trait;
use std::path::PathBuf;

use super::types::{ClusterInfo, CreateClusterOptions};

#[allow(dead_code)]
#[async_trait]
pub trait OrchestratorProvider: Send + Sync {
    async fn create_cluster(&self, options: &CreateClusterOptions) -> Result<PathBuf>;
    async fn delete_cluster(&self, name: &str) -> Result<()>;
    async fn get_kubeconfig_path(&self, name: &str) -> Result<PathBuf>;
    async fn is_running(&self, name: &str) -> Result<bool>;
    async fn list_clusters(&self) -> Result<Vec<ClusterInfo>>;
}
