import { existsSync, mkdirSync } from 'fs';
import { join } from 'path';
import { render } from 'ink';
import React from 'react';

import { 
  getProjectRoot, 
  getProjectName, 
  generateSessionHash,
  getBaseBranch,
  createWorktree,
  generateSlug,
  mergeWorktree
} from './utils/git.ts';
import { 
  getDellijDir, 
  initConfig, 
  loadConfig, 
  saveConfig,
  addTab as addTabToConfig,
  removeTab as removeTabFromConfig,
  updateTab as updateTabInConfig
} from './utils/config.ts';
import {
  findLocalStatusPluginPath,
  generateLayoutKdl,
  pluginSupportsZellijRuntime,
  writeLayoutFile,
  zellijAvailable,
  getControlTabName,
} from './utils/zellij.ts';
import { ZellijService } from './services/ZellijService.ts';
import { DellijApp } from './DellijApp.tsx';
import { launchAgentInNewTab, type AgentName } from './utils/agentLaunch.ts';
import type { DellijTab } from './types.ts';
import { HookManager } from './services/HookManager.ts';
import { runDoctor } from './utils/doctor.ts';

function makeId(): string {
  return Math.random().toString(36).slice(2, 10);
}

interface ParsedArgs {
  command: string | null;
  ui: boolean;
  projectRoot: string | null;
  args: string[];
}

function parseArgs(argv: string[]): ParsedArgs {
  const args = argv.slice(2);
  const ui = args.includes('--ui');
  const projectRootIdx = args.indexOf('--project-root');
  const projectRoot =
    projectRootIdx !== -1 ? (args[projectRootIdx + 1] ?? null) : null;
  
  // Filter out flags to find the command
  const filtered = args.filter(a => !a.startsWith('-') && a !== projectRoot);
  const command = filtered[0] ?? null;
  const remainingArgs = filtered.slice(1);

  return { command, ui, projectRoot, args: remainingArgs };
}

function getAttachOptionsArgs(): string[] {
  return [
    'options',
    '--mirror-session',
    'true',
    '--show-release-notes',
    'false',
    '--show-startup-tips',
    'false',
  ];
}

async function main(): Promise<void> {
  const { command, ui, projectRoot: argProjectRoot, args } = parseArgs(process.argv);

  // ── UI-only mode (launched by zellij inside a pane) ───────────────────
  if (ui) {
    const projectRoot = argProjectRoot ?? process.cwd();
    const dellijDir = getDellijDir(projectRoot);

    const config = loadConfig(dellijDir);

    const sessionName =
      config.sessionName ??
      `dellij-${generateSessionHash(projectRoot)}`;
    const controlTabName = config.controlTabName ?? getControlTabName();

    const { unmount } = render(
      React.createElement(DellijApp, {
        config,
        dellijDir,
        sessionName,
        controlTabName,
      }),
    );

    process.on('exit', unmount);
    return;
  }

  // Resolve project root
  let projectRoot: string;
  try {
    projectRoot = await getProjectRoot();
  } catch {
    projectRoot = process.cwd();
  }

  const projectName = getProjectName(projectRoot);
  const sessionName = `dellij-${generateSessionHash(projectRoot)}`;
  const dellijDir = getDellijDir(projectRoot);
  
  // Ensure .dellij directory exists
  mkdirSync(dellijDir, { recursive: true });
  
  const config = loadConfig(dellijDir);
  if (!existsSync(join(dellijDir, 'dellij.config.json'))) {
    initConfig(projectRoot, dellijDir, { sessionName, projectName });
  }

  const zellijService = ZellijService.getInstance();
  const controlTabName = config.controlTabName ?? getControlTabName();

  // ── CLI Subcommands ───────────────────────────────────────────────────
  if (command === 'list') {
    console.log(`Active dellij tabs for ${projectName}:`);
    if (config.tabs.length === 0) {
      console.log('  (no active tabs)');
    } else {
      config.tabs.forEach(tab => {
        const status = tab.type === 'agent' ? ` [${tab.agent}]` : ' [shell]';
        console.log(`  - ${tab.slug}${status}`);
      });
    }
    return;
  }

  if (command === 'new') {
    const prompt = args[0];
    if (!prompt) {
      console.error('Error: please provide a prompt for the new agent.');
      console.log('Usage: dellij new "your prompt here" [--agent <name>]');
      process.exit(1);
    }

    const agentIdx = process.argv.indexOf('--agent');
    const agentName = (agentIdx !== -1 ? process.argv[agentIdx + 1] : (config.settings.defaultAgent || 'gemini')) as AgentName;

    console.log(`Creating ${agentName} agent for: ${prompt}`);
    
    const slug = generateSlug(prompt, agentName);
    const baseBranch = config.settings.baseBranch ?? (await getBaseBranch(projectRoot));

    const worktreePath = await createWorktree({
      projectRoot,
      slug,
      baseBranch,
    });

    const tab: DellijTab = {
      id: makeId(),
      slug,
      prompt,
      agent: agentName,
      agentStatus: 'working',
      worktreePath,
      branchName: slug,
      projectRoot,
      createdAt: new Date().toISOString(),
      type: 'agent',
    };

    const nextConfig = addTabToConfig(config, tab);
    saveConfig(dellijDir, nextConfig);

    // Fire hook
    HookManager.getInstance(projectRoot, dellijDir).runHook('worktree_created', {
      DELLIJ_ROOT: projectRoot,
      DELLIJ_SLUG: slug,
      DELLIJ_AGENT: agentName,
      DELLIJ_WORKTREE_PATH: worktreePath,
      DELLIJ_BRANCH: slug,
      DELLIJ_PROMPT: prompt,
    });

    if (zellijService.isInsideZellij()) {
      await launchAgentInNewTab({
        slug,
        agent: agentName,
        prompt,
        worktreePath,
        permissionMode: config.settings.permissionMode,
        dellijDir,
        controlTabName,
      });
      console.log(`Agent tab created: ${slug}`);
    } else {
      console.log(`Agent worktree created at: ${worktreePath}`);
      console.log(`Run 'dellij' to attach and launch the agent tab.`);
    }
    return;
  }

  if (command === 'merge') {
    const slug = args[0];
    const tab = config.tabs.find(t => t.slug === slug);
    if (!tab || !tab.worktreePath) {
      console.error(`Error: tab with slug '${slug}' not found or has no worktree.`);
      process.exit(1);
    }

    console.log(`Merging ${slug}...`);
    const result = await mergeWorktree({
      worktreePath: tab.worktreePath,
      slug: tab.slug,
      targetBranch: config.settings.baseBranch ?? (await getBaseBranch(projectRoot)),
      projectRoot,
    });

    if (result.success) {
      console.log(`Successfully merged ${slug}.`);
      const nextConfig = updateTabInConfig(config, tab.id, { agentStatus: 'done' });
      saveConfig(dellijDir, nextConfig);
    } else {
      console.error(`Merge failed for ${slug}${result.conflicts ? ' due to conflicts' : ''}.`);
      process.exit(1);
    }
    return;
  }

  if (command === 'close') {
    const slug = args[0];
    const tab = config.tabs.find(t => t.slug === slug);
    if (!tab) {
      console.error(`Error: tab with slug '${slug}' not found.`);
      process.exit(1);
    }

    const nextConfig = removeTabFromConfig(config, tab.id);
    saveConfig(dellijDir, nextConfig);
    console.log(`Closed tab ${slug}.`);
    return;
  }

  if (command === 'doctor') {
    const exitCode = await runDoctor({
      cwd: process.cwd(),
      projectRoot,
      dellijDir,
      sessionName,
      config,
    });
    process.exit(exitCode);
  }

  // ── Session-manager mode ──────────────────────────────────────────────
  if (!(await zellijAvailable())) {
    console.error(
      'Error: zellij is not installed or not in PATH.\n' +
        'Install it from https://zellij.dev',
    );
    process.exit(1);
  }

  // ── Already inside a zellij session ──────────────────────────────────
  if (zellijService.isInsideZellij()) {
    const { unmount } = render(
      React.createElement(DellijApp, {
        config,
        dellijDir,
        sessionName: zellijService.getSessionName() ?? sessionName,
        controlTabName,
      }),
    );
    process.on('exit', unmount);
    return;
  }

  // ── Outside zellij: create or attach session ──────────────────────────
  
  // Best-effort: try to delete session if it exists but is exited
  if (await zellijService.hasExitedSession(sessionName)) {
    console.log(`Cleaning up exited session: ${sessionName}`);
    await zellijService.deleteSession(sessionName);
  }

  const sessionExists = await zellijService.sessionExists(sessionName);

  if (sessionExists) {
    console.log(`Attaching to dellij session: ${sessionName}`);
    const { execFileSync } = await import('child_process');
    execFileSync(
      'zellij',
      ['attach', sessionName, ...getAttachOptionsArgs()],
      { stdio: 'inherit' },
    );
    return;
  }

  // Create a new session with layout
  console.log(`Creating dellij session: ${sessionName} for ${projectName}`);

  const srcDir = import.meta.dir ?? join(process.cwd(), 'src');
  const distIndexPath = join(srcDir, 'index.ts');
  const pluginCandidate = findLocalStatusPluginPath(srcDir);
  const pluginPath =
    pluginCandidate && pluginSupportsZellijRuntime(pluginCandidate)
      ? pluginCandidate
      : undefined;

  if (pluginCandidate && !pluginPath) {
    console.warn(
      `Skipping incompatible dellij status plugin at ${pluginCandidate} (missing _start export).`,
    );
  }

  const layoutKdl = generateLayoutKdl({
    distIndexPath,
    projectRoot,
    dellijDir,
    pluginPath,
  });

  const layoutFile = writeLayoutFile(layoutKdl);

  try {
    const { execFileSync } = await import('child_process');
    
    // Use 'attach -c' with '--layout' to be more robust
    execFileSync(
      'zellij',
      ['--layout', layoutFile, 'attach', '-c', sessionName, ...getAttachOptionsArgs()],
      { stdio: 'inherit' },
    );
  } finally {
    try {
      const { unlinkSync } = await import('fs');
      unlinkSync(layoutFile);
    } catch {
      // Best-effort cleanup
    }
  }
}

main().catch((err: unknown) => {
  console.error('dellij error:', err instanceof Error ? err.message : String(err));
  process.exit(1);
});
