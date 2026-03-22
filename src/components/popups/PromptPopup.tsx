import React, { useState } from 'react';
import { Box, Text, useInput } from 'ink';
import TextInput from 'ink-text-input';

interface PromptPopupProps {
  onSubmit: (text: string) => void;
  onCancel: () => void;
}

export function PromptPopup({ onSubmit, onCancel }: PromptPopupProps): React.JSX.Element {
  const [value, setValue] = useState('');

  useInput((input, key) => {
    if (key.escape) {
      onCancel();
    }
  });

  function handleSubmit(text: string): void {
    const trimmed = text.trim();
    if (trimmed.length > 0) {
      onSubmit(trimmed);
    }
  }

  return (
    <Box
      flexDirection="column"
      borderStyle="round"
      borderColor="cyan"
      paddingX={2}
      paddingY={1}
      width={60}
    >
      <Box marginBottom={1}>
        <Text bold color="cyan">
          New Agent Tab
        </Text>
      </Box>
      <Box marginBottom={1}>
        <Text dimColor>Describe the task for the agent:</Text>
      </Box>
      <Box>
        <Text color="green">{'> '}</Text>
        <TextInput
          value={value}
          onChange={setValue}
          onSubmit={handleSubmit}
          placeholder="e.g. fix the authentication bug in login.ts"
        />
      </Box>
      <Box marginTop={1}>
        <Text dimColor>[Enter] confirm  [Esc] cancel</Text>
      </Box>
    </Box>
  );
}
