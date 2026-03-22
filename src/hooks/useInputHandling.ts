import { useState, useCallback } from 'react';
import { useInput } from 'ink';
import type { AgentName } from '../utils/agentLaunch.ts';

export type ModalState =
  | 'none'
  | 'promptInput'
  | 'agentSelect'
  | 'help'
  | 'confirmClose'
  | 'confirmMerge';

interface UseInputHandlingOpts {
  tabCount: number;
  selectedIndex: number;
  setSelectedIndex: (index: number) => void;
  onNavigateToTab: (index: number) => void;
  onCloseTab: (index: number) => void;
  onMergeTab: (index: number) => void;
  onNewShellTab: () => void;
  onQuit: () => void;
  onNewAgent: (prompt: string, agent: AgentName) => void;
}

interface UseInputHandlingResult {
  modal: ModalState;
  setModal: (modal: ModalState) => void;
  pendingPrompt: string;
  setPendingPrompt: (prompt: string) => void;
}

export function useInputHandling(
  opts: UseInputHandlingOpts,
): UseInputHandlingResult {
  const {
    tabCount,
    selectedIndex,
    setSelectedIndex,
    onNavigateToTab,
    onCloseTab,
    onMergeTab,
    onNewShellTab,
    onQuit,
    onNewAgent,
  } = opts;

  const [modal, setModal] = useState<ModalState>('none');
  const [pendingPrompt, setPendingPrompt] = useState('');

  useInput(
    (input, key) => {
      if (modal !== 'none') return;

      if (key.upArrow || input === 'k') {
        setSelectedIndex(Math.max(0, selectedIndex - 1));
        return;
      }

      if (key.downArrow || input === 'j') {
        setSelectedIndex(Math.min(tabCount - 1, selectedIndex + 1));
        return;
      }

      if (key.return || input === 'l') {
        if (tabCount > 0) {
          onNavigateToTab(selectedIndex);
        }
        return;
      }

      if (input === 'n') {
        setModal('promptInput');
        return;
      }

      if (input === 'x') {
        if (tabCount > 0) {
          setModal('confirmClose');
        }
        return;
      }

      if (input === 'm') {
        if (tabCount > 0) {
          setModal('confirmMerge');
        }
        return;
      }

      if (input === 's') {
        onNewShellTab();
        return;
      }

      if (input === '?') {
        setModal('help');
        return;
      }

      if (input === 'q') {
        onQuit();
        return;
      }
    },
    { isActive: modal === 'none' },
  );

  return {
    modal,
    setModal,
    pendingPrompt,
    setPendingPrompt,
  };
}
