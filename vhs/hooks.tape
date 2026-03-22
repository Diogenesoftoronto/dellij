# VHS tape for dellij hooks demo
Output vhs/hooks.gif

Set Shell "bash"
Set FontSize 18
Set Width 1000
Set Height 600
Set Padding 20
Set Theme "Catppuccin Mocha"

# Start dellij in UI mode
Type "cd /home/diogenes/Projects/dellij && /home/diogenes/.bun/bin/bun run src/index.ts --ui --project-root /tmp/dellij-hooks-demo" Enter
Sleep 2s

# Launch an agent to trigger the hook
Type "n"
Sleep 1s
Type "test the hook system"
Sleep 1s
Enter
Sleep 1s
Enter
Sleep 2s

# Show the status message
Type "?"
Sleep 2s
Type "q"
Sleep 1s

# Clean up
Hide
Type "exit" Enter
