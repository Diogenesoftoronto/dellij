import { existsSync, mkdirSync } from 'fs';
import { join } from 'path';
import { render } from 'ink';
import React from 'react';

import { getProjectRoot, getProjectName, generateSessionHash } from './utils/git.ts';
import { getDellijDir, initConfig, loadConfig } from './utils/config.ts';
import {
  generateLayoutKdl,
  writeLayoutFile,
  zellijAvailable,
  getControlTabName,
} from './utils/zellij.ts';
import { ZellijService } from './services/ZellijService.ts';
import { DellijApp } from './DellijApp.tsx';


function parseArgs(argv: string[]): {
  ui: boolean;
  projectRoot: string | null;
} {
  const args = argv.slice(2);
  const ui = args.includes('--ui');
  const projectRootIdx = args.indexOf('--project-root');
  const projectRoot =
    projectRootIdx !== -1 ? (args[projectRootIdx + 1] ?? null) : null;
  return { ui, projectRoot };
}

async function main(): Promise<void> {
  const { ui, projectRoot: argProjectRoot } = parseArgs(process.argv);

  // ── UI-only mode (launched by zellij inside a pane) ───────────────────
  if (ui) {
    const projectRoot = argProjectRoot ?? process.cwd();
    const dellijDir = getDellijDir(projectRoot);

    let config = existsSync(join(dellijDir, 'dellij.config.json'))
      ? loadConfig(dellijDir)
      : initConfig(projectRoot, dellijDir);

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

    // Keep process alive until React exits
    process.on('exit', unmount);
    return;
  }

  // ── Session-manager mode ──────────────────────────────────────────────
  if (!(await zellijAvailable())) {
    console.error(
      'Error: zellij is not installed or not in PATH.\n' +
        'Install it from https://zellij.dev',
    );
    process.exit(1);
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

  // Ensure .dellij directory + config exist
  mkdirSync(dellijDir, { recursive: true });
  const config = initConfig(projectRoot, dellijDir, {
    sessionName,
    projectName,
  });

  const controlTabName = config.controlTabName ?? getControlTabName();
  const zellijService = ZellijService.getInstance();

  // ── Already inside a zellij session ──────────────────────────────────
  if (zellijService.isInsideZellij()) {
    // Just render the TUI directly in this pane
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
  const sessionExists = await zellijService.sessionExists(sessionName);

  if (sessionExists) {
    // Attach to existing session
    console.log(`Attaching to dellij session: ${sessionName}`);
    const { execFileSync } = await import('child_process');
    execFileSync('zellij', ['attach', sessionName], { stdio: 'inherit' });
    return;
  }

  // Create a new session with layout
  console.log(`Creating dellij session: ${sessionName} for ${projectName}`);

  const srcDir = import.meta.dir;
  const distIndexPath = join(srcDir, 'index.ts');
  const pluginWasmPathP1 = join(srcDir, '..', 'plugin', 'target', 'wasm32-wasip1', 'release', 'dellij_status.wasm');
  const pluginWasmPathWasi = join(srcDir, '..', 'plugin', 'target', 'wasm32-wasi', 'release', 'dellij_status.wasm');
  const pluginPath = existsSync(pluginWasmPathP1) ? pluginWasmPathP1 : (existsSync(pluginWasmPathWasi) ? pluginWasmPathWasi : undefined);

  const layoutKdl = generateLayoutKdl({
    distIndexPath,
    projectRoot,
    dellijDir,
    pluginPath,
  });

  const layoutFile = writeLayoutFile(layoutKdl);

  try {
    const { execFileSync } = await import('child_process');
    execFileSync(
      'zellij',
      ['--session', sessionName, '--layout', layoutFile],
      { stdio: 'inherit' },
    );
  } finally {
    // Clean up temp layout file
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
