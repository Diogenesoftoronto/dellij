import { writeFileSync } from 'fs';
import { join } from 'path';
import { ZellijService } from '../services/ZellijService.ts';
import type { PermissionMode } from '../types.ts';
import { generateAgentLayoutKdl, writeLayoutFile } from './zellij.ts';

export const AGENT_IDS = [
  'claude',
  'opencode',
  'codex',
  'cline',
  'gemini',
  'qwen',
  'amp',
  'pi',
  'cursor',
  'copilot',
  'crush',
  'aider',
] as const;

export type AgentName = (typeof AGENT_IDS)[number];
export type PromptTransport = 'positional' | 'option' | 'stdin' | 'send-keys';

export interface AgentRegistryEntry {
  id: AgentName;
  name: string;
  shortLabel: string;
  description: string;
  slugSuffix: string;
  installTestCommand: string;
  commonPaths: string[];
  promptCommand: string;
  noPromptCommand?: string;
  promptTransport: PromptTransport;
  promptOption?: string;
  sendKeysPrePrompt?: string[];
  sendKeysSubmit?: string[];
  sendKeysPostPasteDelayMs?: number;
  sendKeysReadyDelayMs?: number;
  permissionFlags: Partial<Record<Exclude<PermissionMode, ''>, string>>;
  defaultEnabled: boolean;
  resumeCommandTemplate?: string;
}

const HOME = process.env['HOME'] || '';
const homePath = (suffix: string): string[] =>
  HOME ? [`${HOME}/${suffix}`] : [];

export const AGENT_REGISTRY: Readonly<Record<AgentName, AgentRegistryEntry>> =
  {
    claude: {
      id: 'claude',
      name: 'Claude Code',
      shortLabel: 'cc',
      description: 'Anthropic Claude Code CLI',
      slugSuffix: 'claude-code',
      installTestCommand:
        'command -v claude 2>/dev/null || which claude 2>/dev/null',
      commonPaths: [
        ...homePath('.claude/local/claude'),
        ...homePath('.local/bin/claude'),
        '/usr/local/bin/claude',
        '/opt/homebrew/bin/claude',
        '/usr/bin/claude',
        ...homePath('bin/claude'),
      ],
      promptCommand: 'claude',
      promptTransport: 'positional',
      permissionFlags: {
        plan: '--permission-mode plan',
        acceptEdits: '--permission-mode acceptEdits',
        bypassPermissions: '--dangerously-skip-permissions',
      },
      defaultEnabled: true,
      resumeCommandTemplate: 'claude --continue{permissions}',
    },
    opencode: {
      id: 'opencode',
      name: 'OpenCode',
      shortLabel: 'oc',
      description: 'OpenCode CLI',
      slugSuffix: 'opencode',
      installTestCommand:
        'command -v opencode 2>/dev/null || which opencode 2>/dev/null',
      commonPaths: [
        '/opt/homebrew/bin/opencode',
        '/usr/local/bin/opencode',
        ...homePath('.local/bin/opencode'),
        ...homePath('bin/opencode'),
      ],
      promptCommand: 'opencode',
      promptTransport: 'option',
      promptOption: '--prompt',
      permissionFlags: {},
      defaultEnabled: true,
    },
    codex: {
      id: 'codex',
      name: 'Codex',
      shortLabel: 'cx',
      description: 'OpenAI Codex CLI',
      slugSuffix: 'codex',
      installTestCommand:
        'command -v codex 2>/dev/null || which codex 2>/dev/null',
      commonPaths: [
        '/usr/local/bin/codex',
        '/opt/homebrew/bin/codex',
        ...homePath('.local/bin/codex'),
        ...homePath('bin/codex'),
        ...homePath('.npm-global/bin/codex'),
      ],
      promptCommand: 'codex',
      promptTransport: 'positional',
      permissionFlags: {
        acceptEdits: '--full-auto',
        bypassPermissions: '--dangerously-bypass-approvals-and-sandbox',
      },
      defaultEnabled: true,
      resumeCommandTemplate: 'codex resume --last{permissions}',
    },
    cline: {
      id: 'cline',
      name: 'Cline CLI',
      shortLabel: 'cl',
      description: 'Cline terminal coding agent',
      slugSuffix: 'cline',
      installTestCommand:
        'command -v cline 2>/dev/null || which cline 2>/dev/null',
      commonPaths: [
        '/usr/local/bin/cline',
        '/opt/homebrew/bin/cline',
        ...homePath('.local/bin/cline'),
        ...homePath('bin/cline'),
      ],
      promptCommand: 'cline',
      promptTransport: 'send-keys',
      sendKeysPostPasteDelayMs: 120,
      sendKeysReadyDelayMs: 2500,
      permissionFlags: {
        plan: '--plan',
        acceptEdits: '--act',
        bypassPermissions: '--act --yolo',
      },
      defaultEnabled: false,
    },
    gemini: {
      id: 'gemini',
      name: 'Gemini CLI',
      shortLabel: 'gm',
      description: 'Google Gemini CLI',
      slugSuffix: 'gemini',
      installTestCommand:
        'command -v gemini 2>/dev/null || which gemini 2>/dev/null',
      commonPaths: [
        '/usr/local/bin/gemini',
        '/opt/homebrew/bin/gemini',
        ...homePath('.local/bin/gemini'),
        ...homePath('bin/gemini'),
        ...homePath('.npm-global/bin/gemini'),
      ],
      promptCommand: 'gemini',
      promptTransport: 'option',
      promptOption: '--prompt-interactive',
      permissionFlags: {
        plan: '--approval-mode plan',
        acceptEdits: '--approval-mode auto_edit',
        bypassPermissions: '--approval-mode yolo',
      },
      defaultEnabled: false,
      resumeCommandTemplate: 'gemini --resume latest{permissions}',
    },
    qwen: {
      id: 'qwen',
      name: 'Qwen CLI',
      shortLabel: 'qn',
      description: 'Qwen Code CLI',
      slugSuffix: 'qwen',
      installTestCommand:
        'command -v qwen 2>/dev/null || which qwen 2>/dev/null',
      commonPaths: [
        '/usr/local/bin/qwen',
        '/opt/homebrew/bin/qwen',
        ...homePath('.local/bin/qwen'),
        ...homePath('bin/qwen'),
        ...homePath('.npm-global/bin/qwen'),
      ],
      promptCommand: 'qwen',
      promptTransport: 'option',
      promptOption: '-i',
      permissionFlags: {
        plan: '--approval-mode plan',
        acceptEdits: '--approval-mode auto-edit',
        bypassPermissions: '--approval-mode yolo',
      },
      defaultEnabled: false,
      resumeCommandTemplate: 'qwen --continue{permissions}',
    },
    amp: {
      id: 'amp',
      name: 'Amp CLI',
      shortLabel: 'ap',
      description: 'Sourcegraph Amp CLI',
      slugSuffix: 'amp',
      installTestCommand:
        'command -v amp 2>/dev/null || which amp 2>/dev/null',
      commonPaths: [
        '/usr/local/bin/amp',
        '/opt/homebrew/bin/amp',
        ...homePath('.local/bin/amp'),
        ...homePath('bin/amp'),
        ...homePath('.npm-global/bin/amp'),
      ],
      promptCommand: 'amp',
      promptTransport: 'stdin',
      permissionFlags: {
        bypassPermissions: '--dangerously-allow-all',
      },
      defaultEnabled: false,
    },
    pi: {
      id: 'pi',
      name: 'pi CLI',
      shortLabel: 'pi',
      description: 'pi coding agent CLI',
      slugSuffix: 'pi',
      installTestCommand: 'command -v pi 2>/dev/null || which pi 2>/dev/null',
      commonPaths: [
        '/usr/local/bin/pi',
        '/opt/homebrew/bin/pi',
        ...homePath('.local/bin/pi'),
        ...homePath('bin/pi'),
        ...homePath('.npm-global/bin/pi'),
      ],
      promptCommand: 'pi',
      promptTransport: 'positional',
      permissionFlags: {
        plan: '--tools read,grep,find,ls',
      },
      defaultEnabled: false,
      resumeCommandTemplate: 'pi --continue{permissions}',
    },
    cursor: {
      id: 'cursor',
      name: 'Cursor CLI',
      shortLabel: 'cr',
      description: 'Cursor agent CLI',
      slugSuffix: 'cursor',
      installTestCommand:
        'command -v cursor-agent 2>/dev/null || which cursor-agent 2>/dev/null',
      commonPaths: [
        ...homePath('.cursor/bin/cursor-agent'),
        '/usr/local/bin/cursor-agent',
        '/opt/homebrew/bin/cursor-agent',
        ...homePath('.local/bin/cursor-agent'),
        ...homePath('bin/cursor-agent'),
      ],
      promptCommand: 'cursor-agent',
      promptTransport: 'positional',
      permissionFlags: {},
      defaultEnabled: false,
    },
    copilot: {
      id: 'copilot',
      name: 'Copilot CLI',
      shortLabel: 'co',
      description: 'GitHub Copilot CLI',
      slugSuffix: 'copilot',
      installTestCommand:
        'command -v copilot 2>/dev/null || which copilot 2>/dev/null',
      commonPaths: [
        '/usr/local/bin/copilot',
        '/opt/homebrew/bin/copilot',
        ...homePath('.local/bin/copilot'),
        ...homePath('bin/copilot'),
        ...homePath('.npm-global/bin/copilot'),
      ],
      promptCommand: 'copilot',
      promptTransport: 'option',
      promptOption: '-i',
      permissionFlags: {
        acceptEdits: '--allow-tool write',
        bypassPermissions: '--allow-all',
      },
      defaultEnabled: false,
      resumeCommandTemplate: 'copilot --continue{permissions}',
    },
    crush: {
      id: 'crush',
      name: 'Crush CLI',
      shortLabel: 'cs',
      description: 'Charmbracelet Crush CLI',
      slugSuffix: 'crush',
      installTestCommand:
        'command -v crush 2>/dev/null || which crush 2>/dev/null',
      commonPaths: [
        '/usr/local/bin/crush',
        '/opt/homebrew/bin/crush',
        ...homePath('.local/bin/crush'),
        ...homePath('bin/crush'),
        ...homePath('.npm-global/bin/crush'),
      ],
      promptCommand: 'crush run',
      noPromptCommand: 'crush',
      promptTransport: 'send-keys',
      sendKeysPrePrompt: ['Escape', 'Tab'],
      sendKeysSubmit: ['Enter'],
      sendKeysPostPasteDelayMs: 200,
      sendKeysReadyDelayMs: 1200,
      permissionFlags: {
        bypassPermissions: '--yolo',
      },
      defaultEnabled: false,
    },
    aider: {
      id: 'aider',
      name: 'Aider',
      shortLabel: 'ai',
      description: 'Aider AI pair programming CLI',
      slugSuffix: 'aider',
      installTestCommand: 'command -v aider 2>/dev/null',
      commonPaths: [
        '/usr/local/bin/aider',
        '/opt/homebrew/bin/aider',
        ...homePath('.local/bin/aider'),
        ...homePath('bin/aider'),
        ...homePath('.npm-global/bin/aider'),
      ],
      promptCommand: 'aider',
      promptTransport: 'option',
      promptOption: '--message',
      permissionFlags: {
        bypassPermissions: '--yes',
      },
      defaultEnabled: false,
    },
  };

// Runtime validation
for (const agentId of AGENT_IDS) {
  const shortLabel = AGENT_REGISTRY[agentId].shortLabel;
  if (shortLabel.length !== 2) {
    throw new Error(
      `Invalid shortLabel for agent "${agentId}": expected 2 characters, received "${shortLabel}"`,
    );
  }
}

const shortLabelSet = new Set<string>();
for (const agentId of AGENT_IDS) {
  const shortLabel = AGENT_REGISTRY[agentId].shortLabel;
  if (shortLabelSet.has(shortLabel)) {
    throw new Error(`Duplicate shortLabel "${shortLabel}" in agent registry`);
  }
  shortLabelSet.add(shortLabel);
}

// ── Helper functions ──────────────────────────────────────────────────────

export function isAgentName(value: string): value is AgentName {
  return (AGENT_IDS as readonly string[]).includes(value);
}

export function getAgentDefinitions(): AgentRegistryEntry[] {
  return AGENT_IDS.map((agent) => AGENT_REGISTRY[agent]);
}

export function getAgentDefinition(agent: AgentName): AgentRegistryEntry {
  return AGENT_REGISTRY[agent];
}

export function getAgentShortLabel(agent: AgentName): string {
  return AGENT_REGISTRY[agent].shortLabel;
}

export function getDefaultEnabledAgents(): AgentName[] {
  return AGENT_IDS.filter((agent) => AGENT_REGISTRY[agent].defaultEnabled);
}

export function resolveEnabledAgents(
  enabledAgents: readonly string[] | undefined,
): AgentName[] {
  if (Array.isArray(enabledAgents)) {
    const configured = new Set(enabledAgents.filter(isAgentName));
    return AGENT_IDS.filter((agent) => configured.has(agent));
  }
  return getDefaultEnabledAgents();
}

export function getPermissionFlags(
  agent: AgentName,
  permissionMode: PermissionMode | undefined,
): string {
  const mode = permissionMode || '';
  if (mode === '') return '';
  return AGENT_REGISTRY[agent].permissionFlags[mode] || '';
}

function appendFlags(base: string, flags: string): string {
  return flags ? `${base} ${flags}` : base;
}

export function buildAgentCommand(
  agent: AgentName,
  permissionMode: PermissionMode | undefined,
): string {
  const definition = AGENT_REGISTRY[agent];
  const baseCommand = definition.noPromptCommand || definition.promptCommand;
  return appendFlags(baseCommand, getPermissionFlags(agent, permissionMode));
}

function escapeShellArg(s: string): string {
  return "'" + s.replace(/'/g, "'\\''") + "'";
}

export function buildInitialPromptCommand(
  agent: AgentName,
  prompt: string,
  permissionMode: PermissionMode | undefined,
): string {
  const definition = AGENT_REGISTRY[agent];
  const permFlags = getPermissionFlags(agent, permissionMode);
  const baseCommand = appendFlags(definition.promptCommand, permFlags);
  const escapedPrompt = escapeShellArg(prompt);

  if (definition.promptTransport === 'send-keys') {
    // For send-keys agents, start without prompt and pipe it after startup
    return appendFlags(
      definition.noPromptCommand || definition.promptCommand,
      permFlags,
    );
  }

  if (definition.promptTransport === 'stdin') {
    return `printf '%s\\n' ${escapedPrompt} | ${baseCommand}`;
  }

  if (definition.promptTransport === 'option' && definition.promptOption) {
    return `${baseCommand} ${definition.promptOption} ${escapedPrompt}`;
  }

  // positional
  return `${baseCommand} ${escapedPrompt}`;
}

// ── Startup script + launch ───────────────────────────────────────────────

export interface BuildStartupScriptOpts {
  slug: string;
  agent: AgentName;
  prompt: string;
  worktreePath: string;
  permissionMode?: PermissionMode;
  statusFilePath: string;
}

/**
 * Build a shell script that:
 * 1. cd to the worktree
 * 2. Runs the agent with the prompt
 * 3. On exit, writes a "done" status JSON file
 *
 * Returns the path to the written temp script.
 */
export function buildStartupScript(
  opts: BuildStartupScriptOpts,
): string {
  const { slug, agent, prompt, worktreePath, permissionMode, statusFilePath } =
    opts;
  const definition = AGENT_REGISTRY[agent];
  const permFlags = getPermissionFlags(agent, permissionMode);
  const baseCommand = appendFlags(definition.promptCommand, permFlags);
  const escapedPrompt = escapeShellArg(prompt);

  let agentInvocation: string;

  if (definition.promptTransport === 'send-keys') {
    const readyDelay = definition.sendKeysReadyDelayMs ?? 0;
    const noPromptCmd = appendFlags(
      definition.noPromptCommand || definition.promptCommand,
      permFlags,
    );
    // Start agent, wait for ready, then feed prompt via stdin
    agentInvocation = [
      `${noPromptCmd} &`,
      `AGENT_PID=$!`,
      `sleep ${(readyDelay / 1000).toFixed(1)}`,
      // Send pre-prompt keys if any
      ...(definition.sendKeysPrePrompt ?? []).map(
        () => `# pre-prompt key (send-keys transport)`,
      ),
      `printf '%s\\n' ${escapedPrompt} > /proc/$AGENT_PID/fd/0 2>/dev/null || true`,
      `wait $AGENT_PID`,
    ].join('\n');
  } else if (definition.promptTransport === 'stdin') {
    agentInvocation = `printf '%s\\n' ${escapedPrompt} | ${baseCommand}`;
  } else if (definition.promptTransport === 'option' && definition.promptOption) {
    agentInvocation = `${baseCommand} ${definition.promptOption} ${escapedPrompt}`;
  } else {
    // positional
    agentInvocation = `${baseCommand} ${escapedPrompt}`;
  }

  const scriptContent = `#!/usr/bin/env bash
set -euo pipefail

SLUG=${escapeShellArg(slug)}
STATUS_FILE=${escapeShellArg(statusFilePath)}

# Navigate to worktree
cd ${escapeShellArg(worktreePath)}

# Write "working" status
mkdir -p "$(dirname "$STATUS_FILE")"
printf '{"status":"working","slug":"%s","startedAt":"%s"}\\n' "$SLUG" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$STATUS_FILE"

# Run agent
${agentInvocation}
EXIT_CODE=$?

# Write final status
if [ "$EXIT_CODE" -eq 0 ]; then
  printf '{"status":"done","slug":"%s","exitCode":0,"finishedAt":"%s"}\\n' "$SLUG" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$STATUS_FILE"
else
  printf '{"status":"error","slug":"%s","exitCode":%d,"finishedAt":"%s"}\\n' "$SLUG" "$EXIT_CODE" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$STATUS_FILE"
fi
`;

  const scriptPath = `/tmp/dellij-${slug}.sh`;
  writeFileSync(scriptPath, scriptContent, { mode: 0o755 });
  return scriptPath;
}

export interface LaunchAgentOpts {
  slug: string;
  agent: AgentName;
  prompt: string;
  worktreePath: string;
  permissionMode?: PermissionMode;
  dellijDir: string;
  controlTabName: string;
}

/**
 * Launch an agent in a new zellij tab, wrapping it in the startup script.
 */
export async function launchAgentInNewTab(
  opts: LaunchAgentOpts,
): Promise<void> {
  const {
    slug,
    agent,
    prompt,
    worktreePath,
    permissionMode,
    dellijDir,
    controlTabName,
  } = opts;

  const statusFilePath = join(dellijDir, 'status', `${slug}.json`);
  const scriptPath = buildStartupScript({
    slug,
    agent,
    prompt,
    worktreePath,
    permissionMode,
    statusFilePath,
  });

  const zellijService = ZellijService.getInstance();
  
  const layoutContent = generateAgentLayoutKdl({
    name: slug,
    worktreePath,
    command: scriptPath,
  });
  const layoutPath = writeLayoutFile(layoutContent);
  
  await zellijService.newTabWithLayout(layoutPath);
  await zellijService.goToTab(controlTabName);
}

