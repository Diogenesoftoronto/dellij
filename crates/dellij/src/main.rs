use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::process::Command;

use anyhow::{bail, Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use chrono::Utc;
use clap::{Args, Parser, Subcommand};

use dellij_core::{
    ahead_behind, command_exists, git, git_output, inside_zellij, render_agent_command,
    slugify, write_json, yes_no, Bookmark, Config, LayoutRenderer,
    PipeCommand, StatusFile, Workspace, WorkspaceStatus,
    convex::ConvexSyncClient,
};

// ── CLI ───────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let project_root = dellij_core::git::resolve_project_root(cli.project_root)?;
    let mut app = App::load_or_init(project_root).await?;
    app.run(cli.command.unwrap_or(Commands::Open(OpenArgs::default()))).await
}

#[derive(Debug, Parser)]
#[command(name = "dellij")]
#[command(about = "Rust-first terminal workspace manager for Zellij")]
struct Cli {
    #[arg(long, global = true)]
    project_root: Option<Utf8PathBuf>,
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Initialize dellij in the current project
    Init,
    /// Open the project session or a specific workspace tab
    Open(OpenArgs),
    /// Create a new workspace (git worktree + layout + optional setup hook)
    New(NewArgs),
    /// Import an existing worktree directory as a workspace
    Import(ImportArgs),
    /// List all tracked workspaces
    List,
    /// Print workspace JSON
    Show(WorkspaceRef),
    /// Run a bookmark or command inside a workspace
    Run(RunArgs),
    /// Send text to a workspace's shell pane via the Zellij plugin
    Send(SendArgs),
    /// Manage command bookmarks
    Bookmark {
        #[command(subcommand)]
        command: BookmarkCommand,
    },
    /// Update workspace status
    Status(StatusArgs),
    /// Close a workspace (removes worktree, optionally runs teardown hook)
    Close(CloseArgs),
    /// Print the generated KDL layout for a workspace
    Layout(WorkspaceRef),
    /// Manage GitHub PR associations
    Pr {
        #[command(subcommand)]
        command: PrCommand,
    },
    /// Open the workspace in an editor (code, cursor, zed, …)
    Edit(EditArgs),
    /// Launch the dellij GUI (requires dellij-gui binary)
    Ui,
    /// Diagnostic check
    Doctor,
}

// ── argument types ────────────────────────────────────────────────────────────

#[derive(Debug, Args)]
struct NewArgs {
    prompt: String,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    agent: Option<String>,
    #[arg(long)]
    base_branch: Option<String>,
    /// Use an existing branch instead of creating a new one
    #[arg(long)]
    use_branch: Option<String>,
    /// Create worktree from the branch of a GitHub PR (requires gh CLI)
    #[arg(long)]
    base_pr: Option<u32>,
    #[arg(long)]
    port: Vec<u16>,
    #[arg(long)]
    url: Vec<String>,
    /// Named layout (default, minimal, full, agent-only, or a custom name from settings.layouts)
    #[arg(long)]
    layout: Option<String>,
    #[arg(long)]
    setup: bool,
    #[arg(long)]
    open: bool,
}

#[derive(Debug, Args, Default)]
struct OpenArgs {
    slug: Option<String>,
    #[arg(long)]
    create: bool,
    #[arg(long)]
    prompt: Option<String>,
    #[arg(long)]
    agent: Option<String>,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    port: Vec<u16>,
    #[arg(long)]
    url: Vec<String>,
    #[arg(long)]
    setup: bool,
}

#[derive(Debug, Args)]
struct ImportArgs {
    /// Path to the existing worktree directory
    path: Utf8PathBuf,
    /// Workspace slug (derived from directory name if omitted)
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    agent: Option<String>,
    #[arg(long)]
    prompt: Option<String>,
    #[arg(long)]
    port: Vec<u16>,
    #[arg(long)]
    url: Vec<String>,
    #[arg(long)]
    layout: Option<String>,
}

#[derive(Debug, Args)]
struct WorkspaceRef {
    slug: String,
}

#[derive(Debug, Args)]
struct RunArgs {
    slug: String,
    target: String,
    #[arg(long)]
    floating: bool,
    #[arg(long)]
    close_on_exit: bool,
}

#[derive(Debug, Args)]
struct SendArgs {
    slug: String,
    /// Text to send (newline appended automatically)
    text: String,
}

#[derive(Debug, Subcommand)]
enum BookmarkCommand {
    Add(BookmarkAddArgs),
    List,
    Remove(BookmarkRef),
}

#[derive(Debug, Args)]
struct BookmarkAddArgs {
    name: String,
    command: String,
    #[arg(long)]
    description: Option<String>,
}

#[derive(Debug, Args)]
struct BookmarkRef {
    name: String,
}

#[derive(Debug, Args)]
struct StatusArgs {
    slug: String,
    state: WorkspaceStatus,
    #[arg(long)]
    note: Option<String>,
}

#[derive(Debug, Args)]
struct CloseArgs {
    slug: String,
    #[arg(long)]
    keep_worktree: bool,
    #[arg(long)]
    teardown: bool,
}

#[derive(Debug, Subcommand)]
enum PrCommand {
    /// Associate a GitHub PR number with a workspace
    Set(PrSetArgs),
    /// Open the associated PR in the browser
    Open(WorkspaceRef),
    /// Show PR status via gh CLI
    Status(WorkspaceRef),
}

#[derive(Debug, Args)]
struct PrSetArgs {
    slug: String,
    number: u32,
}

#[derive(Debug, Args)]
struct EditArgs {
    slug: String,
    /// Editor binary (default: $EDITOR, then code, cursor, zed in order)
    #[arg(long)]
    editor: Option<String>,
}

// ── App ───────────────────────────────────────────────────────────────────────

#[derive(Debug)]
struct App {
    project_root: Utf8PathBuf,
    state_dir: Utf8PathBuf,
    config_path: Utf8PathBuf,
    config: Config,
    convex_client: Option<ConvexSyncClient>,
}

impl App {
    async fn load_or_init(project_root: Utf8PathBuf) -> Result<Self> {
        let state_dir = project_root.join(".dellij");
        for sub in &["hooks", "status", "layouts"] {
            fs::create_dir_all(state_dir.join(sub))
                .with_context(|| format!("creating {}", state_dir.join(sub)))?;
        }

        let config_path = state_dir.join("dellij.json");
        let config = if config_path.exists() {
            let raw = fs::read_to_string(&config_path)
                .with_context(|| format!("reading {}", config_path))?;
            serde_json::from_str(&raw).with_context(|| format!("parsing {}", config_path))?
        } else {
            let config = Config::new(project_root.clone());
            write_json(&config_path, &config)?;
            config
        };

        let mut convex_client = if let Ok(url) = env::var("CONVEX_URL") {
            Some(ConvexSyncClient::new(&url).await?)
        } else {
            None
        };

        if let (Some(client), Ok(token)) = (&mut convex_client, env::var("CONVEX_AUTH_TOKEN")) {
            client.set_auth(Some(token)).await;
        }

        Ok(Self { project_root, state_dir, config_path, config, convex_client })
    }

    async fn run(&mut self, command: Commands) -> Result<()> {
        match command {
            Commands::Init => self.init().await,
            Commands::Open(args) => self.open(args).await,
            Commands::New(args) => self.new_workspace(args).await,
            Commands::Import(args) => self.import(args).await,
            Commands::List => self.list(),
            Commands::Show(r) => self.show(&r.slug),
            Commands::Run(args) => self.run_in_workspace(args),
            Commands::Send(args) => self.send(args),
            Commands::Bookmark { command } => self.bookmark(command),
            Commands::Status(args) => self.update_status(args).await,
            Commands::Close(args) => self.close(args),
            Commands::Layout(r) => self.print_layout(&r.slug),
            Commands::Pr { command } => self.pr(command),
            Commands::Edit(args) => self.edit(args),
            Commands::Ui => self.launch_ui(),
            Commands::Doctor => self.doctor(),
        }
    }

    // ── init ──────────────────────────────────────────────────────────────────

    async fn init(&mut self) -> Result<()> {
        self.ensure_zellij_available()?;
        self.save()?;
        println!("Initialized dellij at {}", self.state_dir);
        println!("  workspaces root : {}", self.workspace_root());
        println!("  hooks dir       : {}", self.state_dir.join("hooks"));
        println!("  session name    : {}", self.session_name());
        Ok(())
    }

    // ── new ───────────────────────────────────────────────────────────────────

    async fn new_workspace(&mut self, args: NewArgs) -> Result<()> {
        self.ensure_zellij_available()?;
        let should_open = args.open;
        let workspace = self.new_workspace_from_args(args).await?;
        if should_open {
            self.open_workspace_tab(&workspace)?;
        }
        Ok(())
    }

    async fn new_workspace_from_args(&mut self, args: NewArgs) -> Result<Workspace> {
        let agent = args
            .agent
            .unwrap_or_else(|| self.config.settings.default_agent.clone());
        let slug = slugify(args.name.as_deref().unwrap_or(&args.prompt), &agent);
        if self.config.workspaces.iter().any(|w| w.slug == slug) {
            bail!("workspace '{slug}' already exists");
        }

        // Resolve base branch: --base-pr → gh, --base-branch flag, then detect
        let base_branch = match &args.base_pr {
            Some(pr) => self.gh_pr_branch(*pr)?,
            None => match args.base_branch {
                Some(b) => b,
                None => dellij_core::git::detect_base_branch(&self.project_root)
                    .unwrap_or_else(|_| self.config.settings.base_branch.clone()),
            },
        };

        let branch_name = match &args.use_branch {
            Some(b) => b.clone(),
            None => format!("{}{}", self.config.settings.branch_prefix, slug),
        };

        let worktree_path = self.workspace_root().join(&slug);
        fs::create_dir_all(self.workspace_root())
            .with_context(|| format!("creating {}", self.workspace_root()))?;

        if args.use_branch.is_some() {
            // Attach to existing branch
            git(
                &self.project_root,
                &["worktree", "add", worktree_path.as_str(), &branch_name],
            )
            .with_context(|| format!("adding worktree for existing branch {branch_name}"))?;
        } else {
            git(
                &self.project_root,
                &[
                    "worktree", "add", "-b", &branch_name,
                    worktree_path.as_str(), &base_branch,
                ],
            )
            .with_context(|| format!("creating git worktree for {slug}"))?;
        }

        let pr_number = args.base_pr;
        let pr_url = pr_number
            .and_then(|n| self.gh_pr_url(n).ok());

        let workspace = Workspace {
            slug: slug.clone(),
            prompt: args.prompt.clone(),
            agent: agent.clone(),
            branch_name: branch_name.clone(),
            base_branch: base_branch.clone(),
            worktree_path: worktree_path.clone(),
            status: WorkspaceStatus::Working,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ports: args.port,
            urls: args.url,
            last_command: Some(render_agent_command(&agent, &args.prompt)),
            notes: Vec::new(),
            pr_number,
            pr_url,
            layout: args.layout,
        };

        self.write_status_file(&workspace).await?;
        self.write_layout_file(&workspace)?;
        self.config.workspaces.push(workspace.clone());
        self.save()?;

        if let Some(client) = &mut self.convex_client {
            let _ = client.push_workspace(&workspace).await;
        }

        if args.setup {
            self.run_hook("workspace_setup", &workspace)?;
        }

        println!("Created workspace {}", workspace.slug);
        println!("  path          : {}", workspace.worktree_path);
        println!("  branch        : {}", workspace.branch_name);
        println!("  agent command : {}", workspace.last_command.as_deref().unwrap_or(""));
        if let Some(pr) = workspace.pr_number {
            println!("  PR            : #{pr}");
        }
        Ok(workspace)
    }

    // ── import ────────────────────────────────────────────────────────────────

    async fn import(&mut self, args: ImportArgs) -> Result<()> {
        let path = args.path.canonicalize_utf8()
            .with_context(|| format!("resolving path {}", args.path))?;

        // Detect branch from the worktree
        let branch = git_output(&path, &["branch", "--show-current"])
            .unwrap_or_else(|_| "unknown".to_string());

        let base_branch = dellij_core::git::detect_base_branch(&self.project_root)
            .unwrap_or_else(|_| self.config.settings.base_branch.clone());

        let dir_name = path
            .file_name()
            .unwrap_or(path.as_str());

        let agent = args.agent.unwrap_or_else(|| self.config.settings.default_agent.clone());
        let slug = slugify(
            args.name.as_deref().unwrap_or(dir_name),
            &agent,
        );

        if self.config.workspaces.iter().any(|w| w.slug == slug) {
            bail!("workspace '{slug}' already exists");
        }

        let prompt = args.prompt.unwrap_or_else(|| format!("imported from {dir_name}"));
        let workspace = Workspace {
            slug: slug.clone(),
            prompt: prompt.clone(),
            agent: agent.clone(),
            branch_name: branch.clone(),
            base_branch,
            worktree_path: path.clone(),
            status: WorkspaceStatus::Working,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ports: args.port,
            urls: args.url,
            last_command: Some(render_agent_command(&agent, &prompt)),
            notes: Vec::new(),
            pr_number: None,
            pr_url: None,
            layout: args.layout,
        };

        self.write_status_file(&workspace).await?;
        self.write_layout_file(&workspace)?;
        self.config.workspaces.push(workspace.clone());
        self.save()?;

        if let Some(client) = &mut self.convex_client {
            let _ = client.push_workspace(&workspace).await;
        }

        println!("Imported workspace {slug}");
        println!("  path   : {path}");
        println!("  branch : {branch}");
        Ok(())
    }

    // ── open ──────────────────────────────────────────────────────────────────

    async fn open(&mut self, args: OpenArgs) -> Result<()> {
        self.ensure_zellij_available()?;

        if args.create {
            let prompt = args.prompt.clone().context("--create requires --prompt")?;
            let slug = {
                let new_args = NewArgs {
                    prompt,
                    name: args.name.clone(),
                    agent: args.agent.clone(),
                    base_branch: None,
                    use_branch: None,
                    base_pr: None,
                    port: args.port.clone(),
                    url: args.url.clone(),
                    layout: None,
                    setup: args.setup,
                    open: false,
                };
                self.new_workspace_from_args(new_args).await?.slug
            };
            return self.open_workspace(&slug);
        }

        match args.slug {
            Some(slug) => self.open_workspace(&slug),
            None => self.open_control_session(),
        }
    }

    // ── list ──────────────────────────────────────────────────────────────────

    fn list(&self) -> Result<()> {
        if self.config.workspaces.is_empty() {
            println!("No workspaces tracked for {}", self.project_root);
            return Ok(());
        }

        for ws in &self.config.workspaces {
            let ports = if ws.ports.is_empty() {
                "-".to_string()
            } else {
                ws.ports.iter().map(u16::to_string).collect::<Vec<_>>().join(",")
            };

            let ab = ahead_behind(&self.project_root, &ws.branch_name, &ws.base_branch)
                .map(|ab| ab.to_string())
                .unwrap_or_default();

            let pr = ws.pr_number
                .map(|n| format!("PR#{n}"))
                .unwrap_or_default();

            let attn = if ws.status.needs_attention() { "!" } else { " " };

            println!(
                "{attn} {:16} {:10} {:12} {:8} {:8} {}",
                ws.slug,
                ws.status,
                ws.agent,
                ab,
                ports,
                pr,
            );
        }
        Ok(())
    }

    // ── show ──────────────────────────────────────────────────────────────────

    fn show(&self, slug: &str) -> Result<()> {
        let ws = self.workspace(slug)?;
        println!("{}", serde_json::to_string_pretty(ws)?);
        Ok(())
    }

    // ── bookmark ──────────────────────────────────────────────────────────────

    fn bookmark(&mut self, command: BookmarkCommand) -> Result<()> {
        match command {
            BookmarkCommand::Add(args) => {
                self.config.bookmarks.retain(|b| b.name != args.name);
                self.config.bookmarks.push(Bookmark {
                    name: args.name,
                    command: args.command,
                    description: args.description,
                });
                self.config.bookmarks.sort_by(|a, b| a.name.cmp(&b.name));
                self.save()?;
                println!("Saved bookmark");
            }
            BookmarkCommand::List => {
                if self.config.bookmarks.is_empty() {
                    println!("No bookmarks saved");
                } else {
                    for bm in &self.config.bookmarks {
                        println!(
                            "{:16} {}{}",
                            bm.name,
                            bm.command,
                            bm.description
                                .as_ref()
                                .map(|d| format!("  # {d}"))
                                .unwrap_or_default()
                        );
                    }
                }
            }
            BookmarkCommand::Remove(r) => {
                let before = self.config.bookmarks.len();
                self.config.bookmarks.retain(|b| b.name != r.name);
                if self.config.bookmarks.len() == before {
                    bail!("bookmark '{}' not found", r.name);
                }
                self.save()?;
                println!("Removed bookmark {}", r.name);
            }
        }
        Ok(())
    }

    // ── run ───────────────────────────────────────────────────────────────────

    fn run_in_workspace(&self, args: RunArgs) -> Result<()> {
        let ws = self.workspace(&args.slug)?;
        let command = self.resolve_bookmark_or_command(&args.target)?;

        if inside_zellij() {
            let mut zellij_args = vec!["run"];
            if args.floating { zellij_args.push("--floating"); }
            if args.close_on_exit { zellij_args.push("--close-on-exit"); }
            zellij_args.extend_from_slice(&[
                "--cwd", ws.worktree_path.as_str(),
                "--name", &args.target,
                "--", "bash", "-lc", &command,
            ]);
            self.zellij(&zellij_args)?;
        } else {
            let status = Command::new("bash")
                .args(["-lc", &command])
                .current_dir(&ws.worktree_path)
                .envs(env_map(ws, &self.state_dir, &self.project_root))
                .status()
                .with_context(|| format!("running command in {}", ws.worktree_path))?;
            if !status.success() {
                bail!("command exited with {}", status);
            }
        }
        Ok(())
    }

    // ── send ──────────────────────────────────────────────────────────────────

    fn send(&self, args: SendArgs) -> Result<()> {
        self.workspace(&args.slug)?; // validate
        let text = if args.text.ends_with('\n') {
            args.text.clone()
        } else {
            format!("{}\n", args.text)
        };
        let cmd = PipeCommand::Send { slug: args.slug.clone(), text };
        self.pipe_to_plugin(&cmd).map(|_| ())
    }

    // ── status ────────────────────────────────────────────────────────────────

    async fn update_status(&mut self, args: StatusArgs) -> Result<()> {
        let ws = self.workspace_mut(&args.slug)?;
        ws.status = args.state;
        ws.updated_at = Utc::now();
        if let Some(note) = args.note {
            ws.notes.push(note);
        }
        let snapshot = ws.clone();
        self.write_status_file(&snapshot).await?;
        self.save()?;
        println!("{} -> {}", snapshot.slug, snapshot.status);

        // Also notify the plugin if we're inside Zellij
        if inside_zellij() {
            let _ = self.pipe_to_plugin(&PipeCommand::Status {
                slug: snapshot.slug.clone(),
                status: snapshot.status.to_string(),
            });
        }
        Ok(())
    }

    // ── close ─────────────────────────────────────────────────────────────────

    fn close(&mut self, args: CloseArgs) -> Result<()> {
        let idx = self
            .config
            .workspaces
            .iter()
            .position(|w| w.slug == args.slug)
            .with_context(|| format!("workspace '{}' not found", args.slug))?;
        let ws = self.config.workspaces[idx].clone();

        if args.teardown {
            self.run_hook("workspace_teardown", &ws)?;
        }

        if !args.keep_worktree {
            git(
                &self.project_root,
                &["worktree", "remove", "--force", ws.worktree_path.as_str()],
            )
            .with_context(|| format!("removing worktree {}", ws.worktree_path))?;
        }

        let status_path = self
            .state_dir
            .join("status")
            .join(format!("{}.json", ws.slug));
        if status_path.exists() {
            fs::remove_file(&status_path)
                .with_context(|| format!("removing {}", status_path))?;
        }

        self.config.workspaces.remove(idx);
        self.save()?;
        println!("Closed workspace {}", ws.slug);
        Ok(())
    }

    // ── layout ────────────────────────────────────────────────────────────────

    fn print_layout(&self, slug: &str) -> Result<()> {
        let ws = self.workspace(slug)?;
        let template = ws.layout.as_deref()
            .and_then(|l| self.config.settings.layouts.get(l).map(String::as_str));
        println!(
            "{}",
            LayoutRenderer::render(ws, self.project_root.as_str(), template)
        );
        Ok(())
    }

    // ── pr ────────────────────────────────────────────────────────────────────

    fn pr(&mut self, command: PrCommand) -> Result<()> {
        match command {
            PrCommand::Set(args) => {
                let url = self.gh_pr_url(args.number).ok();
                let ws = self.workspace_mut(&args.slug)?;
                ws.pr_number = Some(args.number);
                ws.pr_url = url.clone();
                ws.updated_at = Utc::now();
                println!("Set PR #{} on workspace {}", args.number, args.slug);
                if let Some(u) = url {
                    println!("  {u}");
                }
                self.save()?;
            }
            PrCommand::Open(r) => {
                let ws = self.workspace(&r.slug)?;
                let url = ws
                    .pr_url
                    .clone()
                    .or_else(|| ws.pr_number.map(|n| format!("https://github.com/pulls/{n}")))
                    .with_context(|| format!("no PR associated with workspace {}", r.slug))?;
                open_url(&url)?;
            }
            PrCommand::Status(r) => {
                let ws = self.workspace(&r.slug)?.clone();
                let pr = ws.pr_number.with_context(|| {
                    format!("no PR associated with workspace {}", r.slug)
                })?;
                let status = Command::new("gh")
                    .args(["pr", "view", &pr.to_string(), "--json",
                           "number,state,title,url,statusCheckRollup"])
                    .current_dir(&self.project_root)
                    .status()
                    .context("running gh pr view")?;
                if !status.success() {
                    bail!("gh pr view failed");
                }
            }
        }
        Ok(())
    }

    // ── edit ──────────────────────────────────────────────────────────────────

    fn edit(&self, args: EditArgs) -> Result<()> {
        let ws = self.workspace(&args.slug)?;
        let editor = args.editor
            .or_else(|| env::var("EDITOR").ok())
            .or_else(|| {
                ["code", "cursor", "zed", "idea", "vim", "nvim"]
                    .iter()
                    .find(|e| command_exists(e))
                    .map(|s| s.to_string())
            })
            .context("no editor found; set $EDITOR or pass --editor")?;
        dellij_core::git::open_in_editor(&editor, &ws.worktree_path)?;
        println!("Opened {} in {editor}", ws.worktree_path);
        Ok(())
    }

    // ── ui ────────────────────────────────────────────────────────────────────

    fn launch_ui(&self) -> Result<()> {
        let gui_bin = which_binary("dellij-gui")
            .context("dellij-gui binary not found; install with: cargo install --path crates/dellij-gui")?;
        Command::new(gui_bin)
            .arg("--project-root")
            .arg(self.project_root.as_str())
            .spawn()
            .context("launching dellij-gui")?;
        Ok(())
    }

    // ── doctor ────────────────────────────────────────────────────────────────

    fn doctor(&self) -> Result<()> {
        let git_ok = command_exists("git");
        let zellij_ok = command_exists("zellij");
        let gh_ok = command_exists("gh");
        let gui_ok = which_binary("dellij-gui").is_some();

        println!("project_root    : {}", self.project_root);
        println!("session         : {}", self.session_name());
        println!("git             : {}", yes_no(git_ok));
        println!("zellij          : {}", yes_no(zellij_ok));
        println!("gh (GitHub CLI) : {}", yes_no(gh_ok));
        println!("dellij-gui      : {}", yes_no(gui_ok));
        println!("hooks_dir       : {}", yes_no(self.state_dir.join("hooks").exists()));
        println!("workspace_count : {}", self.config.workspaces.len());
        println!("bookmark_count  : {}", self.config.bookmarks.len());

        self.print_hook_snippets();
        Ok(())
    }

    fn print_hook_snippets(&self) {
        let slug_hint = self.config.workspaces.first()
            .map(|w| w.slug.as_str())
            .unwrap_or("<slug>");

        println!();
        println!("── Claude Code hooks (~/.claude/settings.json) ─────────────────");
        println!(r#"{{
  "hooks": {{
    "Notification": [{{
      "matcher": "",
      "hooks": [{{
        "type": "command",
        "command": "dellij status $DELLIJ_SLUG waiting 2>/dev/null || true"
      }}]
    }}],
    "Stop": [{{
      "matcher": "",
      "hooks": [{{
        "type": "command",
        "command": "dellij status $DELLIJ_SLUG done 2>/dev/null || true"
      }}]
    }}]
  }}
}}"#);

        println!();
        println!("── OpenAI Codex (~/.codex/config.toml) ─────────────────────────");
        println!(r#"notify = ["bash", "-c", "dellij status ${{DELLIJ_SLUG:-{slug_hint}}} waiting 2>/dev/null || true"]"#);

        println!();
        println!("── OpenCode plugin (.opencode/plugins/dellij-notify.js) ─────────");
        println!(r#"export const DellijNotifyPlugin = async ({{ $ }}) => {{
  return {{
    event: async ({{ event }}) => {{
      if (event.type === "session.idle") {{
        await $`dellij status ${{process.env.DELLIJ_SLUG || "{slug_hint}"}} waiting`.catch(() => {{}});
      }}
    }},
  }};
}};"#);
    }

    // ── session helpers ───────────────────────────────────────────────────────

    fn workspace_root(&self) -> Utf8PathBuf {
        self.project_root.join(&self.config.settings.workspace_root)
    }

    fn workspace(&self, slug: &str) -> Result<&Workspace> {
        self.config
            .workspaces
            .iter()
            .find(|w| w.slug == slug)
            .with_context(|| format!("workspace '{slug}' not found"))
    }

    fn workspace_mut(&mut self, slug: &str) -> Result<&mut Workspace> {
        self.config
            .workspaces
            .iter_mut()
            .find(|w| w.slug == slug)
            .with_context(|| format!("workspace '{slug}' not found"))
    }

    async fn write_status_file(&mut self, ws: &Workspace) -> Result<()> {
        let sf = StatusFile::from_workspace(ws);
        write_json(
            &self.state_dir.join("status").join(format!("{}.json", ws.slug)),
            &sf,
        )?;

        if let Some(client) = &mut self.convex_client {
            let _ = client.push_status(&sf).await;
        }
        Ok(())
    }

    fn write_layout_file(&self, ws: &Workspace) -> Result<Utf8PathBuf> {
        let path = self.state_dir.join("layouts").join(format!("{}.kdl", ws.slug));
        let template = ws.layout.as_deref()
            .and_then(|l| self.config.settings.layouts.get(l).map(String::as_str));
        fs::write(
            &path,
            LayoutRenderer::render(ws, self.project_root.as_str(), template),
        )
            .with_context(|| format!("writing {}", path))?;
        Ok(path)
    }

    fn open_control_session(&self) -> Result<()> {
        if inside_zellij() {
            self.zellij_action(&["new-tab", "--name", "dellij", "--cwd",
                                 self.project_root.as_str()])?;
            return Ok(());
        }
        let session = self.session_name();
        if self.session_exists(&session)? {
            self.zellij(&["attach", &session])?;
        } else {
            self.zellij(&["--session", &session])?;
        }
        Ok(())
    }

    fn open_workspace(&mut self, slug: &str) -> Result<()> {
        let ws = self.workspace(slug)?.clone();
        self.open_control_session_if_needed()?;
        self.open_workspace_tab(&ws)
    }

    fn open_control_session_if_needed(&self) -> Result<()> {
        if inside_zellij() { return Ok(()); }
        let session = self.session_name();
        if !self.session_exists(&session)? {
            self.zellij(&["--session", &session, "--new-session-with-layout", "default"])?;
        }
        Ok(())
    }

    fn open_workspace_tab(&self, ws: &Workspace) -> Result<()> {
        // Try the plugin pipe first (handles tab dedup)
        if inside_zellij()
            && self.pipe_to_plugin(&PipeCommand::Open { slug: ws.slug.clone() })?
        {
            return Ok(());
        }

        let layout_path = self.write_layout_file(ws)?;
        let session = self.session_name();

        if self.session_exists(&session)? {
            self.zellij(&[
                "action", "--session", &session,
                "new-tab", "--layout", layout_path.as_str(),
                "--name", &ws.slug,
            ])?;
            self.zellij(&["attach", &session])?;
        } else {
            self.zellij(&[
                "--session", &session,
                "--new-session-with-layout", layout_path.as_str(),
            ])?;
        }
        Ok(())
    }

    fn session_name(&self) -> String {
        format!("dellij-{}", slugify(self.project_root.as_str(), "session"))
    }

    fn session_exists(&self, session: &str) -> Result<bool> {
        let output = Command::new("zellij")
            .args(["list-sessions"])
            .current_dir(&self.project_root)
            .output()
            .context("listing zellij sessions")?;
        if !output.status.success() {
            bail!("zellij list-sessions failed");
        }
        let stdout = String::from_utf8(output.stdout)?;
        Ok(stdout.lines().any(|l| l.split_whitespace().next() == Some(session)))
    }

    fn zellij(&self, args: &[&str]) -> Result<()> {
        let status = Command::new("zellij")
            .args(args)
            .current_dir(&self.project_root)
            .status()
            .with_context(|| format!("running zellij {}", args.join(" ")))?;
        if !status.success() {
            bail!("zellij {} exited with {}", args.join(" "), status);
        }
        Ok(())
    }

    fn zellij_action(&self, args: &[&str]) -> Result<()> {
        let mut full = vec!["action"];
        full.extend_from_slice(args);
        self.zellij(&full)
    }

    fn plugin_path(&self) -> Option<Utf8PathBuf> {
        // 1. Project-local plugin (built from source)
        let local = self.project_root
            .join("plugin/target/wasm32-wasip1/release/dellij_status.wasm");
        if local.exists() { return Some(local); }
        // 2. ~/.config/zellij/plugins/dellij_status.wasm
        if let Some(home) = dirs_next() {
            let installed = Utf8PathBuf::from(home).join(".config/zellij/plugins/dellij_status.wasm");
            if installed.exists() { return Some(installed); }
        }
        None
    }

    fn pipe_to_plugin(&self, cmd: &PipeCommand) -> Result<bool> {
        if let Some(plugin_path) = self.plugin_path() {
            let payload = cmd.to_json();
            self.zellij(&[
                "pipe",
                "--name", "dellij",
                "--plugin", &format!("file:{}", plugin_path),
                "--",
                &payload,
            ])?;
            return Ok(true);
        }
        Ok(false)
    }

    fn ensure_zellij_available(&self) -> Result<()> {
        if command_exists("zellij") { return Ok(()); }
        bail!("zellij is required but was not found on PATH");
    }

    fn resolve_bookmark_or_command(&self, target: &str) -> Result<String> {
        if let Some(bm) = self.config.bookmarks.iter().find(|b| b.name == target) {
            return Ok(bm.command.clone());
        }
        Ok(target.to_string())
    }

    fn run_hook(&self, hook_name: &str, ws: &Workspace) -> Result<()> {
        let hook_path = self.state_dir.join("hooks").join(hook_name);
        if !hook_path.exists() { return Ok(()); }
        let status = Command::new(&hook_path)
            .current_dir(&ws.worktree_path)
            .envs(env_map(ws, &self.state_dir, &self.project_root))
            .status()
            .with_context(|| format!("running hook {}", hook_path))?;
        if !status.success() {
            bail!("hook {} exited with {}", hook_path, status);
        }
        Ok(())
    }

    fn save(&self) -> Result<()> {
        write_json(&self.config_path, &self.config)
    }

    // ── GitHub helpers ────────────────────────────────────────────────────────

    fn gh_pr_branch(&self, pr: u32) -> Result<String> {
        let out = Command::new("gh")
            .args(["pr", "view", &pr.to_string(), "--json", "headRefName", "-q", ".headRefName"])
            .current_dir(&self.project_root)
            .output()
            .context("running gh pr view")?;
        if !out.status.success() {
            bail!("gh pr view {} failed: {}", pr, String::from_utf8_lossy(&out.stderr));
        }
        Ok(String::from_utf8(out.stdout)?.trim().to_string())
    }

    fn gh_pr_url(&self, pr: u32) -> Result<String> {
        let out = Command::new("gh")
            .args(["pr", "view", &pr.to_string(), "--json", "url", "-q", ".url"])
            .current_dir(&self.project_root)
            .output()
            .context("running gh pr view")?;
        if !out.status.success() {
            bail!("gh pr view {} failed", pr);
        }
        Ok(String::from_utf8(out.stdout)?.trim().to_string())
    }
}

// ── free functions ────────────────────────────────────────────────────────────

fn env_map(ws: &Workspace, state_dir: &Utf8Path, project_root: &Utf8Path) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("DELLIJ_ROOT".to_string(), project_root.to_string()),
        ("DELLIJ_STATE_DIR".to_string(), state_dir.to_string()),
        ("DELLIJ_SLUG".to_string(), ws.slug.clone()),
        ("DELLIJ_AGENT".to_string(), ws.agent.clone()),
        ("DELLIJ_BRANCH".to_string(), ws.branch_name.clone()),
        ("DELLIJ_WORKTREE_PATH".to_string(), ws.worktree_path.to_string()),
        ("DELLIJ_PROMPT".to_string(), ws.prompt.clone()),
    ])
}

fn open_url(url: &str) -> Result<()> {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "linux") {
        "xdg-open"
    } else {
        "start"
    };
    Command::new(opener).arg(url).spawn().with_context(|| format!("opening {url}"))?;
    Ok(())
}

fn which_binary(name: &str) -> Option<std::path::PathBuf> {
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths).find_map(|dir| {
            let candidate = dir.join(name);
            if candidate.is_file() { Some(candidate) } else { None }
        })
    })
}

fn dirs_next() -> Option<String> {
    env::var("HOME").ok()
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;
    use dellij_core::{shell_escape, slugify, render_agent_command, WorkspaceStatus};

    fn make_workspace(slug: &str) -> Workspace {
        Workspace {
            slug: slug.to_string(),
            prompt: "fix the login bug".to_string(),
            agent: "codex".to_string(),
            branch_name: format!("dellij/{slug}"),
            base_branch: "main".to_string(),
            worktree_path: Utf8PathBuf::from(format!("/tmp/ws/{slug}")),
            status: WorkspaceStatus::Working,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            ports: vec![3000],
            urls: vec![],
            last_command: Some(format!("codex \"fix the login bug\"")),
            notes: vec![],
            pr_number: None,
            pr_url: None,
            layout: None,
        }
    }

    mod slugify_unit {
        use super::*;
        #[test]
        fn basic() { assert_eq!(slugify("fix the bug", "codex"), "fix-the-bug-codex"); }
        #[test]
        fn fallback() { assert_eq!(slugify("!!!", "ag"), "workspace-ag"); }
        #[test]
        fn truncates() {
            assert_eq!(
                slugify("one two three four five six seven", "ag"),
                "one-two-three-four-five-six-ag"
            );
        }
        #[test]
        fn uppercase() { assert_eq!(slugify("FIX BUG", "codex"), "fix-bug-codex"); }
    }

    mod shell_escape_unit {
        use super::*;
        #[test]
        fn passthrough() { assert_eq!(shell_escape("hello"), "hello"); }
        #[test]
        fn quotes() { assert_eq!(shell_escape(r#"say "hi""#), r#"say \"hi\""#); }
        #[test]
        fn backslash() { assert_eq!(shell_escape(r"a\b"), r"a\\b"); }
    }

    mod render_unit {
        use super::*;
        #[test]
        fn codex() { assert!(render_agent_command("codex", "fix").starts_with("codex")); }
        #[test]
        fn aider() { assert!(render_agent_command("aider", "fix").contains("--message")); }
    }

    mod serde {
        use super::*;
        #[test]
        fn status_roundtrip() {
            for s in [WorkspaceStatus::Working, WorkspaceStatus::Blocked,
                      WorkspaceStatus::Error, WorkspaceStatus::Done] {
                let j = serde_json::to_string(&s).unwrap();
                let r: WorkspaceStatus = serde_json::from_str(&j).unwrap();
                assert_eq!(s.to_string(), r.to_string());
            }
        }
        #[test]
        fn workspace_roundtrip() {
            let ws = make_workspace("my-slug");
            let j = serde_json::to_string(&ws).unwrap();
            let r: Workspace = serde_json::from_str(&j).unwrap();
            assert_eq!(ws.slug, r.slug);
            assert_eq!(ws.ports, r.ports);
        }
        #[test]
        fn needs_attention() {
            assert!(WorkspaceStatus::Blocked.needs_attention());
            assert!(WorkspaceStatus::Error.needs_attention());
            assert!(!WorkspaceStatus::Working.needs_attention());
            assert!(!WorkspaceStatus::Done.needs_attention());
        }
    }

    mod layout_unit {
        use super::*;
        use dellij_core::LayoutRenderer;
        #[test]
        fn default_contains_env_block() {
            let ws = make_workspace("test");
            let kdl = LayoutRenderer::render(&ws, "/tmp/project", None);
            assert!(kdl.contains("DELLIJ_SLUG"));
            assert!(kdl.contains("DELLIJ_AGENT"));
            assert!(kdl.contains("DELLIJ_ROOT \"/tmp/project\""));
        }
        #[test]
        fn minimal_has_two_panes() {
            let mut ws = make_workspace("test");
            ws.layout = Some("minimal".to_string());
            let kdl = LayoutRenderer::render(&ws, "/tmp/project", None);
            assert!(kdl.contains("split_direction=\"vertical\""));
        }
    }

    mod properties {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn slugify_ends_with_agent(input in ".*", agent in "[a-z]+") {
                let result = slugify(&input, &agent);
                let suffix = format!("-{agent}");
                prop_assert!(result.ends_with(&suffix));
            }
            #[test]
            fn slugify_charset(input in ".*", agent in "[a-z0-9]+") {
                let result = slugify(&input, &agent);
                prop_assert!(result.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'));
            }
            #[test]
            fn slugify_max_segments(input in ".*", agent in "[a-z]+") {
                let result = slugify(&input, &agent);
                let base = result.strip_suffix(&format!("-{agent}")).unwrap_or(&result);
                let count = base.split('-').filter(|s| !s.is_empty()).count();
                prop_assert!(count <= 6);
            }
            #[test]
            fn shell_escape_valid_sequences(input in ".*") {
                let escaped = shell_escape(&input);
                let chars: Vec<char> = escaped.chars().collect();
                let mut i = 0;
                while i < chars.len() {
                    if chars[i] == '\\' {
                        prop_assert!(i + 1 < chars.len() && (chars[i+1] == '\\' || chars[i+1] == '"'));
                        i += 2;
                    } else {
                        prop_assert!(chars[i] != '"');
                        i += 1;
                    }
                }
            }
            #[test]
            fn render_contains_prompt(agent in "[a-z]+", prompt in "[^\"\\\\]*") {
                let cmd = render_agent_command(&agent, &prompt);
                prop_assert!(cmd.contains(&prompt));
            }
        }
    }
}
