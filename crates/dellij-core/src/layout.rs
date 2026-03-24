use crate::types::Workspace;
use crate::{render_agent_command, shell_escape};

/// Built-in layout names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuiltinLayout {
    /// 3-pane: agent command | shell | git status
    Default,
    /// 2-pane: agent command | shell
    Minimal,
    /// 4-pane: agent command | shell | git diff | ports/urls
    Full,
    /// Just the agent command in a single pane
    AgentOnly,
}

impl BuiltinLayout {
    pub fn name(&self) -> &str {
        match self {
            Self::Default => "default",
            Self::Minimal => "minimal",
            Self::Full => "full",
            Self::AgentOnly => "agent-only",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "default" => Some(Self::Default),
            "minimal" => Some(Self::Minimal),
            "full" => Some(Self::Full),
            "agent-only" => Some(Self::AgentOnly),
            _ => None,
        }
    }
}

pub struct LayoutRenderer;

impl LayoutRenderer {
    pub fn render(
        workspace: &Workspace,
        project_root: &str,
        named_template: Option<&str>,
    ) -> String {
        if let Some(template) = named_template {
            return Self::render_template(template, workspace);
        }

        let layout_name = workspace.layout.as_deref().unwrap_or("default");
        match BuiltinLayout::from_name(layout_name) {
            Some(BuiltinLayout::Default) | None => Self::render_default(workspace, project_root),
            Some(BuiltinLayout::Minimal) => Self::render_minimal(workspace, project_root),
            Some(BuiltinLayout::Full) => Self::render_full(workspace, project_root),
            Some(BuiltinLayout::AgentOnly) => Self::render_agent_only(workspace, project_root),
        }
    }

    fn env_block(workspace: &Workspace, project_root: &str) -> String {
        format!(
            r#"    env {{
      DELLIJ_SLUG "{slug}"
      DELLIJ_AGENT "{agent}"
      DELLIJ_BRANCH "{branch}"
      DELLIJ_BASE_BRANCH "{base}"
      DELLIJ_WORKTREE_PATH "{cwd}"
      DELLIJ_ROOT "{root}"
      DELLIJ_PROMPT "{prompt}"
    }}"#,
            slug = workspace.slug,
            agent = workspace.agent,
            branch = workspace.branch_name,
            base = workspace.base_branch,
            cwd = workspace.worktree_path,
            root = project_root,
            prompt = shell_escape(&workspace.prompt),
        )
    }

    fn render_default(workspace: &Workspace, project_root: &str) -> String {
        let cwd = &workspace.worktree_path;
        let agent_cmd =
            shell_escape(workspace.last_command.as_deref().unwrap_or("bash"));
        let env = Self::env_block(workspace, project_root);
        format!(
            r#"layout {{
  pane split_direction="vertical" {{
    pane command="bash" cwd="{cwd}" {{
      args "-lc" "{agent_cmd}"
{env}
    }}
    pane command="bash" cwd="{cwd}" {{
{env}
    }}
    pane command="bash" cwd="{cwd}" {{
      args "-lc" "git status --short --branch && git log --oneline -5"
{env}
    }}
  }}
}}
"#
        )
    }

    fn render_minimal(workspace: &Workspace, project_root: &str) -> String {
        let cwd = &workspace.worktree_path;
        let agent_cmd =
            shell_escape(workspace.last_command.as_deref().unwrap_or("bash"));
        let env = Self::env_block(workspace, project_root);
        format!(
            r#"layout {{
  pane split_direction="vertical" {{
    pane command="bash" cwd="{cwd}" {{
      args "-lc" "{agent_cmd}"
{env}
    }}
    pane command="bash" cwd="{cwd}" {{
{env}
    }}
  }}
}}
"#
        )
    }

    fn render_full(workspace: &Workspace, project_root: &str) -> String {
        let cwd = &workspace.worktree_path;
        let agent_cmd =
            shell_escape(workspace.last_command.as_deref().unwrap_or("bash"));
        let env = Self::env_block(workspace, project_root);
        let ports_cmd = if workspace.ports.is_empty() {
            "echo 'no ports configured'".to_string()
        } else {
            let ports: Vec<String> = workspace.ports.iter().map(|p| p.to_string()).collect();
            format!("echo 'ports: {}'; watch -n2 'ss -tlnp 2>/dev/null | grep -E \"{}\"'",
                ports.join(", "),
                ports.join("|"))
        };
        format!(
            r#"layout {{
  pane split_direction="vertical" {{
    pane split_direction="horizontal" {{
      pane command="bash" cwd="{cwd}" {{
        args "-lc" "{agent_cmd}"
{env}
      }}
      pane command="bash" cwd="{cwd}" {{
{env}
      }}
    }}
    pane split_direction="horizontal" {{
      pane command="bash" cwd="{cwd}" {{
        args "-lc" "git diff {base}...HEAD --stat"
{env}
      }}
      pane command="bash" cwd="{cwd}" {{
        args "-lc" "{ports_cmd}"
{env}
      }}
    }}
  }}
}}
"#,
            base = workspace.base_branch,
            ports_cmd = shell_escape(&ports_cmd),
        )
    }

    fn render_agent_only(workspace: &Workspace, project_root: &str) -> String {
        let cwd = &workspace.worktree_path;
        let agent_cmd =
            shell_escape(workspace.last_command.as_deref().unwrap_or("bash"));
        let env = Self::env_block(workspace, project_root);
        format!(
            r#"layout {{
  pane command="bash" cwd="{cwd}" {{
    args "-lc" "{agent_cmd}"
{env}
  }}
}}
"#
        )
    }

    /// Render a user-supplied template string.
    /// Placeholders: `{cwd}`, `{agent_cmd}`, `{slug}`, `{branch}`, `{base_branch}`, `{prompt}`
    fn render_template(template: &str, workspace: &Workspace) -> String {
        let agent_cmd = shell_escape(workspace.last_command.as_deref().unwrap_or("bash"));
        template
            .replace("{cwd}", workspace.worktree_path.as_str())
            .replace("{agent_cmd}", &agent_cmd)
            .replace("{slug}", &workspace.slug)
            .replace("{branch}", &workspace.branch_name)
            .replace("{base_branch}", &workspace.base_branch)
            .replace("{prompt}", &shell_escape(&workspace.prompt))
    }
}

/// Render the agent command for a workspace (exported for use by CLI and GUI).
pub fn workspace_agent_command(workspace: &Workspace) -> String {
    render_agent_command(&workspace.agent, &workspace.prompt)
}
