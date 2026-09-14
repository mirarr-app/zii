#!/usr/bin/env bash
set -euo pipefail

PREFIX="${PREFIX:-/usr/local}"
USE_USER=false

for arg in "$@"; do
    case "$arg" in
        --user)
            USE_USER=true
            ;;
        --prefix=*)
            PREFIX="${arg#*=}"
            ;;
        -h|--help)
            echo "Usage: ./install.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --user         Install to ~/.local (no sudo required)"
            echo "  --prefix=DIR   Install to custom prefix [default: /usr/local]"
            echo "  -h, --help     Show this help message"
            exit 0
            ;;
    esac
done

if [ "$USE_USER" = true ]; then
    PREFIX="${XDG_DATA_HOME:-$HOME/.local}"
fi

BIN_DIR="$PREFIX/bin"
SHARE_DIR="$PREFIX/share/zii"
APPS_DIR="$PREFIX/share/applications"

echo "==> Installing Zii into $PREFIX..."
mkdir -p "$BIN_DIR" "$SHARE_DIR" "$APPS_DIR"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Find binary (current dir, target/release, or bin)
if [ -f "$SCRIPT_DIR/zii" ]; then
    BIN_SRC="$SCRIPT_DIR/zii"
elif [ -f "$SCRIPT_DIR/target/release/zii" ]; then
    BIN_SRC="$SCRIPT_DIR/target/release/zii"
elif [ -f "$SCRIPT_DIR/bin/zii" ]; then
    BIN_SRC="$SCRIPT_DIR/bin/zii"
else
    echo "Error: zii binary not found. Run 'cargo build --release' first." >&2
    exit 1
fi

install -m 755 "$BIN_SRC" "$BIN_DIR/zii"
echo "  Installed binary -> $BIN_DIR/zii"

# Install UI files
if [ -d "$SCRIPT_DIR/ui" ]; then
    rm -rf "$SHARE_DIR/ui"
    cp -r "$SCRIPT_DIR/ui" "$SHARE_DIR/ui"
    echo "  Installed UI -> $SHARE_DIR/ui"
fi

# Install desktop entry
if [ -f "$SCRIPT_DIR/zii.desktop" ]; then
    install -m 644 "$SCRIPT_DIR/zii.desktop" "$APPS_DIR/zii.desktop"
    echo "  Installed desktop entry -> $APPS_DIR/zii.desktop"
    if command -v update-desktop-database >/dev/null 2>&1; then
        update-desktop-database "$APPS_DIR" 2>/dev/null || true
    fi
fi

echo "==> Done! Zii is installed."
echo "    Run 'zii' in terminal or launch from your app menu."
