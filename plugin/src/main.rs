use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use zellij_tile::prelude::*;

// ── types ─────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct DellijStatusPlugin {
    agents: Vec<AgentInfo>,
    config_dir: String,
    tick: u64,
    tabs: Vec<TabState>,
    /// Panes in the current session: tab_name → Vec<pane_id>
    panes: BTreeMap<String, Vec<u32>>,
    permissions_granted: bool,
}

#[derive(Clone)]
struct AgentInfo {
    slug: String,
    short_label: String,
    status: String,
    needs_attention: bool,
    pr_number: Option<u32>,
}

#[derive(Clone, Debug)]
struct TabState {
    position: usize,
    name: String,
    active: bool,
}

#[derive(Debug, Deserialize)]
struct StatusFile {
    slug: String,
    status: String,
    #[serde(default)]
    agent: String,
    #[serde(default)]
    needs_attention: bool,
    #[serde(default)]
    pr_number: Option<u32>,
}

/// Commands sent by the dellij CLI via `zellij pipe`.
#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum PipeCommand {
    Open { slug: String },
    Focus { slug: String },
    Send { slug: String, text: String },
    Status { slug: String, status: String },
}

register_plugin!(DellijStatusPlugin);

impl ZellijPlugin for DellijStatusPlugin {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.config_dir = configuration.get("config_dir").cloned().unwrap_or_default();

        set_selectable(false);

        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
            PermissionType::WriteToStdin,
        ]);

        subscribe(&[
            EventType::Timer,
            EventType::TabUpdate,
            EventType::PaneUpdate,
            EventType::PermissionRequestResult,
        ]);

        set_timeout(2.0);
    }

    fn update(&mut self, event: Event) -> bool {
        let mut should_render = false;
        match event {
            Event::Timer(_) => {
                set_timeout(2.0);
                self.tick += 1;
                self.agents = self.read_status_files();
                should_render = true;
            }

            Event::TabUpdate(tab_infos) => {
                self.tabs = tab_infos
                    .iter()
                    .map(|t| TabState {
                        position: t.position,
                        name: t.name.clone(),
                        active: t.active,
                    })
                    .collect();
                // Update attention indicators on tab names
                if self.permissions_granted {
                    self.sync_tab_attention();
                }
                should_render = true;
            }

            Event::PaneUpdate(pane_manifest) => {
                self.panes.clear();
                for (tab_pos, panes) in pane_manifest.panes {
                    let tab_name = self
                        .tabs
                        .iter()
                        .find(|t| t.position == tab_pos)
                        .map(|t| t.name.clone())
                        .unwrap_or_default();
                    let ids: Vec<u32> = panes.iter().filter_map(|p| {
                        if p.is_plugin { None } else { Some(p.id) }
                    }).collect();
                    self.panes.insert(tab_name, ids);
                }
            }

            Event::PermissionRequestResult(status) => {
                self.permissions_granted = status == PermissionStatus::Granted;
                if self.permissions_granted {
                    should_render = true;
                }
            }

            _ => {}
        }
        should_render
    }

    fn render(&mut self, _rows: usize, _cols: usize) {
        if self.config_dir.is_empty() || self.agents.is_empty() {
            return;
        }

        let mut text = String::from(" dellij ");
        // (color_idx, start_byte, end_byte)
        let mut styling: Vec<(usize, usize, usize)> = vec![(2, 0, 8)]; // "dellij" in green
        let mut pos = 8;

        for agent in &self.agents {
            let is_active = self
                .tabs
                .iter()
                .any(|t| t.active && t.name == agent.slug);
            let status_color = status_to_color_idx(&agent.status);
            let attn_marker = if agent.needs_attention { "! " } else { "" };
            let pr_suffix = agent.pr_number
                .map(|n| format!(" #{n}"))
                .unwrap_or_default();

            let segment = format!(
                " {}{} ● {}{}",
                attn_marker, agent.short_label, agent.status, pr_suffix
            );
            let end = pos + segment.len();

            // Base segment color
            styling.push((if is_active { 0 } else { 1 }, pos, end));

            // Dot color (●): attn_marker(0-2) + label(2) + space(1) + dot_byte = pos+3+attn
            let dot_offset = attn_marker.len() + 1 + agent.short_label.len() + 1 + 1;
            let dot_start = pos + dot_offset;
            let dot_end = dot_start + 3; // ● is 3 bytes in UTF-8
            if dot_end <= end {
                styling.push((status_color, dot_start, dot_end));
            }

            // Attention marker color (red)
            if agent.needs_attention {
                styling.push((4, pos + 1, pos + 3));
            }

            text.push_str(&segment);
            pos = end;
        }

        use zellij_tile::ui_components::{print_ribbon, Text};
        let mut component = Text::new(text);
        for (idx, start, end) in styling {
            component = component.color_range(idx, start..end);
        }
        print_ribbon(component);
    }

    fn pipe(&mut self, pipe_message: PipeMessage) -> bool {
        if let Some(payload) = pipe_message.payload.as_deref() {
            if let Ok(cmd) = serde_json::from_str::<PipeCommand>(payload) {
                self.handle_pipe_command(cmd);
                return true;
            }
        }
        false
    }
}

impl DellijStatusPlugin {
    // ── pipe command handling ──────────────────────────────────────────────

    fn handle_pipe_command(&mut self, cmd: PipeCommand) {
        match cmd {
            PipeCommand::Open { slug } | PipeCommand::Focus { slug } => {
                self.focus_or_create_tab(&slug);
            }
            PipeCommand::Send { slug, text } => {
                self.send_to_workspace_shell(&slug, &text);
            }
            PipeCommand::Status { slug, status } => {
                // Update in-memory status so render reflects immediately
                if let Some(agent) = self.agents.iter_mut().find(|a| a.slug == slug) {
                    agent.needs_attention = matches!(
                        status.as_str(),
                        "blocked" | "error" | "review"
                    );
                    agent.status = status;
                }
                self.sync_tab_attention();
            }
        }
    }

    fn focus_or_create_tab(&self, slug: &str) {
        if let Some(tab) = self.tabs.iter().find(|t| t.name == slug) {
            // Tab already exists — focus it
            go_to_tab(tab.position as u32);
        } else {
            // Create a new tab using the stored layout file
            let layout_path = format!("{}/layouts/{}.kdl", self.config_dir, slug);
            if Path::new(&layout_path).exists() {
                match fs::read_to_string(&layout_path) {
                    Ok(layout) => new_tabs_with_layout(&layout),
                    Err(_) => new_tab(Some(slug), None::<&str>),
                }
            } else {
                // Fallback: create a blank tab named after the slug
                new_tab(Some(slug), None::<&str>);
            }
        }
    }

    fn send_to_workspace_shell(&self, slug: &str, text: &str) {
        // Focus the tab first
        if let Some(tab) = self.tabs.iter().find(|t| t.name == slug) {
            go_to_tab(tab.position as u32);
        }
        // Write text to the currently focused terminal pane
        write_chars(text);
    }

    /// Annotate tab names for workspaces that need attention.
    /// Adds a "!" prefix to tabs with blocked/error/review status.
    fn sync_tab_attention(&self) {
        for tab in &self.tabs {
            let needs_attn = self
                .agents
                .iter()
                .any(|a| a.slug == tab.name && a.needs_attention);
            let desired_name = if needs_attn {
                if tab.name.starts_with('!') {
                    tab.name.clone()
                } else {
                    format!("!{}", tab.name)
                }
            } else {
                tab.name.trim_start_matches('!').to_string()
            };
            if desired_name != tab.name {
                rename_tab(tab.position as u32, &desired_name);
            }
        }
    }

    // ── status file reading ────────────────────────────────────────────────

    fn read_status_files(&self) -> Vec<AgentInfo> {
        let status_dir = Path::new(&self.config_dir).join("status");
        let read_dir = match fs::read_dir(&status_dir) {
            Ok(rd) => rd,
            Err(_) => return Vec::new(),
        };

        let mut agents = Vec::new();
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }

            let content = match fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let parsed: StatusFile = match serde_json::from_str(&content) {
                Ok(p) => p,
                Err(_) => continue,
            };
            if parsed.slug.is_empty() { continue; }

            let short_label = derive_short_label(&parsed.slug);
            agents.push(AgentInfo {
                slug: parsed.slug.clone(),
                short_label,
                status: parsed.status.clone(),
                needs_attention: parsed.needs_attention || matches!(
                    parsed.status.as_str(),
                    "blocked" | "error" | "review"
                ),
                pr_number: parsed.pr_number,
            });
        }
        agents.sort_by(|a, b| a.slug.cmp(&b.slug));
        agents
    }
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn status_to_color_idx(status: &str) -> usize {
    match status {
        "working" => 3, // Yellow
        "waiting" => 7, // Cyan
        "error" => 4,   // Red
        "blocked" => 4, // Red
        "review" => 7,  // Cyan
        "done" => 2,    // Green
        _ => 1,
    }
}

fn derive_short_label(slug: &str) -> String {
    let known: &[(&str, &str)] = &[
        ("claude-code", "cc"),
        ("opencode", "oc"),
        ("codex", "cx"),
        ("cline", "cl"),
        ("gemini", "gm"),
        ("qwen", "qn"),
        ("amp", "ap"),
        ("pi", "pi"),
        ("cursor", "cr"),
        ("copilot", "co"),
        ("crush", "cs"),
        ("aider", "ai"),
    ];
    for (suffix, label) in known {
        if slug.ends_with(suffix) || slug.contains(&format!("-{suffix}")) {
            return label.to_string();
        }
    }
    slug.chars().take(2).collect()
}
