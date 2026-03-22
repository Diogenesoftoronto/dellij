import { exec as execCb } from 'child_process';
import { promisify } from 'util';
import { parseTabNames, parseSessions } from '../utils/zellij.ts';
import { join } from 'path';
import { tmpdir } from 'os';
import { readFileSync, unlinkSync } from 'fs';

const exec = promisify(execCb);

/**
 * Retry strategy for zellij operations
 */
export enum RetryStrategy {
  NONE = 'none',
  FAST = 'fast',
  IDEMPOTENT = 'idempotent',
}

interface RetryConfig {
  strategy: RetryStrategy;
  maxRetries: number;
  baseDelay: number; // milliseconds
  maxDelay: number; // cap for exponential backoff
}

const RETRY_CONFIGS: Record<RetryStrategy, RetryConfig> = {
  [RetryStrategy.NONE]: { strategy: RetryStrategy.NONE, maxRetries: 0, baseDelay: 0, maxDelay: 0 },
  [RetryStrategy.FAST]: { strategy: RetryStrategy.FAST, maxRetries: 2, baseDelay: 50, maxDelay: 100 },
  [RetryStrategy.IDEMPOTENT]: { strategy: RetryStrategy.IDEMPOTENT, maxRetries: 3, baseDelay: 100, maxDelay: 500 },
};

// Errors that should NEVER be retried
const PERMANENT_ERRORS = [
  'zellij not found',
  'command not found',
  'permission denied',
  'no such session',
  'no session found',
];

function isPermanentError(error: unknown): boolean {
  const message = String(error).toLowerCase();
  return PERMANENT_ERRORS.some(pattern => message.includes(pattern));
}

export class ZellijService {
  private static instance: ZellijService;

  private constructor() {}

  static getInstance(): ZellijService {
    if (!ZellijService.instance) {
      ZellijService.instance = new ZellijService();
    }
    return ZellijService.instance;
  }

  /**
   * Execute a zellij command with retry logic
   */
  private async executeWithRetry<T>(
    operation: () => Promise<T>,
    strategy: RetryStrategy = RetryStrategy.IDEMPOTENT,
    context?: string
  ): Promise<T> {
    const config = RETRY_CONFIGS[strategy];

    if (config.maxRetries === 0) {
      return operation();
    }

    let lastError: unknown;
    for (let attempt = 0; attempt <= config.maxRetries; attempt++) {
      try {
        return await operation();
      } catch (error) {
        lastError = error;

        // Don't retry permanent errors
        if (isPermanentError(error)) {
          throw error;
        }

        // Don't sleep on last attempt
        if (attempt < config.maxRetries) {
          const delay = Math.min(config.baseDelay * (attempt + 1), config.maxDelay);
          await this.sleep(delay);
        }
      }
    }

    throw lastError;
  }

  // ── Session ──────────────────────────────────────────────────────────────

  async listSessions(): Promise<string[]> {
    try {
      const output = await this.executeWithRetry(
        () => this.exec('zellij list-sessions --no-formatting 2>/dev/null || zellij list-sessions 2>/dev/null'),
        RetryStrategy.IDEMPOTENT
      );
      return parseSessions(output);
    } catch (err) {
      const message = String(err).toLowerCase();
      if (message.includes('no active session') || message.includes('no session found')) {
        return [];
      }
      return [];
    }
  }

  async sessionExists(name: string): Promise<boolean> {
    const sessions = await this.listSessions();
    return sessions.includes(name);
  }

  async hasExitedSession(name: string): Promise<boolean> {
    try {
      const output = await this.exec('zellij list-sessions --no-formatting 2>/dev/null || zellij list-sessions 2>/dev/null');
      return output.includes(`${name}`) && output.includes('(EXITED)');
    } catch {
      return false;
    }
  }

  async deleteSession(name: string): Promise<void> {
    try {
      await this.exec(`zellij delete-session ${name}`);
    } catch {
      // Best effort
    }
  }

  isInsideZellij(): boolean {
    return !!process.env['ZELLIJ'];
  }

  getSessionName(): string | undefined {
    return process.env['ZELLIJ_SESSION_NAME'];
  }

  // ── Tab operations ───────────────────────────────────────────────────────

  async newTab(name: string, cwd?: string): Promise<void> {
    const cwdArg = cwd ? ` --cwd "${cwd}"` : '';
    await this.executeWithRetry(
      () => this.action(`new-tab --name "${name}"${cwdArg}`),
      RetryStrategy.FAST
    );
  }

  async newTabWithLayout(layoutPath: string): Promise<void> {
    await this.executeWithRetry(
      () => this.action(`new-tab --layout "${layoutPath}"`),
      RetryStrategy.FAST
    );
  }

  async closeTab(): Promise<void> {
    await this.executeWithRetry(
      () => this.action('close-tab'),
      RetryStrategy.NONE
    );
  }

  async goToTab(name: string): Promise<void> {
    await this.executeWithRetry(
      () => this.action(`go-to-tab-name "${name}"`),
      RetryStrategy.FAST
    );
  }

  async renameTab(name: string): Promise<void> {
    await this.executeWithRetry(
      () => this.action(`rename-tab "${name}"`),
      RetryStrategy.FAST
    );
  }

  async listTabs(): Promise<string[]> {
    try {
      const output = await this.executeWithRetry(
        () => this.action('query-tab-names'),
        RetryStrategy.IDEMPOTENT
      );
      return parseTabNames(output);
    } catch {
      return [];
    }
  }

  async tabExists(name: string): Promise<boolean> {
    const tabs = await this.listTabs();
    return tabs.includes(name);
  }

  // ── Pane operations ──────────────────────────────────────────────────────

  async writeChars(chars: string): Promise<void> {
    // Escape single quotes for shell safety
    const escaped = chars.replace(/'/g, "'\\''");
    await this.executeWithRetry(
      () => this.action(`write-chars '${escaped}'`),
      RetryStrategy.FAST
    );
  }

  async newPane(
    direction?: 'up' | 'down' | 'left' | 'right',
    floating?: boolean,
  ): Promise<void> {
    const dirArg = direction ? ` --direction ${direction}` : '';
    const floatArg = floating ? ' --floating' : '';
    await this.executeWithRetry(
      () => this.action(`new-pane${dirArg}${floatArg}`),
      RetryStrategy.FAST
    );
  }

  async closePane(): Promise<void> {
    await this.executeWithRetry(
      () => this.action('close-pane'),
      RetryStrategy.NONE
    );
  }

  async dumpScreen(path: string): Promise<void> {
    await this.executeWithRetry(
      () => this.action(`dump-screen "${path}"`),
      RetryStrategy.IDEMPOTENT
    );
  }

  async getPaneContent(): Promise<string> {
    const tempPath = join(tmpdir(), `dellij-screen-${Date.now()}-${Math.random().toString(36).slice(2)}.txt`);
    try {
      await this.dumpScreen(tempPath);
      const content = readFileSync(tempPath, 'utf8');
      return content;
    } catch (err) {
      throw new Error(`Failed to get pane content: ${err instanceof Error ? err.message : String(err)}`);
    } finally {
      try {
        unlinkSync(tempPath);
      } catch {
        // Ignore errors during cleanup
      }
    }
  }

  async renamePane(name: string): Promise<void> {
    await this.executeWithRetry(
      () => this.action(`rename-pane "${name}"`),
      RetryStrategy.FAST
    );
  }

  // ── High-level helpers ───────────────────────────────────────────────────

  /**
   * Launch an agent in a new zellij tab.
   * 1. Create the tab (with cwd set to worktree path)
   * 2. Wait briefly for the shell to be ready
   * 3. Write the command chars + newline to start the agent
   * 4. Navigate back to the control tab
   */
  async launchAgentTab(opts: {
    slug: string;
    worktreePath: string;
    command: string;
    controlTabName: string;
  }): Promise<void> {
    const { slug, worktreePath, command, controlTabName } = opts;

    await this.newTab(slug, worktreePath);
    await this.sleep(300);
    await this.writeChars(`${command}\n`);
    await this.sleep(100);
    await this.goToTab(controlTabName);
  }

  // ── Low-level ────────────────────────────────────────────────────────────

  async action(args: string): Promise<string> {
    return this.exec(`zellij action ${args}`);
  }

  async exec(cmd: string): Promise<string> {
    try {
      const { stdout } = await exec(cmd);
      return stdout.trim();
    } catch (err: unknown) {
      if (err instanceof Error && 'stderr' in err) {
        const cmdErr = err as { stderr?: string; message?: string };
        const detail = cmdErr.stderr || cmdErr.message || String(err);
        throw new Error(`Command failed: ${cmd}\n${detail}`);
      }
      throw err;
    }
  }

  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
}

