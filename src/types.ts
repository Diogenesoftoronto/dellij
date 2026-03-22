export type AgentStatus =
  | 'idle'
  | 'working'
  | 'waiting'
  | 'analyzing'
  | 'error'
  | 'done';

export type PermissionMode =
  | ''
  | 'plan'
  | 'acceptEdits'
  | 'bypassPermissions';

export interface DellijTab {
  id: string;
  slug: string;
  prompt: string;
  agent?: string;
  agentStatus?: AgentStatus;
  worktreePath?: string;
  branchName?: string;
  projectRoot?: string;
  createdAt: string;
  type: 'agent' | 'shell';
  pid?: number;
}

export interface DellijSettings {
  defaultAgent?: string;
  enabledAgents?: string[];
  permissionMode?: PermissionMode;
  baseBranch?: string;
  branchPrefix?: string;
}

export interface DellijConfig {
  projectName: string;
  projectRoot: string;
  tabs: DellijTab[];
  settings: DellijSettings;
  sessionName?: string;
  controlTabName?: string;
  lastUpdated?: string;
}

export interface HookContext {
  DELLIJ_ROOT: string;
  DELLIJ_SLUG: string;
  DELLIJ_AGENT?: string;
  DELLIJ_WORKTREE_PATH?: string;
  DELLIJ_BRANCH?: string;
  DELLIJ_PROMPT?: string;
  [key: string]: string | undefined;
}

export interface ZellijSessionInfo {
  name: string;
  isCurrentSession: boolean;
}
