import { writeFileSync } from 'fs';
import { join } from 'path';
import { exec as execCb } from 'child_process';
import { promisify } from 'util';
import { tmpdir } from 'os';

const exec = promisify(execCb);

/**
 * Returns the fixed name of the dellij control tab.
 */
export function getControlTabName(): string {
  return 'dellij';
}

/**
 * Generate a multi-pane KDL layout for a new agent tab.
 */
export function generateAgentLayoutKdl(opts: {
  name: string;
  worktreePath: string;
  command: string;
}): string {
  return `layout {
    tab name="${opts.name}" cwd="${opts.worktreePath}" {
        pane split_direction="vertical" {
            pane name="Agent" command="bash" {
                args "-c" "${opts.command}"
                focus true
            }
            pane split_direction="horizontal" size="30%" {
                pane name="Shell" cwd="${opts.worktreePath}"
                pane name="Git Status" command="watch" {
                    args "-n" "2" "--color" "git" "-c" "color.status=always" "status"
                }
            }
        }
    }
}`;
}

/**
 * Generate the KDL layout file content for a new zellij session.
 * If pluginPath is provided and the WASM file should exist, include the plugin pane.
 */
export function generateLayoutKdl(opts: {
  distIndexPath: string;
  projectRoot: string;
  dellijDir: string;
  pluginPath?: string;
}): string {
  const { distIndexPath, projectRoot, dellijDir, pluginPath } = opts;

  const pluginPane = pluginPath
    ? `        pane size=1 borderless=true {
            plugin location="file:${pluginPath}" {
                config_dir "${dellijDir}"
            }
        }`
    : null;

  const defaultTabTemplate = pluginPane
    ? `    default_tab_template {
        children
${pluginPane}
    }`
    : `    default_tab_template {
        children
    }`;

  return `layout {
${defaultTabTemplate}
    tab name="dellij" {
        pane command="bun" {
            args "run" "${distIndexPath}" "--ui" "--project-root" "${projectRoot}"
        }
    }
}
`;
}

/**
 * Write a layout KDL string to a temp file and return the path.
 */
export function writeLayoutFile(layoutContent: string): string {
  const path = join(tmpdir(), `dellij-layout-${Date.now()}.kdl`);
  writeFileSync(path, layoutContent, 'utf8');
  return path;
}

/**
 * Parse the output of `zellij action query-tab-names` into a list of tab names.
 * Output is typically one tab name per line.
 */
export function parseTabNames(output: string): string[] {
  return output
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0);
}

/**
 * Parse the output of `zellij list-sessions` (or `--no-formatting` variant)
 * into a list of session names.
 *
 * The `--no-formatting` output is plain text, one session name per line.
 * Without the flag each line may contain ANSI codes and extra info.
 */
export function parseSessions(output: string): string[] {
  return output
    .split('\n')
    .map((line) =>
      line
        // Strip ANSI escape sequences
        .replace(/\x1b\[[0-9;]*m/g, '')
        .trim(),
    )
    .map((line) => {
      // Lines may be like "session-name (attached)" or just "session-name"
      const match = line.match(/^([^\s(]+)/);
      return match ? match[1] : '';
    })
    .filter((name): name is string => name !== undefined && name.length > 0);
}

/**
 * Check whether the `zellij` binary is available on PATH.
 */
export async function zellijAvailable(): Promise<boolean> {
  try {
    await exec('command -v zellij 2>/dev/null || which zellij 2>/dev/null');
    return true;
  } catch {
    return false;
  }
}
