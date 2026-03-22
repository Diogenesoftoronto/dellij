import React from 'react';
import { Box, Text } from 'ink';
import type { DellijTab, AgentStatus } from '../types.ts';
import { TabItem } from './TabItem.tsx';

interface SidebarProps {
  tabs: DellijTab[];
  selectedIndex: number;
  statusMap: Record<string, AgentStatus>;
  terminalWidth?: number;
}

export function Sidebar({
  tabs,
  selectedIndex,
  statusMap,
  terminalWidth,
}: SidebarProps): React.JSX.Element {
  return (
    <Box flexDirection="column" flexGrow={1}>
      <Box marginBottom={1}>
        <Text bold color="cyan">
          {'Agents ('}
          {tabs.length}
          {')'}
        </Text>
      </Box>

      {tabs.length === 0 ? (
        <Box paddingX={2}>
          <Text dimColor>No agents running.</Text>
        </Box>
      ) : (
        tabs.map((tab, idx) => (
          <TabItem
            key={tab.id}
            tab={tab}
            isSelected={idx === selectedIndex}
            statusOverride={statusMap[tab.slug]}
            terminalWidth={terminalWidth}
          />
        ))
      )}
    </Box>
  );
}
