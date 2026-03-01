#!/bin/bash

set -e

echo "Installing envtui..."

if ! command -v cargo &> /dev/null; then
    echo "Error: Cargo is not installed."
    echo "Please install Rust: https://rustup.rs/"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "Building envtui..."
cargo build --release

BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"

if [ -f "$SCRIPT_DIR/target/release/envtui" ]; then
    cp "$SCRIPT_DIR/target/release/envtui" "$BIN_DIR/envtui"
    echo "Installed envtui to $BIN_DIR/envtui"
    
    if ! grep -q "$BIN_DIR" <<< "$PATH"; then
        echo ""
        echo "Add the following to your shell profile (~/.zshrc or ~/.bashrc):"
        echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
    fi
else
    echo "Error: Build failed"
    exit 1
fi

echo "Done!"
