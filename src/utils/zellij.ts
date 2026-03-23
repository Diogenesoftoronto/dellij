import { existsSync, readFileSync, writeFileSync } from 'fs';
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
 * Returns the locally built status plugin path, preferring the current
 * wasip1 artifact and falling back to the older wasi target if present.
 */
export function findLocalStatusPluginPath(srcDir: string): string | undefined {
  const pluginWasmPathP1 = join(
    srcDir,
    '..',
    'plugin',
    'target',
    'wasm32-wasip1',
    'release',
    'dellij_status.wasm',
  );
  const pluginWasmPathWasi = join(
    srcDir,
    '..',
    'plugin',
    'target',
    'wasm32-wasi',
    'release',
    'dellij_status.wasm',
  );

  if (existsSync(pluginWasmPathP1)) {
    return pluginWasmPathP1;
  }
  if (existsSync(pluginWasmPathWasi)) {
    return pluginWasmPathWasi;
  }
  return undefined;
}

/**
 * Zellij currently expects plugin wasm artifacts to expose a WASI `_start`
 * entrypoint. Some locally built artifacts exist on disk but are still
 * incompatible with the packaged Zellij runtime, so we guard plugin loading
 * before adding them to a layout.
 */
export function pluginSupportsZellijRuntime(pluginPath: string): boolean {
  try {
    const bytes = readFileSync(pluginPath);

    // WebAssembly binary header: magic (4 bytes) + version (4 bytes)
    if (
      bytes.length < 8 ||
      bytes[0] !== 0x00 ||
      bytes[1] !== 0x61 ||
      bytes[2] !== 0x73 ||
      bytes[3] !== 0x6d
    ) {
      return false;
    }

    let offset = 8;

    const readVarUint32 = (): number => {
      let result = 0;
      let shift = 0;

      while (offset < bytes.length) {
        const byte = bytes[offset++];
        result |= (byte & 0x7f) << shift;
        if ((byte & 0x80) === 0) {
          return result >>> 0;
        }
        shift += 7;
      }

      throw new Error('Unexpected end of wasm while reading varuint32');
    };

    while (offset < bytes.length) {
      const sectionId = bytes[offset++];
      const sectionSize = readVarUint32();
      const sectionEnd = offset + sectionSize;

      if (sectionEnd > bytes.length) {
        return false;
      }

      // Export section
      if (sectionId === 7) {
        const exportCount = readVarUint32();
        for (let i = 0; i < exportCount; i++) {
          const nameLen = readVarUint32();
          const name = bytes.subarray(offset, offset + nameLen).toString('utf8');
          offset += nameLen;

          // Skip export kind + index
          offset += 1;
          readVarUint32();

          if (name === '_start') {
            return true;
          }
        }

        return false;
      }

      offset = sectionEnd;
    }

    return false;
  } catch {
    return false;
  }
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
    .filter((line) => !line.includes('(EXITED)'))
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
