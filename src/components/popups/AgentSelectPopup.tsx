import React, { useState } from 'react';
import { Box, Text, useInput } from 'ink';
import type { AgentName } from '../../utils/agentLaunch.ts';
import { AGENT_REGISTRY } from '../../utils/agentLaunch.ts';

interface AgentSelectPopupProps {
  availableAgents: AgentName[];
  onSelect: (agent: AgentName) => void;
  onCancel: () => void;
}

export function AgentSelectPopup({
  availableAgents,
  onSelect,
  onCancel,
}: AgentSelectPopupProps): React.JSX.Element {
  const [selectedIdx, setSelectedIdx] = useState(0);

  useInput((input, key) => {
    if (key.escape) {
      onCancel();
      return;
    }

    if (key.upArrow || input === 'k') {
      setSelectedIdx((prev) => Math.max(0, prev - 1));
      return;
    }

    if (key.downArrow || input === 'j') {
      setSelectedIdx((prev) => Math.min(availableAgents.length - 1, prev + 1));
      return;
    }

    if (key.return) {
      const agent = availableAgents[selectedIdx];
      if (agent) {
        onSelect(agent);
      }
      return;
    }
  });

  return (
    <Box
      flexDirection="column"
      borderStyle="round"
      borderColor="cyan"
      paddingX={2}
      paddingY={1}
      width={50}
    >
      <Box marginBottom={1}>
        <Text bold color="cyan">
          Select Agent
        </Text>
      </Box>
      {availableAgents.map((agent, idx) => {
        const entry = AGENT_REGISTRY[agent];
        const isSelected = idx === selectedIdx;
        return (
          <Box key={agent}>
            <Text
              color={isSelected ? 'cyan' : undefined}
              bold={isSelected}
            >
              {isSelected ? '> ' : '  '}
              {'['}
              {entry.shortLabel}
              {'] '}
              {entry.name}
              <Text dimColor>{' — '}{entry.description}</Text>
            </Text>
          </Box>
        );
      })}
      <Box marginTop={1}>
        <Text dimColor>[↑↓] navigate  [Enter] select  [Esc] cancel</Text>
      </Box>
    </Box>
  );
}
