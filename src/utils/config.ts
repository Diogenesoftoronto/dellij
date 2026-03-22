import {
  readFileSync,
  writeFileSync,
  mkdirSync,
  existsSync,
  renameSync,
} from 'fs';
import { join } from 'path';
import type { DellijConfig, DellijTab, DellijSettings } from '../types.ts';

/**
 * Returns the path to the .dellij directory for a project.
 */
export function getDellijDir(projectRoot: string): string {
  return join(projectRoot, '.dellij');
}

const CONFIG_FILE = 'dellij.config.json';

function configPath(dellijDir: string): string {
  return join(dellijDir, CONFIG_FILE);
}

/**
 * Load config from disk. Throws if the config file does not exist.
 */
export function loadConfig(dellijDir: string): DellijConfig {
  const filePath = configPath(dellijDir);
  const raw = readFileSync(filePath, 'utf8');
  return JSON.parse(raw) as DellijConfig;
}

/**
 * Atomic write of config to disk (write temp file, then rename).
 */
export function saveConfig(dellijDir: string, config: DellijConfig): void {
  const filePath = configPath(dellijDir);
  const tempPath = `${filePath}.tmp`;
  const updated: DellijConfig = {
    ...config,
    lastUpdated: new Date().toISOString(),
  };
  writeFileSync(tempPath, JSON.stringify(updated, null, 2) + '\n', 'utf8');
  renameSync(tempPath, filePath);
}

/**
 * Creates a default config in the .dellij directory.
 * Also creates required subdirectories.
 */
export function initConfig(
  projectRoot: string,
  dellijDir: string,
  overrides?: Partial<DellijConfig>,
): DellijConfig {
  mkdirSync(dellijDir, { recursive: true });
  mkdirSync(join(dellijDir, 'status'), { recursive: true });
  mkdirSync(join(dellijDir, 'hooks'), { recursive: true });
  mkdirSync(join(dellijDir, 'worktrees'), { recursive: true });

  const projectName =
    projectRoot.split('/').filter(Boolean).pop() ?? 'project';

  const defaultSettings: DellijSettings = {
    defaultAgent: 'claude',
    enabledAgents: ['claude', 'opencode', 'codex'],
    permissionMode: '',
  };

  const config: DellijConfig = {
    projectName,
    projectRoot,
    tabs: [],
    settings: defaultSettings,
    controlTabName: 'dellij',
    lastUpdated: new Date().toISOString(),
    ...overrides,
  };

  if (!existsSync(configPath(dellijDir))) {
    writeFileSync(
      configPath(dellijDir),
      JSON.stringify(config, null, 2) + '\n',
      'utf8',
    );
  } else {
    // Config already exists; reload it
    return loadConfig(dellijDir);
  }

  return config;
}

/**
 * Immutably update a tab by ID.
 */
export function updateTab(
  config: DellijConfig,
  tabId: string,
  updates: Partial<DellijTab>,
): DellijConfig {
  return {
    ...config,
    tabs: config.tabs.map((t) => (t.id === tabId ? { ...t, ...updates } : t)),
  };
}

/**
 * Immutably add a tab.
 */
export function addTab(config: DellijConfig, tab: DellijTab): DellijConfig {
  return {
    ...config,
    tabs: [...config.tabs, tab],
  };
}

/**
 * Immutably remove a tab by ID.
 */
export function removeTab(config: DellijConfig, tabId: string): DellijConfig {
  return {
    ...config,
    tabs: config.tabs.filter((t) => t.id !== tabId),
  };
}
