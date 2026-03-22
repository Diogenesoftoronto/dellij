# VHS tape for dellij demo
Output dellij/vhs/demo.gif

Set Shell "bash"
Set FontSize 18
Set Width 1000
Set Height 600
Set Padding 20
Set Theme "Catppuccin Mocha"

# Start dellij in UI mode
Type "cd /home/diogenes/Projects/dellij && /home/diogenes/.bun/bin/bun run src/index.ts --ui --project-root /tmp/dellij-demo" Enter
Sleep 3s

# Open "New Agent" prompt
Type "n"
Sleep 1s
Type "implement user auth"
Sleep 2s
Enter
Sleep 1s

# Select Claude agent
Type "j"
Sleep 1s
Type "k"
Sleep 1s
Enter
Sleep 4s

# Open help
Type "?"
Sleep 3s
Type "q"
Sleep 1s

# Clean up
Hide
Type "rm -rf /tmp/dellij-demo" Enter
Type "exit" Enter
