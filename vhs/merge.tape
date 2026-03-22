# VHS tape for dellij merge demo
Output vhs/merge.gif

Set Shell "bash"
Set FontSize 18
Set Width 1000
Set Height 600
Set Padding 20
Set Theme "Catppuccin Mocha"

# Start dellij in UI mode
Type "cd /home/diogenes/Projects/dellij && /home/diogenes/.bun/bin/bun run src/index.ts --ui --project-root /tmp/dellij-merge-demo" Enter
Sleep 2s

# Select the tab to merge
Type "j"
Sleep 1s

# Trigger merge
Type "m"
Sleep 2s

# Confirm merge
Type "y"
Sleep 3s

# Show help and quit
Type "?"
Sleep 2s
Type "q"
Sleep 1s

# Clean up
Hide
Type "exit" Enter
