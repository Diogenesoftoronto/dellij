# Dellij Guide

This guide covers the normal way to use `dellij` in a real project.

## What Dellij Does

`dellij` creates a Zellij session tied to your project, keeps a small project config in `.dellij/`, and can open agent worktrees in dedicated tabs.

Outside Zellij:

- `dellij` creates or attaches to the project session.

Inside Zellij:

- `dellij` renders the control UI in the current pane.
- `dellij new "prompt"` creates a worktree and opens a new agent tab.

## Prerequisites

You need:

- `zellij`
- `bun`
- `git`
- at least one supported agent CLI if you want agent tabs

Supported agent IDs include:

- `claude`
- `opencode`
- `codex`
- `cline`
- `gemini`
- `qwen`
- `amp`
- `pi`
- `cursor`
- `copilot`
- `crush`
- `aider`

## Install Dellij

Install the CLI so `dellij` is available on your `PATH`:

```bash
cd /path/to/dellij
bun install
bun link
```

Verify it:

```bash
command -v dellij
```

This guide assumes that command succeeds.

If you are working on `dellij` itself, the repository also contains:

- the `dellij` launcher script
- the Bun source entrypoint
- `mise` tasks for development

## First Run

Run `dellij` from the root of the repository you want to manage:

```bash
dellij doctor
dellij
```

On first run, `dellij` creates:

- `.dellij/dellij.config.json`
- `.dellij/status/`
- `.dellij/hooks/`
- `.dellij/worktrees/`

It also creates or attaches to a Zellij session named from your project path.

## Basic Workflow

### 1. Open the control session

From the project you want to work on:

```bash
dellij
```

### 2. Create an agent tab

Inside the project session:

```bash
dellij new "add pagination to the admin API"
```

You can pick a specific agent:

```bash
dellij new "fix the flaky auth test" --agent codex
```

What happens:

- a git worktree is created
- a branch is created using the generated slug
- the tab is added to `.dellij/dellij.config.json`
- a Zellij tab opens for that agent

### 3. List tracked tabs

```bash
dellij list
```

This prints the tabs currently tracked in the config.

### Health check

```bash
dellij doctor
```

This checks:

- whether `dellij`, `bun`, `git`, and `zellij` are on `PATH`
- whether you are in a git repo and at the project root
- whether `.dellij` state exists yet
- whether the local status plugin is usable
- whether enabled agent CLIs are available

### 4. Merge a finished worktree

```bash
dellij merge my-agent-slug
```

If the merge succeeds, the tab status is updated to `done`.

### 5. Remove a tab from the config

```bash
dellij close my-agent-slug
```

This removes the tab from the config. It does not automatically merge code for you.

## Where to Run Commands

Use this rule:

- run plain `dellij` from a normal shell when you want to create or attach to the project session
- run `dellij new`, `dellij list`, `dellij merge`, and `dellij close` from inside the project session when managing tabs

If you launch `mise run dev` a second time, it now reattaches with mirrored-session settings and suppresses Zellij release-note popups, which avoids the odd split-client behavior that was previously happening.

## Config

The project config lives at:

```text
.dellij/dellij.config.json
```

Important settings:

- `defaultAgent`
- `enabledAgents`
- `permissionMode`
- `baseBranch`
- `branchPrefix`

Default values are created automatically. Environment variables can override them:

- `DELLIJ_DEFAULT_AGENT`
- `DELLIJ_ENABLED_AGENTS`
- `DELLIJ_BASE_BRANCH`
- `DELLIJ_BRANCH_PREFIX`
- `DELLIJ_PERMISSION_MODE`

Example:

```bash
export DELLIJ_DEFAULT_AGENT=codex
export DELLIJ_ENABLED_AGENTS=codex,claude,opencode
export DELLIJ_PERMISSION_MODE=acceptEdits
```

## Hooks

`dellij` looks for executable hooks in this order:

1. `.dellij-hooks/`
2. `.dellij/hooks/`
3. `~/.dellij/hooks/`

Available hook names:

- `before_pane_create`
- `pane_created`
- `worktree_created`
- `before_pane_close`
- `pane_closed`
- `pre_merge`
- `post_merge`
- `run_test`
- `run_dev`

Simple example:

```bash
mkdir -p .dellij/hooks
cat > .dellij/hooks/worktree_created <<'EOF'
#!/usr/bin/env bash
echo "Created worktree: $DELLIJ_WORKTREE_PATH"
EOF
chmod +x .dellij/hooks/worktree_created
```

## Development Commands

Inside the `dellij` repo:

- `mise run dev` starts the app
- `mise run ui` runs the TUI pane directly
- `mise run build:plugin` rebuilds the plugin
- `mise run typecheck` runs TypeScript checks
- `mise run release` runs the release checks

## Troubleshooting

### The second `mise run dev` behaves strangely

The session attach flow now uses mirrored-session options and disables Zellij release notes and startup tips on attach. If you still hit an old broken session, close it and start again:

```bash
zellij kill-session <session-name>
mise run dev
```

### The status plugin does not appear

`dellij` now skips plugin builds that Zellij cannot load. On this machine, the local log showed the plugin wasm was missing the `_start` export that the packaged Zellij runtime expects.

That means:

- the main workflow still works
- the status ribbon may be omitted
- you can still manage sessions, worktrees, and agent tabs normally

### I ran `dellij new ...` outside Zellij

That still creates the worktree and updates the config, but it cannot open a new agent tab until you attach to the session.

### I want to inspect the project state

Check:

- `.dellij/dellij.config.json`
- `.dellij/worktrees/`
- `.dellij/status/`

## Recommended Daily Flow

```bash
cd /your/project
dellij
dellij new "first task" --agent codex
dellij new "second task" --agent claude
dellij list
dellij merge first-task-codex
dellij close first-task-codex
```

That is the core `dellij` loop.
