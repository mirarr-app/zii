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
    PREFIX="$HOME/.local"
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

# Install icons
ICONS_DIR="$PREFIX/share/icons/hicolor"
PIXMAPS_DIR="$PREFIX/share/pixmaps"
mkdir -p "$PIXMAPS_DIR"

if [ -f "$SCRIPT_DIR/assets/zii.svg" ]; then
    mkdir -p "$ICONS_DIR/scalable/apps"
    install -m 644 "$SCRIPT_DIR/assets/zii.svg" "$ICONS_DIR/scalable/apps/zii.svg"
    install -m 644 "$SCRIPT_DIR/assets/zii.svg" "$PIXMAPS_DIR/zii.svg"
    echo "  Installed scalable icon -> $ICONS_DIR/scalable/apps/zii.svg"
fi

for sz in 32 48 64 128 256 512; do
    PNG_SRC=""
    if [ "$sz" = "512" ] && [ -f "$SCRIPT_DIR/assets/zii.png" ]; then
        PNG_SRC="$SCRIPT_DIR/assets/zii.png"
    elif [ -f "$SCRIPT_DIR/assets/zii-${sz}.png" ]; then
        PNG_SRC="$SCRIPT_DIR/assets/zii-${sz}.png"
    fi
    if [ -n "$PNG_SRC" ]; then
        mkdir -p "$ICONS_DIR/${sz}x${sz}/apps"
        install -m 644 "$PNG_SRC" "$ICONS_DIR/${sz}x${sz}/apps/zii.png"
    fi
done

if [ -f "$SCRIPT_DIR/assets/zii.png" ]; then
    install -m 644 "$SCRIPT_DIR/assets/zii.png" "$PIXMAPS_DIR/zii.png"
    echo "  Installed pixmap -> $PIXMAPS_DIR/zii.png"
fi

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -f -t "$ICONS_DIR" 2>/dev/null || true
fi

echo "==> Done! Zii is installed."
echo "    Run 'zii' in terminal or launch from your app menu."
