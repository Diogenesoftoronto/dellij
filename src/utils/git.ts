import { exec as execCb } from 'child_process';
import { promisify } from 'util';
import { basename, join } from 'path';
import { createHash } from 'crypto';
import { mkdirSync } from 'fs';
import { type AgentName, AGENT_REGISTRY } from './agentLaunch.ts';

const exec = promisify(execCb);

async function run(cmd: string, cwd?: string): Promise<string> {
  const { stdout } = await exec(cmd, { cwd });
  return stdout.trim();
}

/**
 * Returns the git project root (throws if not a git repo).
 */
export async function getProjectRoot(): Promise<string> {
  return run('git rev-parse --show-toplevel');
}

/**
 * Returns the basename of the project root as the project name.
 */
export function getProjectName(root: string): string {
  return basename(root);
}

/**
 * Generates a stable 8-character hex hash from the project root path.
 */
export function generateSessionHash(root: string): string {
  return createHash('md5').update(root).digest('hex').slice(0, 8);
}

/**
 * Detect the repo's default base branch (main > master > develop > HEAD).
 */
export async function getBaseBranch(projectRoot: string): Promise<string> {
  for (const candidate of ['main', 'master', 'develop']) {
    try {
      await run(
        `git show-ref --verify --quiet refs/heads/${candidate}`,
        projectRoot,
      );
      return candidate;
    } catch {
      // not found, try next
    }
  }
  // Fall back to current HEAD branch name
  try {
    return await run('git rev-parse --abbrev-ref HEAD', projectRoot);
  } catch {
    return 'main';
  }
}

/**
 * Generate a URL-safe slug from a prompt + agent suffix.
 */
export function generateSlug(prompt: string, agent: AgentName): string {
  const suffix = AGENT_REGISTRY[agent].slugSuffix;

  const words = prompt
    .toLowerCase()
    .replace(/[^a-z0-9\s]/g, ' ')
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 4);

  const base = words.join('-') || 'task';
  return `${base}-${suffix}`;
}

/**
 * Creates a git worktree at `.dellij/worktrees/{slug}` branching from baseBranch.
 * Returns the full path of the created worktree.
 */
export async function createWorktree(opts: {
  projectRoot: string;
  slug: string;
  baseBranch: string;
}): Promise<string> {
  const { projectRoot, slug, baseBranch } = opts;
  const worktreesDir = join(projectRoot, '.dellij', 'worktrees');
  const worktreePath = join(worktreesDir, slug);

  mkdirSync(worktreesDir, { recursive: true });

  await run(
    `git worktree add "${worktreePath}" -b "${slug}" "${baseBranch}"`,
    projectRoot,
  );

  return worktreePath;
}

/**
 * Removes a git worktree and its associated branch.
 */
export async function removeWorktree(worktreePath: string): Promise<void> {
  try {
    await run(`git worktree remove "${worktreePath}" --force`);
  } catch {
    // If worktree remove fails, try prune
    try {
      await run('git worktree prune');
    } catch {
      // Best effort
    }
  }
}

/**
 * Merge the worktree branch into targetBranch (two-phase).
 * Phase 1: merge targetBranch INTO worktree branch (rebase/merge to get latest)
 * Phase 2: merge worktree branch INTO targetBranch
 */
export async function mergeWorktree(opts: {
  worktreePath: string;
  slug: string;
  targetBranch: string;
  projectRoot: string;
}): Promise<{ success: boolean; conflicts: boolean }> {
  const { worktreePath, slug, targetBranch, projectRoot } = opts;

  try {
    // Phase 1: update worktree branch with target
    await run(
      `git merge "${targetBranch}" --no-edit`,
      worktreePath,
    );
  } catch {
    // Conflicts in phase 1 - abort and report
    try {
      await run('git merge --abort', worktreePath);
    } catch {
      // ignore abort failure
    }
    return { success: false, conflicts: true };
  }

  try {
    // Phase 2: merge worktree branch into target
    await run(
      `git merge "${slug}" --no-edit`,
      projectRoot,
    );
    return { success: true, conflicts: false };
  } catch {
    try {
      await run('git merge --abort', projectRoot);
    } catch {
      // ignore
    }
    return { success: false, conflicts: true };
  }
}

/**
 * Check if there are uncommitted changes in a worktree.
 */
export async function hasUncommittedChanges(
  worktreePath: string,
): Promise<boolean> {
  try {
    const output = await run('git status --porcelain', worktreePath);
    return output.length > 0;
  } catch {
    return false;
  }
}
