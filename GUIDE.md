# dellij — complete guide

> Terminal-native workspace management for parallel AI coding agents.

---

## Table of contents

1. [Concepts](#concepts)
2. [Install](#install)
3. [Cloud Sync (Convex)](#cloud-sync-convex)
4. [First run](#first-run)
5. [Creating workspaces](#creating-workspaces)
6. [Opening workspaces](#opening-workspaces)
7. [Workspace status](#workspace-status)
8. [Layouts](#layouts)
9. [Bookmarks and run](#bookmarks-and-run)
10. [Sending text to panes](#sending-text-to-panes)
11. [GitHub PR integration](#github-pr-integration)
12. [Hooks](#hooks)
13. [Agent hook snippets](#agent-hook-snippets)
14. [Environment variables](#environment-variables)
15. [Importing existing worktrees](#importing-existing-worktrees)
16. [The Zellij plugin](#the-zellij-plugin)
17. [The desktop GUI](#the-desktop-gui)
18. [The mobile app](#the-mobile-app)
19. [IDE deep-linking](#ide-deep-linking)
20. [Diagnostics](#diagnostics)
21. [Configuration reference](#configuration-reference)

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

**GUI / Mobile** — optional GPUI apps for desktop and Android. They watch workspace state
in real-time, even across devices when synced via Convex.

---

## Install & Development

Use `mise` to manage the project or install components manually:

```bash
# Setup
mise run setup          # fetch dependencies

# Components
mise run build          # build desktop workspace (CLI + GUI)
mise run build:plugin   # build WASM plugin
mise run build:mobile   # build Android APK

# Quality
mise run test           # run all unit and property tests
mise run lint           # check formatting and clippy
mise run sync           # deploy Convex functions

# Running
mise run dev            # run CLI from source
mise run release        # optimised release build
```

---

## Cloud Sync (Convex)

Dellij uses [Convex](https://convex.dev) to sync workspace status and alerts in real-time
between your desktop and mobile phone.

### Setup

1. **Deploy Backend**: Run `npx convex deploy` from the project root to push the schema in `convex/`.
2. **Environment**: Add your Convex URL and Auth Token to `.env`:
   ```bash
   CONVEX_URL=https://your-deployment.convex.cloud
   CONVEX_AUTH_TOKEN=your-token
   ```
3. **Automatic Push**: Every time you run `dellij new` or `dellij status`, the CLI
   automatically pushes the update to your Convex cloud.
4. **Mobile Subscription**: The mobile app subscribes to these changes and notifies
   you immediately of any status updates (like an agent finishing its task).

---

## The mobile app

The Android app (`dellij-mobile`) is built with `gpui-mobile` and provides a compact
view of all your active coding agents.

**Home Screen** — List of all workspaces synced via Convex. Real-time status dots
(blue for working, yellow for waiting, red for alerts).

**GitHub Auth** — "Link GitHub" button uses GitHub OAuth to handle repository
permissions for mobile Git operations.

**Alerts & Progress** — Monitor agents while away from your desk. If an agent hits
an error or requests a review, you'll see it instantly on your phone.

---

## Architecture

```
dellij workspace
├── crates/
│   ├── dellij-core/    # Core logic (git2-rs, Convex client, shared types)
│   ├── dellij/         # Async CLI (tokio)
│   ├── dellij-gui/     # GPUI desktop app
│   └── dellij-mobile/  # GPUI mobile app (Android)
├── convex/             # Backend schema & functions (TypeScript)
└── plugin/             # Zellij WASM plugin (status ribbon)
```

The CLI and Desktop GUI use local file-watching for millisecond-latency updates,
while the Mobile app and CLI use Convex for cross-device synchronization.

---

## Diagnostics

```bash
dellij doctor
```

Prints diagnostic info including project root, tool availability, and copy-paste
snippets for AI agent hooks.
