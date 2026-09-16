#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
# Zii (字) — Modern Wayland Photo Viewer & Editor for Omarchy Linux
# Installer & Updater Script
# Repository: https://github.com/mirarr-app/zii
# ==============================================================================

REPO="mirarr-app/zii"
MODE="local"       # local, release, prerelease, source
USE_USER=true
PREFIX="$HOME/.local"
MAKE_DEFAULT=false

CLEANUP_DIRS=()
cleanup_temp_dirs() {
    for dir in "${CLEANUP_DIRS[@]}"; do
        if [ -d "$dir" ]; then
            rm -rf "$dir"
        fi
    done
}
trap cleanup_temp_dirs EXIT

# Detect if piped from curl or run directly outside of a repo/archive
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd || echo "")"
if [ -z "$SCRIPT_DIR" ] || [ ! -f "$SCRIPT_DIR/zii.desktop" ]; then
    MODE="release"
fi

for arg in "$@"; do
    case "$arg" in
        --source)
            MODE="source"
            ;;
        --release)
            MODE="release"
            ;;
        --prerelease|--pre-release|--pre)
            MODE="prerelease"
            ;;
        --system)
            USE_USER=false
            PREFIX="/usr/local"
            ;;
        --user)
            USE_USER=true
            PREFIX="$HOME/.local"
            ;;
        --prefix=*)
            PREFIX="${arg#*=}"
            ;;
        --make-default)
            MAKE_DEFAULT=true
            ;;
        -h|--help)
            echo "Zii (字) Installer & Updater"
            echo ""
            echo "Usage: ./install.sh [OPTIONS]"
            echo "   or: curl -fsSL https://raw.githubusercontent.com/mirarr-app/zii/main/install.sh | bash -s -- [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --release       Download and install the latest stable binary release [default over curl]"
            echo "  --prerelease    Download and install the latest pre-release binary"
            echo "  --source        Build and install directly from latest source on GitHub"
            echo "  --user          Install to ~/.local without root privileges [default]"
            echo "  --system        Install system-wide to /usr/local (requires sudo)"
            echo "  --prefix=DIR    Install to custom destination prefix"
            echo "  --make-default  Set Zii as the default application for all supported image formats"
            echo "  -h, --help      Show this help message"
            exit 0
            ;;
    esac
done

if [ "$USE_USER" = false ] && [ "$PREFIX" = "$HOME/.local" ]; then
    PREFIX="/usr/local"
fi

# Check root permissions if installing system-wide or to a prefix not under $HOME
if [ "$USE_USER" = false ] || [ -z "$HOME" ] || [[ "$PREFIX" != "$HOME"/* && "$PREFIX" != "$HOME" ]]; then
    if [ "$(id -u)" -ne 0 ]; then
        echo "==> Error: Installing to $PREFIX requires root privileges." >&2
        echo "    Please rerun with: sudo ./install.sh --system" >&2
        exit 1
    fi
fi

# ------------------------------------------------------------------------------
# Dependency Checker
# ------------------------------------------------------------------------------
check_runtime_deps() {
    if ! command -v quickshell >/dev/null 2>&1; then
        echo "==> Warning: 'quickshell' is not installed or not in PATH."
        echo "    Zii requires Quickshell for its Wayland UI."
        echo "    On Omarchy / Arch Linux, install it with:"
        echo "        sudo pacman -S quickshell"
        echo ""
    fi
}

check_build_deps() {
    local missing=()
    if ! command -v cargo >/dev/null 2>&1; then missing+=("rust"); fi
    if ! command -v git >/dev/null 2>&1; then missing+=("git"); fi
    if ! command -v quickshell >/dev/null 2>&1; then missing+=("quickshell"); fi

    if [ ${#missing[@]} -gt 0 ]; then
        echo "==> Error: Missing build dependencies to compile Zii from source: ${missing[*]}"
        echo "    Install them on Omarchy / Arch Linux via:"
        echo "        sudo pacman -S --needed ${missing[*]}"
        exit 1
    fi
}

# ------------------------------------------------------------------------------
# Default Application Association
# ------------------------------------------------------------------------------
set_default_image_viewer() {
    local apps_dir="$1"
    local src_dir="$2"

    echo "==> Configuring Zii as default viewer for all supported image formats..."

    local desktop_file=""
    if [ -f "$apps_dir/zii.desktop" ]; then
        desktop_file="$apps_dir/zii.desktop"
    elif [ -f "$src_dir/zii.desktop" ]; then
        desktop_file="$src_dir/zii.desktop"
    fi

    local mimes=()
    if [ -n "$desktop_file" ] && grep -q "^MimeType=" "$desktop_file"; then
        local raw_line
        raw_line="$(grep "^MimeType=" "$desktop_file" | head -n 1 | cut -d= -f2-)"
        local IFS=';'
        read -r -a mimes <<< "$raw_line"
    fi

    if [ ${#mimes[@]} -eq 0 ]; then
        mimes=(
            "image/jpeg" "image/jpg" "image/pjpeg"
            "image/png" "image/x-png" "image/vnd.mozilla.apng"
            "image/webp" "image/gif"
            "image/bmp" "image/x-bmp" "image/x-ms-bmp"
            "image/tiff"
            "image/svg+xml" "image/svg+xml-compressed"
            "image/x-icon" "image/vnd.microsoft.icon"
        )
    fi

    local count=0
    for mime in "${mimes[@]}"; do
        [ -z "$mime" ] && continue
        if command -v xdg-mime >/dev/null 2>&1; then
            xdg-mime default zii.desktop "$mime" 2>/dev/null || true
        fi
        if command -v gio >/dev/null 2>&1; then
            gio mime "$mime" zii.desktop >/dev/null 2>&1 || true
        fi
        count=$((count + 1))
    done

    if [ -n "${SUDO_USER:-}" ] && [ "$SUDO_USER" != "root" ]; then
        for mime in "${mimes[@]}"; do
            [ -z "$mime" ] && continue
            su - "$SUDO_USER" -c "command -v xdg-mime >/dev/null 2>&1 && xdg-mime default zii.desktop '$mime' 2>/dev/null; command -v gio >/dev/null 2>&1 && gio mime '$mime' zii.desktop 2>/dev/null" 2>/dev/null || true
        done
    fi

    if command -v xdg-mime >/dev/null 2>&1 || command -v gio >/dev/null 2>&1; then
        echo "  -> Associated $count image MIME types with zii.desktop"
    else
        echo "==> Warning: neither 'xdg-mime' nor 'gio' found to set default applications."
    fi
}

# ------------------------------------------------------------------------------
# Core File Installer
# ------------------------------------------------------------------------------
install_files() {
    local src_dir="$1"
    local bin_dir="$PREFIX/bin"
    local share_dir="$PREFIX/share/zii"
    local apps_dir="$PREFIX/share/applications"
    local icons_dir="$PREFIX/share/icons/hicolor"
    local pixmaps_dir="$PREFIX/share/pixmaps"

    echo "==> Installing Zii into $PREFIX..."
    mkdir -p "$bin_dir" "$share_dir" "$apps_dir" "$pixmaps_dir"

    # 1. Binary
    local bin_src=""
    if [ -f "$src_dir/zii" ]; then
        bin_src="$src_dir/zii"
    elif [ -f "$src_dir/target/release/zii" ]; then
        bin_src="$src_dir/target/release/zii"
    elif [ -f "$src_dir/bin/zii" ]; then
        bin_src="$src_dir/bin/zii"
    else
        echo "Error: zii binary not found in $src_dir." >&2
        exit 1
    fi

    install -m 755 "$bin_src" "$bin_dir/zii"
    echo "  -> Installed binary: $bin_dir/zii"

    # 2. UI files
    if [ -d "$src_dir/ui" ]; then
        rm -rf "$share_dir/ui"
        cp -r "$src_dir/ui" "$share_dir/ui"
        echo "  -> Installed UI assets: $share_dir/ui"
    fi

    # 3. Desktop entry
    if [ -f "$src_dir/zii.desktop" ]; then
        install -m 644 "$src_dir/zii.desktop" "$apps_dir/zii.desktop"
        echo "  -> Installed desktop entry: $apps_dir/zii.desktop"
        if command -v update-desktop-database >/dev/null 2>&1; then
            update-desktop-database "$apps_dir" 2>/dev/null || true
        fi
    fi

    # 4. Icons
    local asset_dir="$src_dir/assets"
    if [ ! -d "$asset_dir" ] && [ -d "$src_dir/ui/assets" ]; then
        asset_dir="$src_dir/ui/assets"
    fi

    if [ -f "$asset_dir/zii.svg" ]; then
        mkdir -p "$icons_dir/scalable/apps"
        install -m 644 "$asset_dir/zii.svg" "$icons_dir/scalable/apps/zii.svg"
        install -m 644 "$asset_dir/zii.svg" "$pixmaps_dir/zii.svg"
        echo "  -> Installed scalable icon: $icons_dir/scalable/apps/zii.svg"
    fi

    for sz in 32 48 64 128 256 512; do
        local png_src=""
        if [ "$sz" = "512" ] && [ -f "$asset_dir/zii.png" ]; then
            png_src="$asset_dir/zii.png"
        elif [ -f "$asset_dir/zii-${sz}.png" ]; then
            png_src="$asset_dir/zii-${sz}.png"
        fi
        if [ -n "$png_src" ]; then
            mkdir -p "$icons_dir/${sz}x${sz}/apps"
            install -m 644 "$png_src" "$icons_dir/${sz}x${sz}/apps/zii.png"
        fi
    done

    if [ -f "$asset_dir/zii.png" ]; then
        install -m 644 "$asset_dir/zii.png" "$pixmaps_dir/zii.png"
    fi

    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        gtk-update-icon-cache -f -t "$icons_dir" 2>/dev/null || true
    fi

    # 5. Default application configuration
    if [ "$MAKE_DEFAULT" = true ]; then
        set_default_image_viewer "$apps_dir" "$src_dir"
    fi

    check_runtime_deps

    echo "==> Successfully installed Zii!"
    echo "    Run 'zii' in terminal or launch it from your Omarchy application menu."
}

# ------------------------------------------------------------------------------
# Remote GitHub Release Fetcher
# ------------------------------------------------------------------------------
install_from_github() {
    local is_prerelease="$1"
    local temp_dir
    temp_dir="$(mktemp -d -t zii_install_XXXXXX)"
    CLEANUP_DIRS+=("$temp_dir")

    echo "==> Fetching latest $([ "$is_prerelease" = true ] && echo "pre-release" || echo "release") for Omarchy Linux..."

    local api_url
    if [ "$is_prerelease" = true ]; then
        api_url="https://api.github.com/repos/$REPO/releases"
    else
        api_url="https://api.github.com/repos/$REPO/releases/latest"
    fi

    local releases_json
    releases_json="$(curl -fsSL "$api_url" || echo "")"

    if [ -z "$releases_json" ]; then
        echo "==> Error: Could not query GitHub releases API for $REPO."
        echo "    Check your internet connection or GitHub status."
        exit 1
    fi

    local download_url=""
    local tag_name=""

    if [ "$is_prerelease" = true ]; then
        # Find first release with prerelease == true
        download_url="$(echo "$releases_json" | grep -B 25 -A 25 '"prerelease": true' | grep -o 'https://github.com/'"$REPO"'/releases/download/[^"]*x86_64-unknown-linux-gnu.tar.gz' | head -n 1 || echo "")"
        tag_name="$(echo "$releases_json" | grep -B 10 -A 10 '"prerelease": true' | grep '"tag_name":' | head -n 1 | sed -E 's/.*"([^"]+)".*/\1/' || echo "")"
    else
        download_url="$(echo "$releases_json" | grep -o 'https://github.com/'"$REPO"'/releases/download/[^"]*x86_64-unknown-linux-gnu.tar.gz' | head -n 1 || echo "")"
        tag_name="$(echo "$releases_json" | grep '"tag_name":' | head -n 1 | sed -E 's/.*"([^"]+)".*/\1/' || echo "")"
    fi

    if [ -z "$download_url" ]; then
        echo "==> Warning: No prebuilt binary archive found in $([ "$is_prerelease" = true ] && echo "pre-releases" || echo "releases")."
        echo "    Falling back to building directly from git source..."
        install_from_source
        return
    fi

    echo "==> Downloading Zii ${tag_name:-} from $download_url..."
    curl -fsSL "$download_url" -o "$temp_dir/zii.tar.gz"

    echo "==> Extracting archive..."
    tar -xzf "$temp_dir/zii.tar.gz" -C "$temp_dir"

    # Find the extracted folder
    local extracted_dir
    extracted_dir="$(find "$temp_dir" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
    if [ -z "$extracted_dir" ]; then
        extracted_dir="$temp_dir"
    fi

    install_files "$extracted_dir"
}

# ------------------------------------------------------------------------------
# Build From Source
# ------------------------------------------------------------------------------
install_from_source() {
    check_build_deps

    local temp_dir
    temp_dir="$(mktemp -d -t zii_source_XXXXXX)"
    CLEANUP_DIRS+=("$temp_dir")

    echo "==> Cloning Zii from https://github.com/$REPO.git..."
    git clone --depth 1 "https://github.com/$REPO.git" "$temp_dir"

    echo "==> Building Zii with Cargo (release mode)..."
    (
        cd "$temp_dir"
        cargo build --release
    )

    install_files "$temp_dir"
}

# ------------------------------------------------------------------------------
# Execution Flow
# ------------------------------------------------------------------------------
case "$MODE" in
    source)
        install_from_source
        ;;
    prerelease)
        install_from_github true
        ;;
    release)
        install_from_github false
        ;;
    local)
        install_files "$SCRIPT_DIR"
        ;;
esac
