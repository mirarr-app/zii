# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Adjustment panel keyboard cycling and stepping using `Tab` / `Shift+Tab` and `Up` / `Down` arrows.
- Fallback search for UI QML files in standard `XDG_DATA_DIRS` and relative to binary prefix (`../share/zii/ui/shell.qml`).
- Dynamic derivation of `surfaceElevated` in theme application when `lighter_bg` is provided.
- Comprehensive integration tests for IPC server ready, directory state, edit session, and busy state handling.
- Standalone `parse_colors` function in `theme` module with tests covering short and legacy theme key resolution.

### Changed
- Moved `screenshot.png` into `assets/screenshot.png`.
- Updated GitHub Actions pre-release and release runners from `ubuntu-22.04` to `ubuntu-24.04`.
- Standardized `install.sh` to check for root privileges when installing to system-wide paths and ensure temporary directories are reliably cleaned up on exit.

### Fixed
- Preserved original EXIF metadata during JPEG save operations while neutralizing the orientation tag to normal (1) to prevent double rotation.
- Fixed normalization in `SaveDialog.qml` and `CropOverlay.qml` where `Qt.rgba` calls used `255` instead of `1.0`.
- Handled SIGTERM, SIGHUP, and SIGINT gracefully with unified resource and temporary file cleanup.
- Prevented viewport binding conflicts where loaded image dimensions overwrote natural dimensions.
- Corrected safe UTF-8 boundary checks in EXIF date formatting to prevent panics on malformed strings.
- Atomic fallback and error handling for trash restore operations.

### Performance
- Fast preview generation with Triangle downscaling for images larger than 4096px edge and fast PNG compression with `BufWriter`.
- 768 MB byte budget limit for undo/redo stacks to prevent out-of-memory errors with large images.
- Cached directory image probe stats (modification time, file size) to avoid redundant image decoding on rescan.
- Offloaded CPU-intensive image operations (crop, rotate, flip, adjust, resize, undo, redo, save, open) and subprocesses (wallpaper, clipboard, exif) off the tokio async reactor and `AppState` lock using `tokio::task::spawn_blocking`.
- Atomic `editor_busy` state guard to protect against concurrent overlapping image edits.

### Security
- Set strict `0600` file permissions on the IPC Unix domain socket.
- Added timeouts to external subprocess calls for clipboard and wallpaper commands.
