# <img src="assets/zii.svg" width="36" height="36" align="center" alt="Zii Logo" /> Zii (字) — Photo Viewer & Editor for Omarchy Linux

**Zii** is a high-performance, keyboard-driven photo viewer and non-destructive image editor designed specifically for **Omarchy Linux**. Built with a multithreaded **Rust** core and a fluid **Quickshell** (Qt QML) frontend, it features Vim-style modal navigation and editing, instant trash undo, and real-time Omarchy theme hot-reloading.

---

## Installation & Updating (Omarchy)

### 1. Latest Stable Release (Prebuilt Binary)
Install the latest stable release to `~/.local/bin` and `~/.local/share` (running this command again automatically updates to the newest release):
```bash
curl -fsSL https://raw.githubusercontent.com/mirarr-app/zii/main/install.sh | bash
```

### 2. Latest Pre-release (Bleeding Edge Binary)
Install or update to the latest pre-release build:
```bash
curl -fsSL https://raw.githubusercontent.com/mirarr-app/zii/main/install.sh | bash -s -- --prerelease
```

### 3. Build & Install from Source
#### Prerequisites for building from source on Omarchy:
Ensure the Rust toolchain, Git, and Quickshell are installed:
```bash
sudo pacman -S --needed rust git quickshell
```
Then run the one-liner source installer (re-running it pulls the latest commit, rebuilds, and updates your installation):
```bash
curl -fsSL https://raw.githubusercontent.com/mirarr-app/zii/main/install.sh | bash -s -- --source
```

---

## Features

- **Live Omarchy Theming**: Automatically syncs with `~/.local/state/omarchy/current/theme/colors.toml` in real time with zero restart required, supporting both canonical Quattro semantic roles and legacy theme keys.
- **EXIF Auto-Orientation & Inspector (`e` / `x`)**: Automatically applies digital camera and smartphone EXIF orientation tags. Press `e` or `x` for a sleek metadata inspector showing camera model, lens, exposure, aperture, ISO, and focal length.
- **Wayland Clipboard & Wallpaper**: Press `y` to yank the photo directly to the Wayland clipboard (`wl-copy`), `Y` to copy the file path, or `W` to set the image as the Omarchy desktop wallpaper (`omarchy-theme-bg-set`).
- **Animated GIF & WebP Playback (`Space`)**: Fluid animated playback with pause, frame counter, and looping.
- **Crisp Pixel Zoom**: Automatic nearest-neighbor filtering above 150% zoom so pixel art, logos, and high-magnification details stay sharp instead of blurry.
- **Live Directory Watcher**: Detects newly captured screenshots, downloaded images, or files deleted in file managers (Yazi, Nautilus) in real-time.
- **Vim-Modal Keybindings**:
  - `h` / `l` / Arrow keys for browsing directory images.
  - `j` / `k` for panning when zoomed in (clamped to window boundaries).
  - `+` / `-` / `z` / `Z` / mouse wheel for smooth focal zooming; `0` to fit to window; `1` for 1:1 original scale.
  - `dd` moves photo directly to FreeDesktop Trash; `u` restores it instantly; `Shift+D` permanently deletes.
  - `f` / `F11` for fullscreen toggle.
  - `i` enters **Edit Mode**.
  - `q` / `Esc` quits.
- **Non-Destructive Image Editor (`i`)**:
  - **Crop (`c`)**: Interactive 8-handle crop overlay with rule-of-thirds grid and aspect ratio presets (`Free`, `1:1`, `16:9`, `4:3`, `3:2`, `9:16`). Drag handles with mouse or use arrow keys (move) and `Shift`+arrows (resize), then press `Enter` to apply.
  - **Rotate (`r` / `R`)**: 90° clockwise and counter-clockwise.
  - **Flip (`h` / `v`)**: Horizontal and vertical flipping.
  - **Adjustments (`a`)**: Live Brightness, Contrast, and Saturation sliders with keyboard brackets (`[` / `]`) and reset button.
  - **Undo / Redo (`u` / `Ctrl+r`)**: Full multi-step undo/redo stack. Viewport pan and zoom are smoothly preserved across edit operations.
  - **Atomic & High-Quality Saving (`w` / `s`)**: `w` overwrites original directly with atomic rename safety and 95% high-quality JPEG encoding; `s` opens quick choice to overwrite or save as a new copy (e.g. `_edited_1.png`).

---

## Keybindings Quick Reference

### Normal (Viewing) Mode

| Shortcut | Description |
|----------|-------------|
| `h` / `Left` | Previous image in directory |
| `l` / `Right` | Next image in directory |
| `j` / `Down` | Pan down (when zoomed) |
| `k` / `Up` | Pan up (when zoomed) |
| `+` / `=` / `z` | Zoom in |
| `-` / `_` / `Z` | Zoom out |
| `0` | Fit to window |
| `1` | 100% (1:1 original pixel scale) |
| `Scroll Wheel` | Zoom toward cursor (crisp nearest-neighbor above 150%) |
| `Double Click` | Toggle fit / 100% zoom |
| `Left Mouse Drag` | Pan image smoothly (clamped to viewport) |
| `Space` | Play / pause animated GIF & WebP playback |
| `y` | Copy image to Wayland clipboard (`wl-copy`) |
| `Y` (Shift+y) | Copy absolute file path to clipboard |
| `W` (Shift+w) | Set as Omarchy desktop wallpaper (`omarchy-theme-bg-set`) |
| `e` / `x` | Toggle detailed EXIF metadata inspector modal |
| `f` / `F11` | Fullscreen toggle |
| `dd` | Move image to Trash (FreeDesktop trash) |
| `Shift+D` | Permanently delete file |
| `u` | Undo trash (restore last deleted photo) |
| `?` | Toggle keyboard shortcuts help |
| `i` | **Enter Edit Mode** |
| `q` / `Esc` | Quit Zii |

### Edit Mode (`-- EDIT --`)

| Shortcut | Description |
|----------|-------------|
| `Esc` | Exit Edit Mode (or cancel active crop / tool) |
| `c` | Toggle interactive Crop tool |
| `0` (in crop) | Free aspect ratio |
| `1` (in crop) | 1:1 Square aspect ratio |
| `2` (in crop) | 16:9 aspect ratio |
| `3` (in crop) | 4:3 aspect ratio |
| `4` (in crop) | 3:2 aspect ratio |
| `Arrow Keys` (in crop) | Move crop selection box |
| `Shift + Arrow Keys` | Resize crop selection box (preserves aspect ratio) |
| `Enter` (in crop) | Apply crop |
| `r` | Rotate 90° Clockwise |
| `R` (Shift+r) | Rotate 90° Counter-Clockwise |
| `h` | Flip Horizontal |
| `v` | Flip Vertical |
| `a` | Toggle Adjustments panel (Brightness, Contrast, Saturation) |
| `[` / `]` | Decrease / increase adjustment value |
| `u` | Undo edit operation |
| `Ctrl+r` | Redo edit operation |
| `w` | Overwrite original file directly (95% high-quality JPEG) |
| `s` | Open save options (Overwrite or Save New Copy) |

---

## Architecture

```
zii [IMAGE_OR_FOLDER]
       |
       +---> [Rust Backend] <======== JSON-RPC IPC ========> [Quickshell Frontend]
             - Image Pipeline         (Unix Domain Socket)   - FloatingWindow / Viewport
             - Directory Scanner                             - Interactive Crop Overlay
             - Multi-step Editor                             - HUD & Adjustments Panel
             - FreeDesktop Trash                             - Modal KeyHandler
             - Omarchy Theme Watcher                         - Live colors.toml sync
```

---

## Building & Running

### Prerequisites

Ensure you are running Omarchy Linux with Hyprland and have the following packages:
- `cargo` (Rust toolchain)
- `quickshell` (Quickshell Qt6 QML runner)

### Build

```bash
# Debug build
cargo build

# Release build
cargo build --release
```

### Run

```bash
# View current directory
./target/release/zii

# Open a specific image or folder
./target/release/zii ~/Pictures/photo.jpg
./target/release/zii ~/Pictures/Wallpapers/
```

### Installation

You can install Zii system-wide or to your user profile using the included `install.sh` script:

```bash
# Build release binary
cargo build --release

# Install for current user (~/.local/bin, ~/.local/share/zii, ~/.local/share/applications)
./install.sh --user

# Or install system-wide (requires sudo)
sudo ./install.sh
```

---

## Releases & CI/CD

Automated builds and releases are managed via GitHub Actions:
- **Releases (`.github/workflows/release.yml`)**: Triggered when pushing stable version tags (e.g. `git tag v0.1.0 && git push --tags`). Builds the release package, runs test suites, creates tarballs (`zii-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz`) with SHA256 checksums, and publishes full GitHub Releases.
- **Pre-releases (`.github/workflows/prerelease.yml`)**: Triggered when pushing pre-release tags (e.g. `v0.1.0-beta.1`, `v0.1.0-rc1`) or dispatched manually. Publishes pre-releases marked with `prerelease: true`.
