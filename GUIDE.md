# dellij — complete guide

> Terminal-native workspace management for parallel AI coding agents.

---

## Table of contents

1. [Concepts](#concepts)
2. [Install](#install)
3. [First run](#first-run)
4. [Creating workspaces](#creating-workspaces)
5. [Opening workspaces](#opening-workspaces)
6. [Workspace status](#workspace-status)
7. [Layouts](#layouts)
8. [Bookmarks and run](#bookmarks-and-run)
9. [Sending text to panes](#sending-text-to-panes)
10. [GitHub PR integration](#github-pr-integration)
11. [Hooks](#hooks)
12. [Agent hook snippets](#agent-hook-snippets)
13. [Environment variables](#environment-variables)
14. [Importing existing worktrees](#importing-existing-worktrees)
15. [The Zellij plugin](#the-zellij-plugin)
16. [The desktop GUI](#the-desktop-gui)
17. [IDE deep-linking](#ide-deep-linking)
18. [Diagnostics](#diagnostics)
19. [Configuration reference](#configuration-reference)

---

## Concepts

**Workspace** — one unit of work. Each workspace has:
- an isolated git worktree on its own branch
- a Zellij tab with panes pre-configured for the agent command, a shell, and git status
- `DELLIJ_*` env vars in every pane
- a status (`working` / `waiting` / `blocked` / `review` / `done` / `error`)
- optional ports, URLs, PR number, notes

**Project** — a git repo you've run `dellij init` inside. State lives in `.dellij/`.

**Plugin** — a Zellij WASM plugin that renders a status ribbon and acts as a pipe controller.
The CLI can send it commands to focus tabs, dedup opens, and write to panes.

**GUI** — an optional GPUI desktop app (`dellij ui`) with diff viewer, notification panel,
sidebar, and browser window.

---

## Install

```bash
# 1. CLI
cargo install --path crates/dellij

# 2. WASM plugin
cargo build -p dellij-status --release --target wasm32-wasip1
cp plugin/target/wasm32-wasip1/release/dellij_status.wasm \
   ~/.config/zellij/plugins/

# 3. GUI (optional)
#    Linux system deps (Fedora): dnf install vulkan-loader-devel libxkbcommon-devel wayland-devel
cargo install --path crates/dellij-gui
```

Add to `~/.config/zellij/config.kdl`:

```kdl
plugins {
  dellij-status location="file:~/.config/zellij/plugins/dellij_status.wasm" {
    config_dir "/absolute/path/to/project/.dellij"
  }
}
load_plugins {
  dellij-status
}
```

---

## First run

```bash
cd ~/your-project
dellij init
```

Creates:

```
.dellij/
├── dellij.json       # config: settings, bookmarks, workspaces
├── hooks/            # lifecycle scripts
├── layouts/          # generated KDL files (one per workspace)
├── status/           # JSON snapshots read by the plugin
└── workspaces/       # git worktrees land here by default
```

```bash
dellij open           # creates or attaches to the project Zellij session
```

---

## Creating workspaces

### Basic

```bash
dellij new "add OAuth2 login flow" --agent claude-code --open
```

- creates `.dellij/workspaces/add-oauth2-login-flow-claude-code`
- creates branch `dellij/add-oauth2-login-flow-claude-code` off `main`
- opens a Zellij tab with 3 panes (agent, shell, git status)
- writes `.dellij/status/add-oauth2-login-flow-claude-code.json`

### With ports and URLs

```bash
dellij new "build admin dashboard" --agent codex \
  --port 3000 --port 5173 \
  --url http://localhost:3000/admin \
  --open
```

Ports and URLs are stored in the workspace and shown in `list` and the GUI sidebar.

### From an existing branch

```bash
dellij new "review the payment refactor" \
  --use-branch feat/payment-refactor \
  --agent claude-code --open
```

Skips branch creation, attaches the worktree to the existing branch.

### From a GitHub PR

```bash
dellij new "review PR 247" --base-pr 247 --agent claude-code --open
```

Uses `gh pr view 247` to get the branch, creates the worktree from it, and stores the PR number.

### With a non-default layout

```bash
dellij new "quick fix" --agent codex --layout minimal --open     # 2 panes
dellij new "big refactor" --agent claude-code --layout full --open  # 4 panes
dellij new "deploy run" --agent codex --layout agent-only --open  # 1 pane
```

### With setup hook

```bash
dellij new "seed the database" --agent codex --setup --open
```

Runs `.dellij/hooks/workspace_setup` after creating the worktree.

---

## Opening workspaces

```bash
dellij open                          # open/attach the project session
dellij open add-oauth2-login-flow-claude-code  # focus existing tab, or create it
```

If the plugin is running, `open` sends a pipe command that focuses the matching tab
**without creating a duplicate** — this is the tab-dedup feature.

If you're not inside Zellij yet, dellij starts the session and creates the tab.

---

## Workspace status

Six states: `working` · `waiting` · `blocked` · `review` · `done` · `error`

```bash
dellij status add-oauth2-login-flow-claude-code review \
  --note "auth flow complete, needs security review"

dellij status add-oauth2-login-flow-claude-code done
dellij status add-oauth2-login-flow-claude-code error \
  --note "test suite failing on CI"
```

`blocked`, `error`, and `review` light up attention indicators in the plugin ribbon and the GUI.

---

## Layouts

| Name | Panes | Best for |
|---|---|---|
| `default` | 3: agent · shell · git status | daily use |
| `minimal` | 2: agent · shell | simple tasks |
| `full` | 4: agent · shell · git diff · ports | complex tasks |
| `agent-only` | 1: agent | background jobs |

### Custom layouts

Add to `.dellij/dellij.json` under `settings.layouts`:

```json
{
  "settings": {
    "layouts": {
      "server": "layout {\n  pane split_direction=\"vertical\" {\n    pane command=\"bash\" cwd=\"{cwd}\" { args \"-lc\" \"{agent_cmd}\" }\n    pane command=\"bash\" cwd=\"{cwd}\" { args \"-lc\" \"cargo run\" }\n  }\n}\n"
    }
  }
}
```

Available placeholders:

| Placeholder | Value |
|---|---|
| `{cwd}` | absolute worktree path |
| `{agent_cmd}` | shell-escaped agent launch command |
| `{slug}` | workspace slug |
| `{branch}` | full branch name |
| `{base_branch}` | base branch |
| `{prompt}` | shell-escaped original prompt |

---

## Bookmarks and run

Bookmarks are reusable commands, resolved by name in `dellij run`.

```bash
# Save
dellij bookmark add test "cargo test -q" --description "quick test pass"
dellij bookmark add lint "cargo clippy -- -D warnings"
dellij bookmark add dev  "cargo run -- --dev"
dellij bookmark list

# Run in a workspace
dellij run add-oauth2-login-flow-claude-code test
dellij run add-oauth2-login-flow-claude-code test --floating      # floating pane
dellij run add-oauth2-login-flow-claude-code test --close-on-exit # pane closes on exit

# Or pass a raw command directly
dellij run add-oauth2-login-flow-claude-code "cargo build --release"
```

If run inside Zellij, the command opens in a new pane in the workspace's working directory.
If run outside Zellij, it runs directly in the shell.

---

## Sending text to panes

With the plugin running, you can write directly to a workspace's shell pane:

```bash
dellij send add-oauth2-login-flow-claude-code "cargo test\n"
dellij send add-oauth2-login-flow-claude-code "git diff HEAD~1\n"
```

This uses `zellij pipe` → plugin → `write_chars()`. The plugin focuses the workspace tab
first, then writes the text.

---

## GitHub PR integration

```bash
# Associate a PR with an existing workspace
dellij pr set add-oauth2-login-flow-claude-code 247

# Open the PR in the browser
dellij pr open add-oauth2-login-flow-claude-code

# Show PR status (runs gh pr view)
dellij pr status add-oauth2-login-flow-claude-code
```

PR numbers appear in `dellij list`, the plugin ribbon (if short), and the GUI sidebar.

---

## Hooks

Executables in `.dellij/hooks/` run at lifecycle events.

| Hook | When |
|---|---|
| `workspace_setup` | `dellij new --setup` or `dellij open --setup` |
| `workspace_teardown` | `dellij close --teardown` |

Example setup hook:

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "Setting up $DELLIJ_SLUG"
cd "$DELLIJ_WORKTREE_PATH"

# Install deps
npm ci

# Copy dev env
cp "$DELLIJ_ROOT/.env.example" .env

# Seed database
npm run db:seed

echo "Ready at $DELLIJ_WORKTREE_PATH"
```

Example teardown hook:

```bash
#!/usr/bin/env bash
set -euo pipefail

# Stop dev server if running on the workspace's port
fuser -k 3000/tcp 2>/dev/null || true

# Clean up containers
docker compose -f "$DELLIJ_WORKTREE_PATH/docker-compose.yml" down 2>/dev/null || true
```

---

## Agent hook snippets

Run `dellij doctor` to print copy-paste snippets. Full versions:

### Claude Code

`~/.claude/settings.json`:

```json
{
  "hooks": {
    "Notification": [{
      "matcher": "",
      "hooks": [{"type": "command",
        "command": "dellij status $DELLIJ_SLUG waiting 2>/dev/null || true"
      }]
    }],
    "Stop": [{
      "matcher": "",
      "hooks": [{"type": "command",
        "command": "dellij status $DELLIJ_SLUG done 2>/dev/null || true"
      }]
    }],
    "SubagentStop": [{
      "matcher": "",
      "hooks": [{"type": "command",
        "command": "dellij status $DELLIJ_SLUG working 2>/dev/null || true"
      }]
    }]
  }
}
```

### OpenAI Codex

`~/.codex/config.toml`:

```toml
notify = ["bash", "-c",
  "dellij status ${DELLIJ_SLUG:-unknown} waiting 2>/dev/null || true"]
```

### OpenCode

`.opencode/plugins/dellij-notify.js`:

```javascript
export const DellijNotifyPlugin = async ({ $ }) => ({
  event: async ({ event }) => {
    const slug = process.env.DELLIJ_SLUG;
    if (!slug) return;
    if (event.type === "session.idle")
      await $`dellij status ${slug} waiting`.catch(() => {});
    if (event.type === "session.done")
      await $`dellij status ${slug} done`.catch(() => {});
  },
});
```

---

## Environment variables

Every pane in every workspace has these automatically:

| Variable | Example value |
|---|---|
| `DELLIJ_SLUG` | `fix-jwt-auth-claude-code` |
| `DELLIJ_AGENT` | `claude-code` |
| `DELLIJ_BRANCH` | `dellij/fix-jwt-auth-claude-code` |
| `DELLIJ_BASE_BRANCH` | `main` |
| `DELLIJ_WORKTREE_PATH` | `/home/you/project/.dellij/workspaces/fix-jwt-auth-claude-code` |
| `DELLIJ_ROOT` | `/home/you/project` |
| `DELLIJ_PROMPT` | `fix the JWT auth vulnerability` |

Hooks also receive all of the above.

---

## Importing existing worktrees

If you already have a worktree you didn't create with dellij:

```bash
dellij import ../my-existing-worktree
dellij import ../my-existing-worktree --name "my custom name" --agent claude-code
dellij import ../my-existing-worktree --prompt "fix the thing" --port 4000
```

dellij detects the branch from the worktree, creates the metadata, writes the layout
and status files, and adds it to `.dellij/dellij.json`.

---

## The Zellij plugin

The WASM plugin does three things:

**1. Status ribbon** — shows all tracked workspaces with coloured status dots.
Tab that needs attention gets a `!` prefix.

```
 dellij cc ⬤ working  cx ◯ waiting  !ai ⊘ blocked
```

Color coding:
- `⬤` yellow = working
- `◯` cyan = waiting
- `⊘` red = blocked
- `✗` red = error
- `◈` blue = review
- `✓` green = done

**2. Tab dedup** — when `dellij open <slug>` is called from inside Zellij, the CLI
sends `{"action":"open","slug":"..."}` over `zellij pipe`. The plugin checks if a tab
named `<slug>` already exists: if yes, it calls `go_to_tab`; if no, it reads
`.dellij/layouts/<slug>.kdl` and calls `new_tabs_with_layout`.

**3. Send to pane** — `dellij send <slug> <text>` pipes `{"action":"send",...}` to the
plugin, which focuses the workspace tab then calls `write_chars`.

Configuration (`config.kdl`):

```kdl
plugins {
  dellij-status location="file:~/.config/zellij/plugins/dellij_status.wasm" {
    config_dir "/absolute/path/to/your/project/.dellij"
  }
}
```

---

## The desktop GUI

```bash
dellij ui
```

Launches `dellij-gui`, a GPUI desktop app. It watches `.dellij/status/` live.

**Sidebar** — all workspaces with status dot, agent pill, branch, ahead/behind,
ports, PR number. Click to select. Buttons to open in VS Code, Cursor, Zed, or browser.

**Diff tab** — two-column line-number diff view of `git diff <base>...<branch>` for the
selected workspace. Stats banner shows file count, insertions, deletions.

**Alerts tab** — notification cards for every workspace with `blocked`, `error`, or
`review` status. Each card shows status, agent, branch, PR, and latest note.

**Browser tab** — click a port or URL on any workspace row to spawn a wry WebView
window with `window.dellij.{snapshot,click,fill,eval}` injected.

---

## IDE deep-linking

```bash
dellij edit <slug>                    # opens in $EDITOR
dellij edit <slug> --editor cursor    # specific editor
dellij edit <slug> --editor code
dellij edit <slug> --editor zed
dellij edit <slug> --editor idea
```

Also available via the GUI sidebar — click **VS Code**, **Cursor**, or **Zed** buttons
on the selected workspace row.

---

## Diagnostics

```bash
dellij doctor
```

Prints:

```
project_root    : /home/you/myproject
session         : dellij-home-you-myproject-session
git             : ok
zellij          : ok
gh (GitHub CLI) : ok
dellij-gui      : ok
hooks_dir       : ok
workspace_count : 4
bookmark_count  : 3

── Claude Code hooks (~/.claude/settings.json) ──────────────
{ ... copy-paste ready ... }

── OpenAI Codex (~/.codex/config.toml) ─────────────────────
...

── OpenCode plugin (.opencode/plugins/dellij-notify.js) ─────
...
```

---

## Configuration reference

`.dellij/dellij.json` (auto-created, hand-editable):

```json
{
  "project_root": "/home/you/myproject",
  "created_at": "2025-03-01T10:00:00Z",
  "settings": {
    "default_agent": "claude-code",
    "base_branch": "main",
    "branch_prefix": "dellij/",
    "workspace_root": ".dellij/workspaces",
    "layouts": {}
  },
  "bookmarks": [
    { "name": "test", "command": "cargo test -q", "description": "quick test" }
  ],
  "workspaces": [...]
}
```

`branch_prefix` — prefix added to every created branch. Set to `""` to use the slug directly.

`workspace_root` — relative path from project root where worktrees are created.
Can be outside the project: `"../dellij-workspaces"` keeps the repo clean.

`default_agent` — used when `--agent` is not passed to `dellij new`.
