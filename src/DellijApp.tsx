import React, { useState } from 'react';
import { Box, Text, useApp, useInput } from 'ink';
import type { DellijConfig, DellijTab } from './types.ts';
import type { AgentName } from './utils/agentLaunch.ts';
import { Sidebar } from './components/Sidebar.tsx';
import { PromptPopup } from './components/popups/PromptPopup.tsx';
import { AgentSelectPopup } from './components/popups/AgentSelectPopup.tsx';
import { useTabs } from './hooks/useTabs.ts';
import { useAgentStatus } from './hooks/useAgentStatus.ts';
import { ZellijService } from './services/ZellijService.ts';
import { resolveEnabledAgents, launchAgentInNewTab } from './utils/agentLaunch.ts';
import { createWorktree, getBaseBranch, generateSlug } from './utils/git.ts';
import { HookManager } from './services/HookManager.ts';

// Re-export from git to avoid circular imports
function makeId(): string {
  return Math.random().toString(36).slice(2, 10);
}

type Modal =
  | 'none'
  | 'promptInput'
  | 'agentSelect'
  | 'help'
  | 'confirmClose'
  | 'confirmMerge';

interface DellijAppProps {
  config: DellijConfig;
  dellijDir: string;
  sessionName: string;
  controlTabName: string;
}

export function DellijApp({
  config: initialConfig,
  dellijDir,
  sessionName,
  controlTabName,
}: DellijAppProps): React.JSX.Element {
  const { exit } = useApp();
  const { config, tabs, addTab, removeTab, updateTab } = useTabs(
    dellijDir,
    initialConfig,
  );
  const statusMap = useAgentStatus(dellijDir);
  const zellijService = ZellijService.getInstance();

  const [selectedIndex, setSelectedIndex] = useState(0);
  const [modal, setModal] = useState<Modal>('none');
  const [pendingPrompt, setPendingPrompt] = useState('');
  const [statusMessage, setStatusMessage] = useState('');

  const enabledAgents = resolveEnabledAgents(config.settings.enabledAgents);

  function showStatus(msg: string): void {
    setStatusMessage(msg);
    setTimeout(() => setStatusMessage(''), 3000);
  }

  // ── Keyboard input ──────────────────────────────────────────────────────

  useInput(
    (input, key) => {
      if (modal !== 'none') return;

      if (key.upArrow || input === 'k') {
        setSelectedIndex((prev) => Math.max(0, prev - 1));
        return;
      }

      if (key.downArrow || input === 'j') {
        setSelectedIndex((prev) => Math.min(tabs.length - 1, prev + 1));
        return;
      }

      if (key.return) {
        if (tabs.length > 0) {
          const tab = tabs[selectedIndex];
          if (tab) {
            zellijService.goToTab(tab.slug).catch(() => {});
          }
        }
        return;
      }

      if (input === 'n') {
        setModal('promptInput');
        return;
      }

      if (input === 'x') {
        if (tabs.length > 0) {
          setModal('confirmClose');
        }
        return;
      }

      if (input === 'm') {
        if (tabs.length > 0) {
          setModal('confirmMerge');
        }
        return;
      }

      if (input === 's') {
        handleNewShellTab();
        return;
      }

      if (input === '?') {
        setModal('help');
        return;
      }

      if (input === 'q') {
        exit();
        return;
      }
    },
    { isActive: modal === 'none' },
  );

  // ── Confirm close modal input ───────────────────────────────────────────

  useInput(
    (input, key) => {
      if (modal !== 'confirmClose') return;
      if (key.escape || input === 'n') {
        setModal('none');
        return;
      }
      if (input === 'y' || key.return) {
        handleCloseTab();
        setModal('none');
      }
    },
    { isActive: modal === 'confirmClose' },
  );

  // ── Confirm merge modal input ───────────────────────────────────────────

  useInput(
    (input, key) => {
      if (modal !== 'confirmMerge') return;
      if (key.escape || input === 'n') {
        setModal('none');
        return;
      }
      if (input === 'y' || key.return) {
        handleMergeTab();
        setModal('none');
      }
    },
    { isActive: modal === 'confirmMerge' },
  );

  // ── Help modal input ────────────────────────────────────────────────────

  useInput(
    (_input, key) => {
      if (modal !== 'help') return;
      if (key.escape || key.return) {
        setModal('none');
      }
    },
    { isActive: modal === 'help' },
  );

  // ── Action handlers ─────────────────────────────────────────────────────

  function handlePromptSubmit(prompt: string): void {
    setPendingPrompt(prompt);
    setModal('agentSelect');
  }

  async function handleAgentSelect(agent: AgentName): Promise<void> {
    setModal('none');
    showStatus(`Creating ${agent} agent for: ${pendingPrompt.slice(0, 40)}…`);

    try {
      const slug = generateSlug(pendingPrompt, agent);
      const baseBranch =
        config.settings.baseBranch ??
        (await getBaseBranch(config.projectRoot));

      const worktreePath = await createWorktree({
        projectRoot: config.projectRoot,
        slug,
        baseBranch,
      });

      const tab: DellijTab = {
        id: makeId(),
        slug,
        prompt: pendingPrompt,
        agent,
        agentStatus: 'working',
        worktreePath,
        branchName: slug,
        projectRoot: config.projectRoot,
        createdAt: new Date().toISOString(),
        type: 'agent',
      };

      addTab(tab);

      // Fire worktree_created hook
      HookManager.getInstance(config.projectRoot, dellijDir).runHook('worktree_created', {
        DELLIJ_ROOT: config.projectRoot,
        DELLIJ_SLUG: slug,
        DELLIJ_AGENT: agent,
        DELLIJ_WORKTREE_PATH: worktreePath,
        DELLIJ_BRANCH: slug,
        DELLIJ_PROMPT: pendingPrompt,
      });

      await launchAgentInNewTab({
        slug,
        agent,
        prompt: pendingPrompt,
        worktreePath,
        permissionMode: config.settings.permissionMode,
        dellijDir,
        controlTabName,
      });

      showStatus(`Agent tab created: ${slug}`);
    } catch (err: unknown) {
      showStatus(
        `Error: ${err instanceof Error ? err.message : String(err)}`,
      );
    }

    setPendingPrompt('');
  }

  async function handleNewShellTab(): Promise<void> {
    const slug = `shell-${makeId()}`;
    showStatus(`Opening shell tab: ${slug}`);

    try {
      await zellijService.newTab(slug, config.projectRoot);

      const tab: DellijTab = {
        id: makeId(),
        slug,
        prompt: '',
        type: 'shell',
        createdAt: new Date().toISOString(),
        projectRoot: config.projectRoot,
      };

      addTab(tab);

      await zellijService.goToTab(slug);
    } catch (err: unknown) {
      showStatus(
        `Error: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  }

  function handleCloseTab(): void {
    const tab = tabs[selectedIndex];
    if (!tab) return;

    HookManager.getInstance(config.projectRoot, dellijDir).runHook('before_pane_close', {
      DELLIJ_ROOT: config.projectRoot,
      DELLIJ_SLUG: tab.slug,
      DELLIJ_AGENT: tab.agent ?? '',
      DELLIJ_WORKTREE_PATH: tab.worktreePath ?? '',
      DELLIJ_BRANCH: tab.branchName ?? '',
    });

    removeTab(tab.id);
    setSelectedIndex((prev) => Math.max(0, prev - 1));
    showStatus(`Closed tab: ${tab.slug}`);

    HookManager.getInstance(config.projectRoot, dellijDir).runHook('pane_closed', {
      DELLIJ_ROOT: config.projectRoot,
      DELLIJ_SLUG: tab.slug,
      DELLIJ_BRANCH: tab.branchName ?? '',
    });
  }

  async function handleMergeTab(): Promise<void> {
    const tab = tabs[selectedIndex];
    if (!tab || !tab.worktreePath) {
      showStatus('No worktree to merge');
      return;
    }

    showStatus(`Merging ${tab.slug}…`);

    HookManager.getInstance(config.projectRoot, dellijDir).runHook('pre_merge', {
      DELLIJ_ROOT: config.projectRoot,
      DELLIJ_SLUG: tab.slug,
      DELLIJ_WORKTREE_PATH: tab.worktreePath,
      DELLIJ_BRANCH: tab.branchName ?? tab.slug,
    });

    try {
      const { mergeWorktree } = await import('./utils/git.ts');
      const result = await mergeWorktree({
        worktreePath: tab.worktreePath,
        slug: tab.slug,
        targetBranch:
          config.settings.baseBranch ??
          (await getBaseBranch(config.projectRoot)),
        projectRoot: config.projectRoot,
      });

      if (result.success) {
        showStatus(`Merged ${tab.slug} successfully`);
        updateTab(tab.id, { agentStatus: 'done' });

        HookManager.getInstance(config.projectRoot, dellijDir).runHook('post_merge', {
          DELLIJ_ROOT: config.projectRoot,
          DELLIJ_SLUG: tab.slug,
          DELLIJ_BRANCH: tab.branchName ?? tab.slug,
        });
      } else if (result.conflicts) {
        showStatus(`Merge conflicts in ${tab.slug} — resolve manually`);
      } else {
        showStatus(`Merge failed for ${tab.slug}`);
      }
    } catch (err: unknown) {
      showStatus(
        `Merge error: ${err instanceof Error ? err.message : String(err)}`,
      );
    }
  }

  // ── Render ──────────────────────────────────────────────────────────────

  const selectedTab = tabs[selectedIndex];

  return (
    <Box flexDirection="column" width={50}>
      {/* Header */}
      <Box borderStyle="single" borderColor="cyan" paddingX={1}>
        <Text bold color="cyan">
          dellij
        </Text>
        <Text> v1.0.0  </Text>
        <Text bold>{config.projectName}</Text>
      </Box>

      {/* Sidebar */}
      <Box
        flexDirection="column"
        borderStyle="single"
        borderColor="gray"
        paddingX={1}
        flexGrow={1}
      >
        <Sidebar
          tabs={tabs}
          selectedIndex={selectedIndex}
          statusMap={statusMap}
        />
      </Box>

      {/* Status message */}
      {statusMessage ? (
        <Box paddingX={1}>
          <Text color="yellow">{statusMessage}</Text>
        </Box>
      ) : null}

      {/* Footer */}
      <Box borderStyle="single" borderColor="gray" paddingX={1} flexDirection="column">
        <Text>
          <Text color="cyan">[n]</Text>
          <Text>ew  </Text>
          <Text color="cyan">[↵]</Text>
          <Text>go to  </Text>
          <Text color="cyan">[m]</Text>
          <Text>erge  </Text>
          <Text color="cyan">[x]</Text>
          <Text>close</Text>
        </Text>
        <Text>
          <Text color="cyan">[s]</Text>
          <Text>hell  </Text>
          <Text color="cyan">[?]</Text>
          <Text>help  </Text>
          <Text color="cyan">[q]</Text>
          <Text>uit</Text>
        </Text>
      </Box>

      {/* Modals */}
      {modal === 'promptInput' && (
        <Box position="absolute" marginLeft={2} marginTop={3}>
          <PromptPopup
            onSubmit={handlePromptSubmit}
            onCancel={() => setModal('none')}
          />
        </Box>
      )}

      {modal === 'agentSelect' && (
        <Box position="absolute" marginLeft={2} marginTop={3}>
          <AgentSelectPopup
            availableAgents={enabledAgents}
            onSelect={handleAgentSelect}
            onCancel={() => setModal('none')}
          />
        </Box>
      )}

      {modal === 'confirmClose' && selectedTab && (
        <Box
          position="absolute"
          marginLeft={2}
          marginTop={3}
          flexDirection="column"
          borderStyle="round"
          borderColor="red"
          paddingX={2}
          paddingY={1}
        >
          <Text bold color="red">
            Close tab?
          </Text>
          <Text>
            Close <Text bold>{selectedTab.slug}</Text>? [y/N]
          </Text>
        </Box>
      )}

      {modal === 'confirmMerge' && selectedTab && (
        <Box
          position="absolute"
          marginLeft={2}
          marginTop={3}
          flexDirection="column"
          borderStyle="round"
          borderColor="yellow"
          paddingX={2}
          paddingY={1}
        >
          <Text bold color="yellow">
            Merge worktree?
          </Text>
          <Text>
            Merge <Text bold>{selectedTab.slug}</Text> into base branch? [y/N]
          </Text>
        </Box>
      )}

      {modal === 'help' && (
        <Box
          position="absolute"
          marginLeft={2}
          marginTop={3}
          flexDirection="column"
          borderStyle="round"
          borderColor="cyan"
          paddingX={2}
          paddingY={1}
        >
          <Text bold color="cyan">
            dellij Help
          </Text>
          <Text> </Text>
          <Text><Text color="cyan">n</Text>       New agent tab</Text>
          <Text><Text color="cyan">Enter</Text>   Navigate to selected tab</Text>
          <Text><Text color="cyan">↑/↓ k/j</Text> Navigate list</Text>
          <Text><Text color="cyan">x</Text>       Close selected tab</Text>
          <Text><Text color="cyan">m</Text>       Merge selected worktree</Text>
          <Text><Text color="cyan">s</Text>       New shell tab</Text>
          <Text><Text color="cyan">q</Text>       Quit dellij TUI</Text>
          <Text><Text color="cyan">?</Text>       Show this help</Text>
          <Text> </Text>
          <Text dimColor>[Enter/Esc] close help</Text>
        </Box>
      )}
    </Box>
  );
}
