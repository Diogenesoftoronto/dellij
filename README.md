<div align="center">

```
██████╗ ███████╗██╗     ██╗     ██╗     ██╗
██╔══██╗██╔════╝██║     ██║     ██║     ██║
██║  ██║█████╗  ██║     ██║     ██║     ██║
██║  ██║██╔══╝  ██║     ██║     ██║██   ██║
██████╔╝███████╗███████╗███████╗██║╚█████╔╝
╚═════╝ ╚══════╝╚══════╝╚══════╝╚═╝ ╚════╝
```

**Parallel AI coding agents. Git-isolated. Terminal-native. Mobile-synced. Rust.**

[![Rust](https://img.shields.io/badge/rust-1.82+-orange?logo=rust)](https://rustup.rs)
[![Zellij](https://img.shields.io/badge/zellij-0.43+-purple)](https://zellij.dev)
[![Android](https://img.shields.io/badge/android-14+-green?logo=android)](https://developer.android.com)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](./LICENSE)

</div>

---

You're running ten agents in parallel. Each one owns a branch, a terminal tab, and a port. You
need to see at a glance which ones are blocked, which ones are done, and which ones need a nudge.
You need to spin up a new agent in two keystrokes, not twenty. And you need to check on them
from your phone while you're away from your desk.

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
and status. The Zellij plugin tracks them all in a status ribbon. The desktop GUI and **Mobile App**
give you a live view of diffs, notifications, and agent progress — all synced in real-time
via Convex.

```bash
# Spin up a new agent in your repo
dellij new "fix the JWT auth vulnerability" --agent claude-code --open

# Check on everything from your terminal
dellij list

# Pop open the Desktop GUI
dellij ui

# ...or check your phone for the "Review Needed" alert
```

## Features

| | CLI | Plugin | Desktop GUI | Mobile App |
|---|---|---|---|---|
| Git worktree per workspace | ✓ | | | |
| Tab dedup (focus-if-open) | ✓ | ✓ pipe | | |
| `DELLIJ_*` env in every pane | ✓ | | | |
| 4 built-in layouts | ✓ | | | |
| Custom KDL layout templates | ✓ | | | |
| Ahead/behind counts | ✓ | | ✓ | ✓ |
| GitHub PR linking | ✓ | | ✓ | ✓ |
| Import existing worktrees | ✓ | | | |
| Attention indicators | | ✓ | ✓ | ✓ |
| Send text to workspace pane | ✓ | ✓ | | |
| Status ribbon | | ✓ | | |
| Diff viewer | | | ✓ | ✓ |
| Notification panel | | | ✓ | ✓ |
| Browser with JS API | | | ✓ | |
| IDE deep-linking | ✓ | | ✓ | |
| Real-time Cloud Sync (Convex) | ✓ | | ✓ | ✓ |
| GitHub OAuth | | | | ✓ |

## Install

**Requirements:** `git`, `zellij ≥ 0.43`, Rust 1.82+

```bash
# 1. CLI (required)
cargo install --path crates/dellij

# 2. Zellij WASM plugin (required for tab dedup + attention indicators)
cargo build -p dellij-status --release --target wasm32-wasip1
cp plugin/target/wasm32-wasip1/release/dellij_status.wasm ~/.config/zellij/plugins/

# 3. Desktop GUI (optional, Linux/macOS)
cargo install --path crates/dellij-gui

# 4. Mobile App (Android)
./build-mobile.sh
adb install target/release/apk/dellij-mobile.apk
```

## Cloud Sync (Optional)

Dellij supports real-time synchronization between your desktop and mobile devices using [Convex](https://convex.dev).

1. Create a Convex project and deploy the functions in the `convex/` directory.
2. Set your `CONVEX_URL` and `CONVEX_AUTH_TOKEN` in `.env`.
3. The CLI and Mobile app will now stay in sync automatically.

## Architecture

```
dellij workspace
├── crates/
│   ├── dellij-core/    # Shared types, git2-rs logic, Convex sync client
│   ├── dellij/         # CLI binary (async tokio)
│   ├── dellij-gui/     # GPUI desktop app
│   └── dellij-mobile/  # GPUI mobile app (Android)
├── convex/             # Cloud sync backend (schema & functions)
└── plugin/             # Zellij WASM plugin (status ribbon)
```

## License

MIT — see [LICENSE](./LICENSE)
