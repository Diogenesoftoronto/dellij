pub mod git;
pub mod layout;
pub mod types;

pub use git::{git, git_output};
pub use layout::{BuiltinLayout, LayoutRenderer};
pub use types::{
    Bookmark, Config, Settings, StatusFile, Workspace, WorkspaceStatus,
};

use anyhow::Result;
use camino::Utf8Path;
use serde::Serialize;
use std::fmt;

// ── pure utilities ────────────────────────────────────────────────────────────

pub fn slugify(input: &str, agent: &str) -> String {
    let mut pieces = Vec::new();
    let base = input
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch.to_ascii_lowercase() } else { '-' })
        .collect::<String>();
    for part in base.split('-').filter(|p| !p.is_empty()) {
        pieces.push(part.to_string());
        if pieces.len() == 6 {
            break;
        }
    }
    if pieces.is_empty() {
        pieces.push("workspace".to_string());
    }
    format!("{}-{}", pieces.join("-"), agent)
}

pub fn shell_escape(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

pub fn render_agent_command(agent: &str, prompt: &str) -> String {
    match agent {
        "codex" => format!("codex \"{}\"", shell_escape(prompt)),
        "claude" | "claude-code" => format!("claude \"{}\"", shell_escape(prompt)),
        "aider" => format!("aider --message \"{}\"", shell_escape(prompt)),
        "gemini" => format!("gemini \"{}\"", shell_escape(prompt)),
        "opencode" => format!("opencode \"{}\"", shell_escape(prompt)),
        other => format!("{other} \"{}\"", shell_escape(prompt)),
    }
}

pub fn write_json<T: Serialize>(path: &Utf8Path, value: &T) -> Result<()> {
    use anyhow::Context;
    let body = serde_json::to_string_pretty(value)?;
    std::fs::write(path, body).with_context(|| format!("writing {}", path))?;
    Ok(())
}

pub fn yes_no(value: bool) -> &'static str {
    if value { "ok" } else { "missing" }
}

pub fn command_exists(name: &str) -> bool {
    use std::env;
    env::var_os("PATH")
        .and_then(|paths| {
            env::split_paths(&paths).find(|dir| {
                let candidate = dir.join(name);
                candidate.is_file() || is_windows_exe(&candidate)
            })
        })
        .is_some()
}

fn is_windows_exe(candidate: &std::path::Path) -> bool {
    ["exe", "bat", "cmd"]
        .iter()
        .map(|ext| candidate.with_extension(ext))
        .any(|p| p.is_file())
}

pub fn inside_zellij() -> bool {
    std::env::var_os("ZELLIJ").is_some()
}

// ── ahead/behind ──────────────────────────────────────────────────────────────

pub struct AheadBehind {
    pub ahead: u32,
    pub behind: u32,
}

impl fmt::Display for AheadBehind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.ahead, self.behind) {
            (0, 0) => write!(f, ""),
            (a, 0) => write!(f, "↑{a}"),
            (0, b) => write!(f, "↓{b}"),
            (a, b) => write!(f, "↑{a} ↓{b}"),
        }
    }
}

pub fn ahead_behind(
    project_root: &Utf8Path,
    branch: &str,
    base: &str,
) -> Option<AheadBehind> {
    let ahead = git_output(project_root, &["rev-list", "--count", &format!("{base}..{branch}")])
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(0);
    let behind = git_output(project_root, &["rev-list", "--count", &format!("{branch}..{base}")])
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
        .unwrap_or(0);
    Some(AheadBehind { ahead, behind })
}

// ── pipe protocol ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PipeCommand {
    Open { slug: String },
    Focus { slug: String },
    Send { slug: String, text: String },
    Status { slug: String, status: String },
}

impl PipeCommand {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("PipeCommand serialization")
    }
}
