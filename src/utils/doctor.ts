import { exec as execCb } from 'child_process';
import { existsSync } from 'fs';
import { join } from 'path';
import { promisify } from 'util';
import type { DellijConfig } from '../types.ts';
import {
  getAgentDefinition,
  resolveEnabledAgents,
} from './agentLaunch.ts';
import { findLocalStatusPluginPath, pluginSupportsZellijRuntime } from './zellij.ts';

const exec = promisify(execCb);

type CheckLevel = 'ok' | 'warn' | 'fail';

interface DoctorCheck {
  level: CheckLevel;
  label: string;
  detail: string;
}

interface CommandCheckResult {
  found: boolean;
  path?: string;
}

async function findCommand(command: string): Promise<CommandCheckResult> {
  try {
    const { stdout } = await exec(
      `command -v ${command} 2>/dev/null || which ${command} 2>/dev/null`,
    );
    const resolved = stdout.trim();
    return resolved ? { found: true, path: resolved } : { found: false };
  } catch {
    return { found: false };
  }
}

async function runShellCheck(command: string): Promise<boolean> {
  try {
    await exec(command);
    return true;
  } catch {
    return false;
  }
}

function formatCheck(check: DoctorCheck): string {
  return `[${check.level}] ${check.label}: ${check.detail}`;
}

export async function runDoctor(opts: {
  cwd: string;
  projectRoot: string;
  dellijDir: string;
  sessionName: string;
  config: DellijConfig;
}): Promise<number> {
  const { cwd, projectRoot, dellijDir, sessionName, config } = opts;

  const checks: DoctorCheck[] = [];

  const dellijCmd = await findCommand('dellij');
  checks.push(
    dellijCmd.found
      ? {
          level: 'ok',
          label: 'dellij',
          detail: dellijCmd.path ?? 'found on PATH',
        }
      : {
          level: 'fail',
          label: 'dellij',
          detail: 'not found on PATH',
        },
  );

  for (const command of ['bun', 'git', 'zellij']) {
    const result = await findCommand(command);
    checks.push(
      result.found
        ? {
            level: 'ok',
            label: command,
            detail: result.path ?? 'found on PATH',
          }
        : {
            level: command === 'zellij' ? 'fail' : 'fail',
            label: command,
            detail: 'not found on PATH',
          },
    );
  }

  const gitRepoDetected = await runShellCheck('git rev-parse --show-toplevel');
  checks.push({
    level: gitRepoDetected ? 'ok' : 'fail',
    label: 'git repo',
    detail: gitRepoDetected
      ? projectRoot
      : 'current directory is not inside a git repository',
  });

  checks.push({
    level: cwd === projectRoot ? 'ok' : 'warn',
    label: 'working directory',
    detail:
      cwd === projectRoot
        ? `at project root (${projectRoot})`
        : `inside repo but not at root (cwd: ${cwd}, root: ${projectRoot})`,
  });

  checks.push({
    level: existsSync(dellijDir) ? 'ok' : 'warn',
    label: '.dellij directory',
    detail: existsSync(dellijDir)
      ? `${dellijDir}`
      : `not created yet; will be created on first run at ${dellijDir}`,
  });

  const configPath = join(dellijDir, 'dellij.config.json');
  checks.push({
    level: existsSync(configPath) ? 'ok' : 'warn',
    label: 'config',
    detail: existsSync(configPath)
      ? configPath
      : `not created yet; will be created on first run at ${configPath}`,
  });

  checks.push({
    level: 'ok',
    label: 'session',
    detail: sessionName,
  });

  const srcDir = join(import.meta.dir ?? join(process.cwd(), 'src', 'utils'), '..');
  const pluginCandidate = findLocalStatusPluginPath(srcDir);
  if (!pluginCandidate) {
    checks.push({
      level: 'warn',
      label: 'status plugin',
      detail: 'no local plugin build found; dellij will run without the status ribbon',
    });
  } else if (pluginSupportsZellijRuntime(pluginCandidate)) {
    checks.push({
      level: 'ok',
      label: 'status plugin',
      detail: `compatible build found at ${pluginCandidate}`,
    });
  } else {
    checks.push({
      level: 'warn',
      label: 'status plugin',
      detail: `incompatible build at ${pluginCandidate}; dellij will skip it`,
    });
  }

  const enabledAgents = resolveEnabledAgents(config.settings.enabledAgents);
  if (enabledAgents.length === 0) {
    checks.push({
      level: 'warn',
      label: 'enabled agents',
      detail: 'no enabled agents configured',
    });
  } else {
    for (const agent of enabledAgents) {
      const definition = getAgentDefinition(agent);
      const available = await runShellCheck(definition.installTestCommand);
      checks.push({
        level: available ? 'ok' : 'warn',
        label: `agent:${agent}`,
        detail: available
          ? `${definition.name} available`
          : `${definition.name} not found; \`dellij new ... --agent ${agent}\` will not work`,
      });
    }
  }

  const defaultAgent = config.settings.defaultAgent;
  if (defaultAgent) {
    const enabled = enabledAgents.includes(defaultAgent as typeof enabledAgents[number]);
    checks.push({
      level: enabled ? 'ok' : 'warn',
      label: 'default agent',
      detail: enabled
        ? defaultAgent
        : `${defaultAgent} is configured but not in enabledAgents`,
    });
  }

  const hasFailures = checks.some((check) => check.level === 'fail');
  const hasWarnings = checks.some((check) => check.level === 'warn');

  console.log(`Dellij doctor for ${projectRoot}`);
  console.log('');
  for (const check of checks) {
    console.log(formatCheck(check));
  }
  console.log('');

  if (hasFailures) {
    console.log('Result: not ready');
    return 1;
  }
  if (hasWarnings) {
    console.log('Result: ready with warnings');
    return 0;
  }
  console.log('Result: ready');
  return 0;
}
