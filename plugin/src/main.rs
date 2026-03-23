use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;
use zellij_tile::prelude::*;
use zellij_tile::ui_components::{print_ribbon, Text};

#[derive(Default)]
struct DellijStatusPlugin {
    agents: Vec<AgentInfo>,
    config_dir: String,
    #[allow(dead_code)]
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
    #[allow(dead_code)]
    slug: Option<String>,
}

register_plugin!(DellijStatusPlugin);

impl ZellijPlugin for DellijStatusPlugin {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.config_dir = configuration
            .get("config_dir")
            .cloned()
            .unwrap_or_default();

        // This plugin is only a status ribbon; it should never take keyboard focus.
        set_selectable(false);

        request_permission(&[PermissionType::ReadApplicationState]);

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
            }
            Event::TabUpdate(tab_infos) => {
                for tab in tab_infos {
                    if tab.active {
                        self.active_tab = Some(tab.name);
                        should_render = true;
                        break;
                    }
                }
            }
            Event::PermissionRequestResult(status) => {
                if status == PermissionStatus::Granted {
                    should_render = true;
                }
            }
            _ => {}
        }
        should_render
    }

    fn render(&mut self, _rows: usize, _cols: usize) {
        if self.config_dir.is_empty() {
            println!("dellij: no config_dir configured");
            return;
        }

        let agent_count = self.agents.len();
        if agent_count == 0 {
            return;
        }

        let mut combined_text = String::new();
        combined_text.push_str(" dellij ");

        // Tracking byte ranges for colors
        // Format: (index_level, start_byte, end_byte)
        let mut styling = vec![(2, 0, 8)]; // Brand in Green/Cyan

        let mut current_pos = 8;
        for agent in &self.agents {
            let is_active = self.active_tab.as_ref() == Some(&agent.slug);
            let status_idx = status_to_index(&agent.status);

            let segment = format!(" {} ● {} ", agent.short_label, agent.status);
            let start = current_pos;
            let end = current_pos + segment.len();

            // Base color for the agent segment
            if is_active {
                styling.push((0, start, end)); // Active in Primary
            } else {
                styling.push((1, start, end)); // Inactive in Secondary
            }

            // Dot color override
            // Dot is at space(1) + label(2) + space(1) = 4 bytes from start of segment
            // Dot "●" is 3 bytes
            styling.push((status_idx, start + 4, start + 7));

            combined_text.push_str(&segment);
            current_pos = end;
        }

        let mut text_component = Text::new(combined_text);
        for (idx, start, end) in styling {
            text_component = text_component.color_range(idx, start..end);
        }

        print_ribbon(text_component);
    }
}

fn status_to_index(status: &str) -> usize {
    match status {
        "working" => 3, // Yellow
        "waiting" => 7, // Cyan
        "error" => 4,   // Red
        "done" => 2,    // Green
        _ => 2,
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

            let short_label = derive_short_label(&slug);

            agents.push(AgentInfo {
                slug: slug.clone(),
                short_label,
                status: parsed.status,
            });
        }

        agents.sort_by(|a, b| a.slug.cmp(&b.slug));
        agents
    }
}

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

    slug.chars().take(2).collect()
}
