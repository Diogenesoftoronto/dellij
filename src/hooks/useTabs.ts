import { useState, useEffect, useCallback } from 'react';
import { watch } from 'chokidar';
import { join } from 'path';
import { existsSync } from 'fs';
import type { DellijConfig, DellijTab } from '../types.ts';
import {
  loadConfig,
  saveConfig,
  addTab as addTabToConfig,
  removeTab as removeTabFromConfig,
  updateTab as updateTabInConfig,
} from '../utils/config.ts';

interface UseTabsResult {
  config: DellijConfig;
  tabs: DellijTab[];
  addTab: (tab: DellijTab) => void;
  removeTab: (tabId: string) => void;
  updateTab: (tabId: string, updates: Partial<DellijTab>) => void;
}

export function useTabs(dellijDir: string, initialConfig: DellijConfig): UseTabsResult {
  const [config, setConfig] = useState<DellijConfig>(initialConfig);

  // Watch the config file for external changes
  useEffect(() => {
    const configFile = join(dellijDir, 'dellij.config.json');
    if (!existsSync(configFile)) return;

    const watcher = watch(configFile, {
      persistent: true,
      ignoreInitial: true,
      awaitWriteFinish: { stabilityThreshold: 100, pollInterval: 50 },
    });

    watcher.on('change', () => {
      try {
        const updated = loadConfig(dellijDir);
        setConfig(updated);
      } catch {
        // If file is temporarily unreadable (mid-write), ignore
      }
    });

    return () => {
      watcher.close().catch(() => {});
    };
  }, [dellijDir]);

  const addTab = useCallback(
    (tab: DellijTab) => {
      setConfig((prev: import("../types.ts").DellijConfig) => {
        const next = addTabToConfig(prev, tab);
        saveConfig(dellijDir, next);
        return next;
      });
    },
    [dellijDir],
  );

  const removeTab = useCallback(
    (tabId: string) => {
      setConfig((prev: import("../types.ts").DellijConfig) => {
        const next = removeTabFromConfig(prev, tabId);
        saveConfig(dellijDir, next);
        return next;
      });
    },
    [dellijDir],
  );

  const updateTab = useCallback(
    (tabId: string, updates: Partial<DellijTab>) => {
      setConfig((prev: import("../types.ts").DellijConfig) => {
        const next = updateTabInConfig(prev, tabId, updates);
        saveConfig(dellijDir, next);
        return next;
      });
    },
    [dellijDir],
  );

  return {
    config,
    tabs: config.tabs,
    addTab,
    removeTab,
    updateTab,
  };
}
