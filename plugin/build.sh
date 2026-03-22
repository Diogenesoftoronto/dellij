#!/usr/bin/env bash
# Build the dellij-status zellij plugin
# Requires: Rust toolchain with wasm32-wasip1 target
# Install target: rustup target add wasm32-wasip1

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Adding wasm32-wasip1 target..."
rustup target add wasm32-wasip1

echo "Building dellij-status plugin..."
cargo build --release --target wasm32-wasip1

WASM_PATH="$SCRIPT_DIR/target/wasm32-wasip1/release/dellij_status.wasm"

if [ -f "$WASM_PATH" ]; then
  echo ""
  echo "Plugin built successfully: $WASM_PATH"
  echo ""
  echo "To install globally:"
  echo "  mkdir -p ~/.config/zellij/plugins"
  echo "  cp '$WASM_PATH' ~/.config/zellij/plugins/dellij_status.wasm"
  echo ""
  echo "dellij will automatically use the plugin from the project directory."
else
  echo "ERROR: Build succeeded but WASM file not found at expected path."
  exit 1
fi
