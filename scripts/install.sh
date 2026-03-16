#!/usr/bin/env bash
# Shepherd install script — builds and installs both `shepherd` and `shep` binaries
set -euo pipefail

CARGO_BIN="${CARGO_HOME:-$HOME/.cargo}/bin"

echo "Building and installing Shepherd..."
cargo install --path "$(dirname "$0")/../crates/shepherd-cli"

# If cargo did not install `shep` (e.g. older Cargo), create a symlink
if [ ! -f "$CARGO_BIN/shep" ]; then
    echo "Creating shep symlink..."
    ln -sf "$CARGO_BIN/shepherd" "$CARGO_BIN/shep"
fi

echo ""
echo "Installed: shep ($(shep --version 2>/dev/null || shepherd --version))"
echo ""
echo "  shep                       — start server + open GUI"
echo "  shep status                — see all active tasks"
echo "  shep new 'fix the bug'     — spawn a new agent task"
echo "  shep approve --all         — approve all pending permissions"
echo "  shep pr <task-id>          — open a pull request"
