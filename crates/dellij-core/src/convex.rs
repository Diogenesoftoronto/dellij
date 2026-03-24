use std::collections::BTreeMap;
use anyhow::Result;
use convex::{ConvexClient, Value};
use crate::types::{Workspace, StatusFile};

use std::fmt;

pub struct ConvexSyncClient {
    client: ConvexClient,
}

impl fmt::Debug for ConvexSyncClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConvexSyncClient").finish()
    }
}

impl ConvexSyncClient {
    pub async fn new(url: &str) -> Result<Self> {
        let client = ConvexClient::new(url).await
            .map_err(|e| anyhow::anyhow!("failed to create convex client: {}", e))?;
        Ok(Self { client })
    }

    pub async fn set_auth(&mut self, token: Option<String>) {
        self.client.set_auth(token).await;
    }

    pub async fn push_workspace(&mut self, workspace: &Workspace) -> Result<()> {
        let mut args = BTreeMap::new();
        args.insert("slug".to_string(), Value::String(workspace.slug.clone()));
        args.insert("prompt".to_string(), Value::String(workspace.prompt.clone()));
        args.insert("agent".to_string(), Value::String(workspace.agent.clone()));
        args.insert("branch_name".to_string(), Value::String(workspace.branch_name.clone()));
        args.insert("base_branch".to_string(), Value::String(workspace.base_branch.clone()));
        args.insert("worktree_path".to_string(), Value::String(workspace.worktree_path.to_string()));
        args.insert("status".to_string(), Value::String(workspace.status.to_string()));
        args.insert("updated_at".to_string(), Value::String(workspace.updated_at.to_rfc3339()));
        
        if let Some(pr) = workspace.pr_number {
            args.insert("pr_number".to_string(), Value::Int64(pr as i64));
        }
        if let Some(url) = &workspace.pr_url {
            args.insert("pr_url".to_string(), Value::String(url.clone()));
        }

        self.client.mutation("workspaces:upsert", args).await
            .map_err(|e| anyhow::anyhow!("convex mutation failed: {}", e))?;
        
        Ok(())
    }

    pub async fn push_status(&mut self, status_file: &StatusFile) -> Result<()> {
        let mut args = BTreeMap::new();
        args.insert("slug".to_string(), Value::String(status_file.slug.clone()));
        args.insert("status".to_string(), Value::String(status_file.status.to_string()));
        args.insert("updated_at".to_string(), Value::String(status_file.updated_at.to_rfc3339()));
        args.insert("agent".to_string(), Value::String(status_file.agent.clone()));
        args.insert("needs_attention".to_string(), Value::Boolean(status_file.needs_attention));
        
        if let Some(pr) = status_file.pr_number {
            args.insert("pr_number".to_string(), Value::Int64(pr as i64));
        }

        self.client.mutation("status:upsert", args).await
            .map_err(|e| anyhow::anyhow!("convex mutation failed: {}", e))?;
        
        Ok(())
    }
}
