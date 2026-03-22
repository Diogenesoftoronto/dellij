import { spawn } from 'child_process';
import { existsSync, accessSync, constants } from 'fs';
import { join } from 'path';
import os from 'os';
import type { HookContext } from '../types.ts';

export type HookName =
  | 'before_pane_create'
  | 'pane_created'
  | 'worktree_created'
  | 'before_pane_close'
  | 'pane_closed'
  | 'before_worktree_remove'
  | 'worktree_removed'
  | 'pre_merge'
  | 'post_merge'
  | 'run_test'
  | 'run_dev';

export class HookManager {
  private static instance: HookManager;
  private projectRoot: string;
  private dellijDir: string;

  private constructor(projectRoot: string, dellijDir: string) {
    this.projectRoot = projectRoot;
    this.dellijDir = dellijDir;
  }

  static getInstance(projectRoot: string, dellijDir: string): HookManager {
    if (!HookManager.instance) {
      HookManager.instance = new HookManager(projectRoot, dellijDir);
    }
    return HookManager.instance;
  }

  /**
   * Find a hook script with priority resolution:
   * 1. .dellij-hooks/ (team hooks)
   * 2. .dellij/hooks/ (local override)
   * 3. ~/.dellij/hooks/ (global hooks)
   */
  findHook(hookName: HookName): string | null {
    const searchPaths = [
      join(this.projectRoot, '.dellij-hooks', hookName),
      join(this.dellijDir, 'hooks', hookName),
      join(os.homedir(), '.dellij', 'hooks', hookName),
    ];

    for (const hookPath of searchPaths) {
      if (existsSync(hookPath)) {
        try {
          accessSync(hookPath, constants.X_OK);
          return hookPath;
        } catch {
          // Exists but not executable
        }
      }
    }

    return null;
  }

  /**
   * Run a hook script non-blocking (fire and forget).
   * Sets relevant env vars from context before executing.
   */
  runHook(hookName: HookName, context: HookContext): void {
    const hookPath = this.findHook(hookName);
    if (!hookPath) return;

    const env: NodeJS.ProcessEnv = {
      ...process.env,
      ...context,
    };

    const child = spawn('bash', [hookPath], {
      env,
      detached: true,
      stdio: 'ignore',
    });

    child.unref();
  }


  /**
   * Returns example hook content for user reference.
   */
  getExampleHooks(): Record<HookName, string> {
    return {
      before_pane_create: `#!/usr/bin/env bash\necho "Before pane create"`,
      pane_created: `#!/usr/bin/env bash\necho "Pane created"`,
      worktree_created: `#!/usr/bin/env bash
# Called after a new worktree is created
# Env vars: DELLIJ_ROOT, DELLIJ_SLUG, DELLIJ_AGENT, DELLIJ_WORKTREE_PATH, DELLIJ_BRANCH, DELLIJ_PROMPT

echo "Worktree created: $DELLIJ_BRANCH at $DELLIJ_WORKTREE_PATH"

# Example: install dependencies
# cd "$DELLIJ_WORKTREE_PATH" && npm install
`,
      before_pane_close: `#!/usr/bin/env bash
# Called before a tab is closed
# Env vars: DELLIJ_ROOT, DELLIJ_SLUG, DELLIJ_AGENT, DELLIJ_WORKTREE_PATH, DELLIJ_BRANCH

echo "Closing tab: $DELLIJ_SLUG"
`,
      pane_closed: `#!/usr/bin/env bash
# Called after a tab has been closed and worktree removed
# Env vars: DELLIJ_ROOT, DELLIJ_SLUG, DELLIJ_BRANCH

echo "Tab closed: $DELLIJ_SLUG"
`,
      before_worktree_remove: `#!/usr/bin/env bash\necho "Before worktree remove"`,
      worktree_removed: `#!/usr/bin/env bash\necho "Worktree removed"`,
      pre_merge: `#!/usr/bin/env bash
# Called before merging a worktree branch
# Env vars: DELLIJ_ROOT, DELLIJ_SLUG, DELLIJ_WORKTREE_PATH, DELLIJ_BRANCH

echo "Pre-merge: $DELLIJ_BRANCH"

# Example: run tests before merge
# cd "$DELLIJ_WORKTREE_PATH" && npm test
`,
      post_merge: `#!/usr/bin/env bash
# Called after a successful merge
# Env vars: DELLIJ_ROOT, DELLIJ_SLUG, DELLIJ_BRANCH

echo "Post-merge: $DELLIJ_BRANCH"
`,
      run_test: `#!/usr/bin/env bash\necho "Run tests"`,
      run_dev: `#!/usr/bin/env bash\necho "Run dev server"`,
    };
  }
}

