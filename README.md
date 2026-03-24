<div align="center">

```
██████╗ ███████╗██╗     ██╗     ██╗     ██╗
██╔══██╗██╔════╝██║     ██║     ██║     ██║
██║  ██║█████╗  ██║     ██║     ██║     ██║
██║  ██║██╔══╝  ██║     ██║     ██║██   ██║
██████╔╝███████╗███████╗███████╗██║╚█████╔╝
╚═════╝ ╚══════╝╚══════╝╚══════╝╚═╝ ╚════╝
```

**Parallel AI coding agents. Git-isolated. Terminal-native. Rust.**

[![Rust](https://img.shields.io/badge/rust-1.82+-orange?logo=rust)](https://rustup.rs)
[![Zellij](https://img.shields.io/badge/zellij-0.43+-purple)](https://zellij.dev)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](./LICENSE)

</div>

---

You're running ten agents in parallel. Each one owns a branch, a terminal tab, and a port. You
need to see at a glance which ones are blocked, which ones are done, and which ones need a nudge.
You need to spin up a new agent in two keystrokes, not twenty.

**dellij** is that tool.

```
┌─────────────────────────────────────────────────────────────────┐
│ ◆ dellij  myapp          ⬤ ⬤ ◯ ✓ ⬤              ⚠ 2 alerts   │
├──────────────┬──────────────────────────────────────────────────┤
│ WORKSPACES 5 │  ~ Diff   ⚠ Alerts   ⬡ Browser                  │
│              ├──────────────────────────────────────────────────┤
│▌fix-auth  cc │  fix-auth-cc · main → fix-auth          +42 -18  │
│  fix-auth ↑2 │──────────────────────────────────────────────────│
│  :3000 PR#42 │  12   │  12  │   function authenticate(          │
│  [VS Code]   │  13   │      │ - return jwt.verify(token)        │
│              │       │  13  │ + return jwt.verify(token, {       │
│  add-feat cc │       │  14  │ +   algorithms: ['RS256']          │
│  feat/x  ↑5  │  14   │  15  │   }                               │
│  ⊘ blocked   │                                                   │
│              │                                                   │
│  refactor cx │                                                   │
│  main ↑1     │                                                   │
│              │                                                   │
│ ⚠ 2 alerts   │                                                   │
└──────────────┴───────────────────────────────────────────────────┘
```

## What it does

Each **workspace** is a git worktree on its own branch with its own terminal tab, env vars, ports,
and status. The Zellij plugin tracks them all in a status ribbon. The desktop GUI gives you a
diff viewer, notification panel, and browser — without leaving the keyboard.

```bash
# Spin up a new agent in your repo
dellij new "fix the JWT auth vulnerability" --agent claude-code --open

# The tab opens automatically. Inside it:
# - pane 1: claude "fix the JWT auth vulnerability"   (runs immediately)
# - pane 2: bash shell                                (with DELLIJ_* env)
# - pane 3: git status --short --branch

# Meanwhile, check on everything
dellij list
# !  add-feat-cc          blocked    claude-code  ↑5      :4000
#    fix-auth-cc          working    claude-code  ↑2 ↓0   :3000   PR#42
#    refactor-cx          done       codex        ↑1

# Jump to the blocked one
dellij open add-feat-cc          # focuses existing tab, no duplicate

# When your agent flags it's waiting
dellij status fix-auth-cc review --note "ready for diff review"

# Pop open the GUI
dellij ui
```

## Features

| | CLI | Plugin | GUI |
|---|---|---|---|
| Git worktree per workspace | ✓ | | |
| Tab dedup (focus-if-open) | ✓ | ✓ pipe | |
| `DELLIJ_*` env in every pane | ✓ | | |
| 4 built-in layouts | ✓ | | |
| Custom KDL layout templates | ✓ | | |
| Ahead/behind counts | ✓ | | ✓ |
| GitHub PR linking | ✓ | | ✓ |
| Import existing worktrees | ✓ | | |
| Attention indicators on tabs | | ✓ | ✓ |
| Send text to workspace pane | ✓ | ✓ | |
| Status ribbon | | ✓ | |
| Diff viewer | | | ✓ |
| Notification panel | | | ✓ |
| Browser with JS API | | | ✓ |
| IDE deep-linking | ✓ | | ✓ |
| Claude Code / Codex hooks | ✓ doctor | | |
| Bookmarks / playbooks | ✓ | | |
| Lifecycle hooks | ✓ | | |

## Install

**Requirements:** `git`, `zellij ≥ 0.43`, Rust 1.82+

```bash
# CLI (required)
cargo install --path crates/dellij

# Zellij WASM plugin (required for tab dedup + attention indicators)
cargo build -p dellij-status --release --target wasm32-wasip1
cp plugin/target/wasm32-wasip1/release/dellij_status.wasm \
   ~/.config/zellij/plugins/

# Desktop GUI (optional, Linux/macOS)
# System deps on Fedora/RHEL: dnf install vulkan-loader-devel libxkbcommon-devel
cargo install --path crates/dellij-gui
```

Add the plugin to your Zellij config (`~/.config/zellij/config.kdl`):

```kdl
plugins {
  dellij-status location="file:~/.config/zellij/plugins/dellij_status.wasm" {
    config_dir "/path/to/your/project/.dellij"
  }
}

load_plugins {
  dellij-status
}
```

## Quickstart

```bash
cd ~/your-rust-project
dellij init                         # creates .dellij/
dellij open                         # start the session

# Create workspaces
dellij new "add OAuth2 login" --agent claude-code --port 3000 --open
dellij new "fix memory leak in parser" --agent codex --open
dellij new "refactor auth middleware" --use-branch feat/auth-refactor --open

# From a GitHub PR
dellij new "review payment integration" --base-pr 247 --open

# Import an existing worktree you already have
dellij import ../my-other-branch --agent aider

# Check everything
dellij list

# Open the GUI (detached)
dellij ui
```

## Layouts

Four built-in layouts, or define your own:

```bash
dellij new "my task" --layout minimal       # agent + shell (2 panes)
dellij new "my task" --layout default       # agent + shell + git status (3 panes)
dellij new "my task" --layout full          # agent + shell + diff + ports (4 panes)
dellij new "my task" --layout agent-only    # single pane, just the agent
```

Custom layouts in `.dellij/dellij.json`:

```json
{
  "settings": {
    "layouts": {
      "my-layout": "layout {\n  pane command=\"bash\" cwd=\"{cwd}\" {\n    args \"-lc\" \"{agent_cmd}\"\n  }\n}\n"
    }
  }
}
```

Placeholders: `{cwd}`, `{agent_cmd}`, `{slug}`, `{branch}`, `{base_branch}`, `{prompt}`

## Agent hooks

Wire up your agent to automatically update dellij status. Run `dellij doctor` to get copy-paste snippets, or see below:

**Claude Code** (`~/.claude/settings.json`):

```json
{
  "hooks": {
    "Notification": [{"matcher": "", "hooks": [{"type": "command",
      "command": "dellij status $DELLIJ_SLUG waiting 2>/dev/null || true"}]}],
    "Stop": [{"matcher": "", "hooks": [{"type": "command",
      "command": "dellij status $DELLIJ_SLUG done 2>/dev/null || true"}]}]
  }
}
```

**OpenAI Codex** (`~/.codex/config.toml`):

```toml
notify = ["bash", "-c", "dellij status ${DELLIJ_SLUG} waiting 2>/dev/null || true"]
```

## Environment

Every pane in every workspace has these set automatically:

| Variable | Value |
|---|---|
| `DELLIJ_SLUG` | `fix-jwt-auth-claude-code` |
| `DELLIJ_AGENT` | `claude-code` |
| `DELLIJ_BRANCH` | `dellij/fix-jwt-auth-claude-code` |
| `DELLIJ_BASE_BRANCH` | `main` |
| `DELLIJ_WORKTREE_PATH` | `/home/you/project/.dellij/workspaces/fix-jwt-auth-claude-code` |
| `DELLIJ_ROOT` | `/home/you/project` |
| `DELLIJ_PROMPT` | `fix the JWT auth vulnerability` |

## Architecture

```
dellij workspace
├── crates/
│   ├── dellij-core/    # Shared types: Workspace, Config, LayoutRenderer, PipeCommand
│   ├── dellij/         # CLI binary
│   └── dellij-gui/     # GPUI desktop app (diff viewer, sidebar, browser, notifications)
└── plugin/             # Zellij WASM plugin (status ribbon + pipe controller)
```

The CLI writes JSON state to `.dellij/`. The plugin polls `.dellij/status/` and also receives
`zellij pipe` commands for tab focus/dedup and pane-targeted sends. The GUI watches the same
directory tree for live updates.

## Development

```bash
mise run setup       # install targets and toolchain
mise run build       # build CLI + core
mise run build:plugin  # build WASM plugin
mise run dev         # cargo check --watch
mise run release     # optimised release build
```

```bash
cargo test -p dellij-core -p dellij  # 19 tests (unit + property)
```

## Prior art & credit

- **[Superset](https://superset.sh)** — git worktree isolation, parallel agent UX
- **[cmux](https://cmux.com)** — terminal-first orchestration, pipe API patterns, browser integration
- **[Zellij](https://zellij.dev)** — the multiplexer this wraps
- **[awesome-zellij](https://github.com/zellij-org/awesome-zellij)** — plugin ecosystem inspiration
- **[GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui)** — Zed's GPU-accelerated UI framework

## License

MIT — see [LICENSE](./LICENSE)
