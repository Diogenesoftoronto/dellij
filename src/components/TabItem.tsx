import React from 'react';
import { Box, Text } from 'ink';
import type { DellijTab, AgentStatus } from '../types.ts';
import { AGENT_REGISTRY, isAgentName } from '../utils/agentLaunch.ts';

interface TabItemProps {
  tab: DellijTab;
  isSelected: boolean;
  statusOverride?: AgentStatus;
}

function statusColor(status: AgentStatus | undefined): string {
  switch (status) {
    case 'working':
      return 'yellow';
    case 'waiting':
      return 'cyan';
    case 'analyzing':
      return 'blue';
    case 'error':
      return 'red';
    case 'done':
      return 'green';
    case 'idle':
    default:
      return 'green';
  }
}

function statusLabel(status: AgentStatus | undefined): string {
  switch (status) {
    case 'working':
      return 'working';
    case 'waiting':
      return 'waiting';
    case 'analyzing':
      return 'analyzing';
    case 'error':
      return 'error';
    case 'done':
      return 'done';
    case 'idle':
    default:
      return 'idle';
  }
}

export function TabItem({
  tab,
  isSelected,
  statusOverride,
}: TabItemProps): React.JSX.Element {
  const agentLabel =
    tab.agent && isAgentName(tab.agent) && AGENT_REGISTRY[tab.agent]
      ? AGENT_REGISTRY[tab.agent].shortLabel
      : tab.type === 'shell'
        ? 'sh'
        : '??';

  const effectiveStatus = statusOverride ?? tab.agentStatus ?? 'idle';
  const color = statusColor(effectiveStatus);
  const label = statusLabel(effectiveStatus);

  // Truncate slug if too long
  const maxSlugLen = 22;
  const slug =
    tab.slug.length > maxSlugLen
      ? tab.slug.slice(0, maxSlugLen - 1) + '…'
      : tab.slug;

  return (
    <Box>
      <Text color={isSelected ? 'cyan' : undefined} bold={isSelected}>
        {isSelected ? '> ' : '  '}
        {'['}
        {agentLabel}
        {'] '}
        {slug}
      </Text>
      <Text> </Text>
      <Text color={color}>{'\u25cf'}</Text>
      <Text color={color}>{label}</Text>
    </Box>
  );
}
