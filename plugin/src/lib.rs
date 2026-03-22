use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use zellij_tile::prelude::*;

#[derive(Default)]
struct DellijStatusPlugin {
    agents: Vec<AgentInfo>,
    config_dir: String,
    tick: u64,
    active_tab: Option<String>,
}

#[derive(Clone)]
struct AgentInfo {
    slug: String,
    short_label: String,
    status: String,
}

#[derive(Debug, Deserialize)]
struct StatusFile {
    status: String,
    slug: Option<String>,
}

impl ZellijPlugin for DellijStatusPlugin {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.config_dir = configuration
            .get("config_dir")
            .cloned()
            .unwrap_or_default();

        request_permission(&[
            PermissionType::ReadApplicationState,
        ]);

        subscribe(&[
            EventType::Timer,
            EventType::TabUpdate,
            EventType::PermissionRequestResult,
        ]);
        
        set_timeout(2.0);
    }

    fn update(&mut self, event: Event) -> bool {
        let mut should_render = false;
        match event {
            Event::Timer(_elapsed) => {
                set_timeout(2.0);
                self.tick += 1;
                self.agents = self.read_status_files();
                should_render = true;
            },
            Event::TabUpdate(tab_infos) => {
                for tab in tab_infos {
                    if tab.active {
                        self.active_tab = Some(tab.name);
                        should_render = true;
                        break;
                    }
                }
            },
            Event::PermissionRequestResult(status) => {
                if status == PermissionStatus::Granted {
                    should_render = true;
                }
            },
            _ => {}
        }
        should_render
    }

    fn render(&mut self, _rows: usize, cols: usize) {
        if self.config_dir.is_empty() {
            print!("dellij: no config_dir configured");
            return;
        }

        let agent_count = self.agents.len();

        if agent_count == 0 {
            print!("dellij: no agents running");
            return;
        }

        let summary: Vec<String> = self
            .agents
            .iter()
            .map(|a| {
                let is_active = match &self.active_tab {
                    Some(t) if t == &a.slug => true,
                    _ => false,
                };

                let status_indicator = match a.status.as_str() {
                    "working" => "\x1b[33m●\x1b[0m",   // yellow
                    "waiting" => "\x1b[36m●\x1b[0m",   // cyan
                    "error" => "\x1b[31m●\x1b[0m",     // red
                    "done" => "\x1b[32m●\x1b[0m",      // green (done)
                    _ => "\x1b[32m●\x1b[0m",            // green (idle)
                };

                if is_active {
                    // White background, black text for active tab
                    format!("\x1b[47;30m {}:{} \x1b[0m", a.short_label, status_indicator)
                } else {
                    format!("{}:{}", a.short_label, status_indicator)
                }
            })
            .collect();


        let line = format!(
            " \x1b[1;36mdellij\x1b[0m: [{} agent{}]  {}",
            agent_count,
            if agent_count == 1 { "" } else { "s" },
            summary.join("  ")
        );

        // Truncate to terminal width
        let display = if line.len() > cols.saturating_sub(1) {
            line.chars().take(cols.saturating_sub(1)).collect::<String>()
        } else {
            line
        };

        print!("{}", display);
    }
}

impl DellijStatusPlugin {
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

            let slug = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            if slug.is_empty() {
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

            // Derive a short label from the slug suffix (last token after last '-')
            let short_label = derive_short_label(&slug);

            agents.push(AgentInfo {
                slug: slug.clone(),
                short_label,
                status: parsed.status,
            });
        }

        // Sort by slug for stable display order
        agents.sort_by(|a, b| a.slug.cmp(&b.slug));

        agents
    }
}

/// Derive a 2-char short label from a slug.
/// The slug format is "{words}-{agent-suffix}" e.g. "fix-auth-claude-code".
/// We try to match known agent suffixes, falling back to first 2 chars.
fn derive_short_label(slug: &str) -> String {
    let known_suffixes: &[(&str, &str)] = &[
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

    for (suffix, label) in known_suffixes {
        if slug.ends_with(suffix) || slug.contains(&format!("-{}", suffix)) {
            return label.to_string();
        }
    }

    // Fall back to first 2 chars of slug
    slug.chars().take(2).collect()
}

register_plugin!(DellijStatusPlugin);
