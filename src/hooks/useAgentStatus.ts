import { useState, useEffect } from 'react';
import { readFileSync, existsSync, readdirSync } from 'fs';
import { join } from 'path';
import type { AgentStatus } from '../types.ts';

interface StatusFileContent {
  status: AgentStatus;
  slug?: string;
  exitCode?: number;
  startedAt?: string;
  finishedAt?: string;
}

type StatusMap = Record<string, AgentStatus>;

function readStatusDir(statusDir: string): StatusMap {
  const result: StatusMap = {};
  if (!existsSync(statusDir)) return result;

  let files: string[];
  try {
    files = readdirSync(statusDir);
  } catch {
    return result;
  }

  for (const file of files) {
    if (!file.endsWith('.json')) continue;
    const slug = file.slice(0, -5); // remove .json
    try {
      const raw = readFileSync(join(statusDir, file), 'utf8');
      const parsed = JSON.parse(raw) as StatusFileContent;
      if (parsed.status) {
        result[slug] = parsed.status;
      }
    } catch {
      // Ignore unreadable or malformed status files
    }
  }

  return result;
}

/**
 * Polls .dellij/status/{slug}.json files every 2 seconds.
 * Returns a map of slug -> AgentStatus.
 */
export function useAgentStatus(dellijDir: string): StatusMap {
  const statusDir = join(dellijDir, 'status');
  const [statusMap, setStatusMap] = useState<StatusMap>(() =>
    readStatusDir(statusDir),
  );

  useEffect(() => {
    // Initial read
    setStatusMap(readStatusDir(statusDir));

    const interval = setInterval(() => {
      setStatusMap(readStatusDir(statusDir));
    }, 2000);

    return () => clearInterval(interval);
  }, [statusDir]);

  return statusMap;
}
