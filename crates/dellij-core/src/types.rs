use std::collections::BTreeMap;
use std::fmt;

use camino::Utf8PathBuf;
use chrono::{DateTime, Utc};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

// ── WorkspaceStatus ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceStatus {
    Working,
    Waiting,
    Blocked,
    Review,
    Done,
    Error,
}

impl fmt::Display for WorkspaceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Working => "working",
            Self::Waiting => "waiting",
            Self::Blocked => "blocked",
            Self::Review => "review",
            Self::Done => "done",
            Self::Error => "error",
        };
        f.write_str(s)
    }
}

impl WorkspaceStatus {
    pub fn needs_attention(self) -> bool {
        matches!(self, Self::Blocked | Self::Error | Self::Review)
    }
}

// ── Workspace ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub slug: String,
    pub prompt: String,
    pub agent: String,
    pub branch_name: String,
    pub base_branch: String,
    pub worktree_path: Utf8PathBuf,
    pub status: WorkspaceStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub ports: Vec<u16>,
    pub urls: Vec<String>,
    pub last_command: Option<String>,
    pub notes: Vec<String>,
    /// GitHub PR number if associated
    #[serde(default)]
    pub pr_number: Option<u32>,
    /// GitHub PR URL
    #[serde(default)]
    pub pr_url: Option<String>,
    /// Named layout to use (maps to Settings.layouts)
    #[serde(default)]
    pub layout: Option<String>,
}

// ── Bookmark ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bookmark {
    pub name: String,
    pub command: String,
    pub description: Option<String>,
}

// ── Settings ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub default_agent: String,
    pub base_branch: String,
    pub branch_prefix: String,
    pub workspace_root: Utf8PathBuf,
    /// Named KDL layout templates. Keys are layout names; values are KDL strings
    /// with `{cwd}`, `{agent_cmd}`, `{slug}`, `{branch}`, `{prompt}` placeholders.
    #[serde(default)]
    pub layouts: BTreeMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_agent: "codex".to_string(),
            base_branch: "main".to_string(),
            branch_prefix: "dellij/".to_string(),
            workspace_root: Utf8PathBuf::from(".dellij/workspaces"),
            layouts: BTreeMap::new(),
        }
    }
}

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub project_root: Utf8PathBuf,
    pub created_at: DateTime<Utc>,
    pub settings: Settings,
    pub bookmarks: Vec<Bookmark>,
    pub workspaces: Vec<Workspace>,
}

impl Config {
    pub fn new(project_root: Utf8PathBuf) -> Self {
        Self {
            project_root,
            created_at: Utc::now(),
            settings: Settings::default(),
            bookmarks: Vec::new(),
            workspaces: Vec::new(),
        }
    }
}

// ── StatusFile ────────────────────────────────────────────────────────────────

/// Written to `.dellij/status/<slug>.json`; read by the Zellij plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusFile {
    pub slug: String,
    pub status: WorkspaceStatus,
    pub updated_at: DateTime<Utc>,
    pub agent: String,
    #[serde(default)]
    pub pr_number: Option<u32>,
    #[serde(default)]
    pub needs_attention: bool,
}

impl StatusFile {
    pub fn from_workspace(ws: &Workspace) -> Self {
        Self {
            slug: ws.slug.clone(),
            status: ws.status,
            updated_at: ws.updated_at,
            agent: ws.agent.clone(),
            pr_number: ws.pr_number,
            needs_attention: ws.status.needs_attention(),
        }
    }
}
