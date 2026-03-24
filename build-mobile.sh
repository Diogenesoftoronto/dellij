#!/usr/bin/env bash
set -euo pipefail

# Dellij Mobile Build Script (Android)
# Requires: cargo-apk (cargo install cargo-apk)

echo "🚀 Building dellij-mobile for Android..."

# Ensure we are in the project root
cd "$(dirname "$0")"

# Build the APK in release mode
cargo apk build --release -p dellij-mobile

echo "✅ Build complete!"
echo "📍 APK Location: target/release/apk/dellij-mobile.apk"
echo ""
echo "To install on a connected device:"
echo "  adb install target/release/apk/dellij-mobile.apk"
