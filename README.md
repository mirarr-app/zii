# Zii (字 / 視) — Photo Viewer & Editor for Omarchy Linux

**Zii** is a high-performance, keyboard-driven photo viewer and non-destructive image editor designed specifically for **Omarchy Linux**. Built with a multithreaded **Rust** core and a fluid **Quickshell** (Qt QML) frontend, it features Vim-style modal navigation and editing, instant trash undo, and real-time Omarchy theme hot-reloading.

---

## Features

- ⚡ **Blazing Performance**: Native Rust image processing engine handling JPEG, PNG, WebP, GIF, BMP, TIFF, SVG, and ICO.
- 🎨 **Live Omarchy Theming**: Automatically syncs with `~/.local/state/omarchy/current/theme/colors.toml` in real time with zero restart required.
- ⌨️ **Vim-Modal Keybindings**:
  - `h` / `l` / Arrow keys for browsing directory images.
  - `j` / `k` for panning when zoomed in.
  - `+` / `-` / `z` / `Z` / mouse wheel for smooth focal zooming; `0` to fit to window; `1` for 1:1 original scale.
  - `dd` moves photo directly to FreeDesktop Trash; `u` restores it instantly; `Shift+D` permanently deletes.
  - `f` / `F11` for fullscreen toggle.
  - `i` enters **Edit Mode**.
  - `q` / `Esc` quits.
- ✂️ **Non-Destructive Image Editor (`i`)**:
  - **Crop (`c`)**: Interactive 8-handle crop overlay with dark scrim and rule-of-thirds grid. Drag handles with mouse or use arrow keys (move) and `Shift`+arrows (resize), then press `Enter` to apply.
  - **Rotate (`r` / `R`)**: 90° clockwise and counter-clockwise.
  - **Flip (`h` / `v`)**: Horizontal and vertical flipping.
  - **Adjustments (`a`)**: Live Brightness and Contrast sliders with keyboard brackets (`[` / `]`) and reset button.
  - **Undo / Redo (`u` / `Ctrl+r`)**: Full multi-step undo/redo stack.
  - **Atomic Saving (`w` / `s`)**: `w` overwrites original directly with atomic rename safety; `s` opens quick choice to overwrite or save as a new copy (e.g. `_edited_1.png`).
- 🕶️ **Minimalist Auto-Hiding HUD**: Floating translucent status pill showing mode, filename, resolution, filesize, zoom %, and `[index/total]`. Automatically fades out during viewing and wakes on mouse or key input.

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
| `Scroll Wheel` | Zoom toward cursor |
| `Double Click` | Toggle fit / 100% zoom |
| `Left Mouse Drag` | Pan image smoothly |
| `f` / `F11` | Fullscreen toggle |
| `dd` | Move image to Trash (FreeDesktop trash) |
| `Shift+D` | Permanently delete file |
| `u` | Undo trash (restore last deleted photo) |
| `i` | **Enter Edit Mode** |
| `q` / `Esc` | Quit Zii |

### Edit Mode (`-- EDIT --`)

| Shortcut | Description |
|----------|-------------|
| `Esc` | Exit Edit Mode (or cancel active crop / tool) |
| `c` | Toggle interactive Crop tool |
| `Arrow Keys` (in crop) | Move crop selection box |
| `Shift + Arrow Keys` | Resize crop selection box |
| `Enter` (in crop) | Apply crop |
| `r` | Rotate 90° Clockwise |
| `R` (Shift+r) | Rotate 90° Counter-Clockwise |
| `h` | Flip Horizontal |
| `v` | Flip Vertical |
| `a` | Toggle Adjustments panel (Brightness & Contrast) |
| `[` / `]` | Decrease / increase adjustment value |
| `u` | Undo edit operation |
| `Ctrl+r` | Redo edit operation |
| `w` | Overwrite original file directly |
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

### Install into User PATH

```bash
# Symlink or copy binary
mkdir -p ~/.local/bin ~/.local/share/applications
ln -sf $(pwd)/target/release/zii ~/.local/bin/zii
cp zii.desktop ~/.local/share/applications/
```
