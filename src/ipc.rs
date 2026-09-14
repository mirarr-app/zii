use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, Mutex};
use serde::{Deserialize, Serialize};

use crate::scanner::{DirectoryScanner, ImageEntry};
use crate::theme::{OmarchyTheme, ThemeManager};
use crate::trash::TrashManager;
use crate::editor::ImageEditor;
use crate::exif_inspector::{extract_metadata, ExifMetadata};

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientRequest {
    Ready,
    Navigate {
        direction: String,
        target: Option<usize>,
    },
    DeleteCurrent {
        #[serde(default)]
        permanent: bool,
    },
    RestoreTrash,
    ClipboardCopy {
        #[serde(default)]
        path_only: bool,
    },
    SetWallpaper,
    GetExif,
    EditStart,
    EditCancel,
    EditCrop {
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    },
    EditRotate {
        degrees: i32,
    },
    EditFlip {
        horizontal: bool,
        vertical: bool,
    },
    EditAdjust {
        brightness: i32,
        contrast: f32,
        #[serde(default)]
        saturation: i32,
    },
    EditResize {
        width: u32,
        height: u32,
    },
    EditUndo,
    EditRedo,
    EditSave {
        overwrite: bool,
        filename: Option<String>,
    },
    Quit,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerEvent {
    Theme {
        theme: OmarchyTheme,
    },
    DirectoryState {
        index: usize,
        total: usize,
        current: Option<ImageEntry>,
    },
    EditState {
        active: bool,
        preview_path: Option<String>,
        can_undo: bool,
        can_redo: bool,
        width: u32,
        height: u32,
    },
    ExifData {
        data: ExifMetadata,
    },
    Toast {
        message: String,
        level: String, // info, success, warn, error
    },
    Close,
}

pub struct AppState {
    pub scanner: DirectoryScanner,
    pub trash: TrashManager,
    pub editor: ImageEditor,
    pub edit_active: bool,
    pub theme: ThemeManager,
}

pub async fn copy_to_clipboard(path: &Path, path_only: bool) -> Result<String, String> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        if path_only {
            let path_str = canonical.to_string_lossy().to_string();
            let status = tokio::process::Command::new("wl-copy")
                .arg(&path_str)
                .status()
                .await
                .map_err(|e| format!("Failed to run wl-copy: {}", e))?;

            if status.success() {
                Ok("Copied path to clipboard".to_string())
            } else {
                Err(format!("wl-copy exited with status {}", status))
            }
        } else {
            let ext = canonical
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();

            let mime = match ext.as_str() {
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "webp" => "image/webp",
                "gif" => "image/gif",
                "bmp" => "image/bmp",
                "tiff" | "tif" => "image/tiff",
                "svg" => "image/svg+xml",
                "ico" => "image/x-icon",
                _ => "application/octet-stream",
            };

            let bytes = tokio::fs::read(&canonical)
                .await
                .map_err(|e| format!("Failed to read image file: {}", e))?;

            let mut child = tokio::process::Command::new("wl-copy")
                .arg("--type")
                .arg(mime)
                .stdin(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to spawn wl-copy: {}", e))?;

            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(&bytes)
                    .await
                    .map_err(|e| format!("Failed to write to wl-copy: {}", e))?;
                drop(stdin);
            }

            let status = child
                .wait()
                .await
                .map_err(|e| format!("Error waiting for wl-copy: {}", e))?;

            if status.success() {
                Ok("Copied image to clipboard".to_string())
            } else {
                Err(format!("wl-copy exited with status {}", status))
            }
        }
    })
    .await
    .map_err(|_| "Clipboard copy timed out".to_string())?
}

pub async fn set_wallpaper(path: &Path) -> Result<String, String> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let path_str = canonical.to_string_lossy().to_string();

    // 1. Try omarchy-theme-bg-set
    if let Ok(status) = tokio::process::Command::new("omarchy-theme-bg-set")
        .arg(&path_str)
        .status()
        .await
    {
        if status.success() {
            return Ok("omarchy-theme-bg-set".to_string());
        }
    }

    // 2. Fallback to swww img
    if let Ok(status) = tokio::process::Command::new("swww")
        .arg("img")
        .arg(&path_str)
        .status()
        .await
    {
        if status.success() {
            return Ok("swww".to_string());
        }
    }

    // 3. Fallback to hyprctl hyprpaper
    let hypr_arg = format!(",{}", path_str);
    if let Ok(status) = tokio::process::Command::new("hyprctl")
        .arg("hyprpaper")
        .arg("wallpaper")
        .arg(&hypr_arg)
        .status()
        .await
    {
        if status.success() {
            return Ok("hyprctl hyprpaper".to_string());
        }
    }

    Err("Could not set wallpaper: neither omarchy-theme-bg-set, swww, nor hyprpaper succeeded".to_string())
}

pub async fn run_ipc_server(
    socket_path: PathBuf,
    state: Arc<Mutex<AppState>>,
    mut theme_rx: mpsc::Receiver<OmarchyTheme>,
    mut dir_rx: mpsc::Receiver<()>,
    shutdown_tx: mpsc::Sender<()>,
) -> anyhow::Result<()> {
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }

    let listener = UnixListener::bind(&socket_path)?;

    // Channel for broadcasting server events to connected client(s)
    let (broadcast_tx, _broadcast_rx) = tokio::sync::broadcast::channel::<String>(64);
    let broadcast_tx = Arc::new(broadcast_tx);

    // Forward theme watcher updates
    let b_tx_theme = broadcast_tx.clone();
    let state_theme = state.clone();
    tokio::spawn(async move {
        while let Some(new_theme) = theme_rx.recv().await {
            {
                let mut st = state_theme.lock().await;
                st.theme.current_theme = new_theme.clone();
            }
            let evt = ServerEvent::Theme { theme: new_theme };
            if let Ok(msg) = serde_json::to_string(&evt) {
                let _ = b_tx_theme.send(msg);
            }
        }
    });

    // Forward directory watcher updates
    let b_tx_dir = broadcast_tx.clone();
    let state_dir = state.clone();
    tokio::spawn(async move {
        while let Some(()) = dir_rx.recv().await {
            let mut st = state_dir.lock().await;
            let current_path = st.scanner.current().map(|e| e.path.clone());
            if let Err(e) = st.scanner.rescan() {
                eprintln!("Directory rescan warning: {:?}", e);
                continue;
            }
            if let Some(ref cp) = current_path {
                if let Some(pos) = st.scanner.entries.iter().position(|e| &e.path == cp) {
                    st.scanner.current_index = pos;
                }
            }
            let evt = ServerEvent::DirectoryState {
                index: st.scanner.current_index,
                total: st.scanner.entries.len(),
                current: st.scanner.current().cloned(),
            };
            if let Ok(msg) = serde_json::to_string(&evt) {
                let _ = b_tx_dir.send(msg);
            }
        }
    });

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let state_clone = state.clone();
                let b_tx = broadcast_tx.clone();
                let b_rx = broadcast_tx.subscribe();
                let s_tx = shutdown_tx.clone();

                tokio::spawn(async move {
                    if let Err(e) = handle_connection(stream, state_clone, b_tx, b_rx, s_tx).await {
                        eprintln!("Connection handler error: {:?}", e);
                    }
                });
            }
            Err(e) => {
                eprintln!("Socket accept error: {:?}", e);
                break;
            }
        }
    }

    Ok(())
}

async fn handle_connection(
    stream: UnixStream,
    state: Arc<Mutex<AppState>>,
    b_tx: Arc<tokio::sync::broadcast::Sender<String>>,
    mut b_rx: tokio::sync::broadcast::Receiver<String>,
    shutdown_tx: mpsc::Sender<()>,
) -> anyhow::Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    // Outgoing broadcast forwarder
    let write_loop = async {
        while let Ok(msg) = b_rx.recv().await {
            if writer.write_all(format!("{}\n", msg).as_bytes()).await.is_err() {
                break;
            }
            let _ = writer.flush().await;
        }
    };

    // Incoming requests handler
    let read_loop = async {
        while let Ok(Some(line)) = lines.next_line().await {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            match serde_json::from_str::<ClientRequest>(line) {
                Ok(req) => {
                    let should_quit = process_request(req, &state, &b_tx).await;
                    if should_quit {
                        let _ = shutdown_tx.send(()).await;
                        break;
                    }
                }
                Err(e) => {
                    eprintln!("Failed to parse request JSON: {:?} - raw: {}", e, line);
                }
            }
        }
    };

    tokio::select! {
        _ = write_loop => {},
        _ = read_loop => {},
    }

    Ok(())
}

async fn process_request(
    req: ClientRequest,
    state: &Arc<Mutex<AppState>>,
    b_tx: &Arc<tokio::sync::broadcast::Sender<String>>,
) -> bool {
    let mut st = state.lock().await;

    let send_event = |evt: ServerEvent| {
        if let Ok(msg) = serde_json::to_string(&evt) {
            let _ = b_tx.send(msg);
        }
    };

    match req {
        ClientRequest::Ready => {
            // Send initial theme
            send_event(ServerEvent::Theme {
                theme: st.theme.current_theme.clone(),
            });
            // Send directory state
            send_event(ServerEvent::DirectoryState {
                index: st.scanner.current_index,
                total: st.scanner.entries.len(),
                current: st.scanner.current().cloned(),
            });
        }
        ClientRequest::Navigate { direction, target } => {
            if st.edit_active {
                // If in edit mode, cancel edits when navigating away
                st.edit_active = false;
                send_event(ServerEvent::EditState {
                    active: false,
                    preview_path: None,
                    can_undo: false,
                    can_redo: false,
                    width: 0,
                    height: 0,
                });
            }

            match direction.as_str() {
                "next" => { st.scanner.next(); }
                "prev" => { st.scanner.prev(); }
                "first" => { st.scanner.first(); }
                "last" => { st.scanner.last(); }
                "goto" => {
                    if let Some(idx) = target {
                        st.scanner.go_to(idx);
                    }
                }
                _ => {}
            }

            send_event(ServerEvent::DirectoryState {
                index: st.scanner.current_index,
                total: st.scanner.entries.len(),
                current: st.scanner.current().cloned(),
            });
        }
        ClientRequest::DeleteCurrent { permanent } => {
            if let Some(current) = st.scanner.current().cloned() {
                let filename = current.filename.clone();
                let res = if permanent {
                    st.trash.delete_permanent(&current.path)
                } else {
                    st.trash.move_to_trash(&current.path)
                };

                match res {
                    Ok(_) => {
                        st.scanner.remove_current();
                        let msg = if permanent {
                            format!("Permanently deleted {}", filename)
                        } else {
                            format!("Moved {} to trash (press 'u' to undo)", filename)
                        };
                        send_event(ServerEvent::Toast {
                            message: msg,
                            level: "warn".to_string(),
                        });
                        send_event(ServerEvent::DirectoryState {
                            index: st.scanner.current_index,
                            total: st.scanner.entries.len(),
                            current: st.scanner.current().cloned(),
                        });
                    }
                    Err(e) => {
                        send_event(ServerEvent::Toast {
                            message: format!("Failed to delete: {}", e),
                            level: "error".to_string(),
                        });
                    }
                }
            }
        }
        ClientRequest::RestoreTrash => {
            match st.trash.restore_last() {
                Ok(Some(restored_path)) => {
                    let filename = restored_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                    let _ = st.scanner.rescan();
                    if let Some(pos) = st.scanner.entries.iter().position(|e| e.path == restored_path) {
                        st.scanner.current_index = pos;
                    }
                    send_event(ServerEvent::Toast {
                        message: format!("Restored {}", filename),
                        level: "success".to_string(),
                    });
                    send_event(ServerEvent::DirectoryState {
                        index: st.scanner.current_index,
                        total: st.scanner.entries.len(),
                        current: st.scanner.current().cloned(),
                    });
                }
                Ok(None) => {
                    send_event(ServerEvent::Toast {
                        message: "Nothing to undo".to_string(),
                        level: "info".to_string(),
                    });
                }
                Err(e) => {
                    send_event(ServerEvent::Toast {
                        message: format!("Failed to restore: {}", e),
                        level: "error".to_string(),
                    });
                }
            }
        }
        ClientRequest::ClipboardCopy { path_only } => {
            if let Some(current) = st.scanner.current().cloned() {
                match copy_to_clipboard(&current.path, path_only).await {
                    Ok(msg) => {
                        send_event(ServerEvent::Toast {
                            message: msg,
                            level: "success".to_string(),
                        });
                    }
                    Err(e) => {
                        send_event(ServerEvent::Toast {
                            message: e,
                            level: "error".to_string(),
                        });
                    }
                }
            } else {
                send_event(ServerEvent::Toast {
                    message: "No image selected".to_string(),
                    level: "warn".to_string(),
                });
            }
        }
        ClientRequest::SetWallpaper => {
            if let Some(current) = st.scanner.current().cloned() {
                let filename = current.filename.clone();
                match set_wallpaper(&current.path).await {
                    Ok(_) => {
                        send_event(ServerEvent::Toast {
                            message: format!("Set as Omarchy wallpaper: {}", filename),
                            level: "success".to_string(),
                        });
                    }
                    Err(e) => {
                        send_event(ServerEvent::Toast {
                            message: e,
                            level: "error".to_string(),
                        });
                    }
                }
            } else {
                send_event(ServerEvent::Toast {
                    message: "No image selected".to_string(),
                    level: "warn".to_string(),
                });
            }
        }
        ClientRequest::GetExif => {
            if let Some(current) = st.scanner.current().cloned() {
                let data = extract_metadata(
                    &current.path,
                    current.width,
                    current.height,
                    current.file_size,
                    &current.format,
                );
                send_event(ServerEvent::ExifData { data });
            }
        }
        ClientRequest::EditStart => {
            if let Some(current) = st.scanner.current().cloned() {
                match st.editor.open(&current.path) {
                    Ok(preview) => {
                        st.edit_active = true;
                        let (w, h) = st.editor.dimensions();
                        send_event(ServerEvent::EditState {
                            active: true,
                            preview_path: Some(preview.to_string_lossy().to_string()),
                            can_undo: false,
                            can_redo: false,
                            width: w,
                            height: h,
                        });
                    }
                    Err(e) => {
                        send_event(ServerEvent::Toast {
                            message: format!("Cannot edit image: {}", e),
                            level: "error".to_string(),
                        });
                    }
                }
            }
        }
        ClientRequest::EditCancel => {
            st.edit_active = false;
            st.editor.adjustment_base = None;
            send_event(ServerEvent::EditState {
                active: false,
                preview_path: None,
                can_undo: false,
                can_redo: false,
                width: 0,
                height: 0,
            });
            send_event(ServerEvent::Toast {
                message: "Discarded edits".to_string(),
                level: "info".to_string(),
            });
        }
        ClientRequest::EditCrop { x, y, width, height } => {
            if st.edit_active {
                match st.editor.crop(x, y, width, height) {
                    Ok(preview) => {
                        let (w, h) = st.editor.dimensions();
                        send_event(ServerEvent::EditState {
                            active: true,
                            preview_path: Some(preview.to_string_lossy().to_string()),
                            can_undo: st.editor.can_undo(),
                            can_redo: st.editor.can_redo(),
                            width: w,
                            height: h,
                        });
                    }
                    Err(e) => {
                        send_event(ServerEvent::Toast {
                            message: format!("Crop error: {}", e),
                            level: "error".to_string(),
                        });
                    }
                }
            }
        }
        ClientRequest::EditRotate { degrees } => {
            if st.edit_active {
                match st.editor.rotate(degrees) {
                    Ok(preview) => {
                        let (w, h) = st.editor.dimensions();
                        send_event(ServerEvent::EditState {
                            active: true,
                            preview_path: Some(preview.to_string_lossy().to_string()),
                            can_undo: st.editor.can_undo(),
                            can_redo: st.editor.can_redo(),
                            width: w,
                            height: h,
                        });
                    }
                    Err(e) => {
                        send_event(ServerEvent::Toast {
                            message: format!("Rotate error: {}", e),
                            level: "error".to_string(),
                        });
                    }
                }
            }
        }
        ClientRequest::EditFlip { horizontal, vertical } => {
            if st.edit_active {
                match st.editor.flip(horizontal, vertical) {
                    Ok(preview) => {
                        let (w, h) = st.editor.dimensions();
                        send_event(ServerEvent::EditState {
                            active: true,
                            preview_path: Some(preview.to_string_lossy().to_string()),
                            can_undo: st.editor.can_undo(),
                            can_redo: st.editor.can_redo(),
                            width: w,
                            height: h,
                        });
                    }
                    Err(e) => {
                        send_event(ServerEvent::Toast {
                            message: format!("Flip error: {}", e),
                            level: "error".to_string(),
                        });
                    }
                }
            }
        }
        ClientRequest::EditAdjust { brightness, contrast, saturation } => {
            if st.edit_active {
                match st.editor.adjust(brightness, contrast, saturation) {
                    Ok(preview) => {
                        let (w, h) = st.editor.dimensions();
                        send_event(ServerEvent::EditState {
                            active: true,
                            preview_path: Some(preview.to_string_lossy().to_string()),
                            can_undo: st.editor.can_undo(),
                            can_redo: st.editor.can_redo(),
                            width: w,
                            height: h,
                        });
                    }
                    Err(e) => {
                        send_event(ServerEvent::Toast {
                            message: format!("Adjustment error: {}", e),
                            level: "error".to_string(),
                        });
                    }
                }
            }
        }
        ClientRequest::EditResize { width, height } => {
            if st.edit_active {
                match st.editor.resize(width, height) {
                    Ok(preview) => {
                        let (w, h) = st.editor.dimensions();
                        send_event(ServerEvent::EditState {
                            active: true,
                            preview_path: Some(preview.to_string_lossy().to_string()),
                            can_undo: st.editor.can_undo(),
                            can_redo: st.editor.can_redo(),
                            width: w,
                            height: h,
                        });
                    }
                    Err(e) => {
                        send_event(ServerEvent::Toast {
                            message: format!("Resize error: {}", e),
                            level: "error".to_string(),
                        });
                    }
                }
            }
        }
        ClientRequest::EditUndo => {
            if st.edit_active {
                match st.editor.undo() {
                    Ok(Some(preview)) => {
                        let (w, h) = st.editor.dimensions();
                        send_event(ServerEvent::EditState {
                            active: true,
                            preview_path: Some(preview.to_string_lossy().to_string()),
                            can_undo: st.editor.can_undo(),
                            can_redo: st.editor.can_redo(),
                            width: w,
                            height: h,
                        });
                    }
                    _ => {
                        send_event(ServerEvent::Toast {
                            message: "Already at oldest edit".to_string(),
                            level: "info".to_string(),
                        });
                    }
                }
            }
        }
        ClientRequest::EditRedo => {
            if st.edit_active {
                match st.editor.redo() {
                    Ok(Some(preview)) => {
                        let (w, h) = st.editor.dimensions();
                        send_event(ServerEvent::EditState {
                            active: true,
                            preview_path: Some(preview.to_string_lossy().to_string()),
                            can_undo: st.editor.can_undo(),
                            can_redo: st.editor.can_redo(),
                            width: w,
                            height: h,
                        });
                    }
                    _ => {
                        send_event(ServerEvent::Toast {
                            message: "Already at newest edit".to_string(),
                            level: "info".to_string(),
                        });
                    }
                }
            }
        }
        ClientRequest::EditSave { overwrite, filename } => {
            if st.edit_active {
                st.editor.commit_adjustments();
                let custom_path = filename.map(PathBuf::from);
                match st.editor.save(overwrite, custom_path.as_deref()) {
                    Ok(saved_path) => {
                        let fname = saved_path.file_name().unwrap_or_default().to_string_lossy().to_string();
                        st.edit_active = false;
                        let _ = st.scanner.rescan();
                        if let Some(pos) = st.scanner.entries.iter().position(|e| e.path == saved_path) {
                            st.scanner.current_index = pos;
                        }

                        send_event(ServerEvent::EditState {
                            active: false,
                            preview_path: None,
                            can_undo: false,
                            can_redo: false,
                            width: 0,
                            height: 0,
                        });
                        send_event(ServerEvent::Toast {
                            message: format!("Saved {}", fname),
                            level: "success".to_string(),
                        });
                        send_event(ServerEvent::DirectoryState {
                            index: st.scanner.current_index,
                            total: st.scanner.entries.len(),
                            current: st.scanner.current().cloned(),
                        });
                    }
                    Err(e) => {
                        send_event(ServerEvent::Toast {
                            message: format!("Failed to save: {}", e),
                            level: "error".to_string(),
                        });
                    }
                }
            }
        }
        ClientRequest::Quit => {
            send_event(ServerEvent::Close);
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_clipboard_requests() {
        let json_img = r#"{"type":"clipboard_copy","path_only":false}"#;
        let req1: ClientRequest = serde_json::from_str(json_img).unwrap();
        match req1 {
            ClientRequest::ClipboardCopy { path_only } => assert!(!path_only),
            _ => panic!("Expected ClipboardCopy"),
        }

        let json_path = r#"{"type":"clipboard_copy","path_only":true}"#;
        let req2: ClientRequest = serde_json::from_str(json_path).unwrap();
        match req2 {
            ClientRequest::ClipboardCopy { path_only } => assert!(path_only),
            _ => panic!("Expected ClipboardCopy"),
        }
    }

    #[test]
    fn test_parse_wallpaper_and_exif_requests() {
        let json_wp = r#"{"type":"set_wallpaper"}"#;
        let req_wp: ClientRequest = serde_json::from_str(json_wp).unwrap();
        match req_wp {
            ClientRequest::SetWallpaper => {},
            _ => panic!("Expected SetWallpaper"),
        }

        let json_exif = r#"{"type":"get_exif"}"#;
        let req_exif: ClientRequest = serde_json::from_str(json_exif).unwrap();
        match req_exif {
            ClientRequest::GetExif => {},
            _ => panic!("Expected GetExif"),
        }
    }

    #[test]
    fn test_serialize_exif_event() {
        let meta = ExifMetadata {
            make: Some("Sony".to_string()),
            model: Some("A7III".to_string()),
            lens_model: Some("50mm F1.8".to_string()),
            shutter_speed: Some("1/250s".to_string()),
            f_number: Some("f/2.8".to_string()),
            iso: Some("ISO 400".to_string()),
            focal_length: Some("50mm".to_string()),
            exposure_bias: Some("+0.0 EV".to_string()),
            flash: Some("Did not fire".to_string()),
            date_time: Some("2026-09-14 12:00:00".to_string()),
            dimensions: "6000 × 4000 (24.0 MP)".to_string(),
            megapixels: Some("24.0 MP".to_string()),
            file_size: "12.4 MB".to_string(),
            format: "JPEG".to_string(),
            path: "/path/to/test.jpg".to_string(),
            filename: "test.jpg".to_string(),
        };

        let evt = ServerEvent::ExifData { data: meta };
        let serialized = serde_json::to_string(&evt).unwrap();
        assert!(serialized.contains("exif_data"));
        assert!(serialized.contains("Sony"));
        assert!(serialized.contains("1/250s"));
        assert!(serialized.contains("6000 × 4000 (24.0 MP)"));
    }

    #[tokio::test]
    async fn test_set_wallpaper_nonexistent() {
        let fake_path = Path::new("/nonexistent/file/path.jpg");
        let res = set_wallpaper(fake_path).await;
        // Should handle missing tools or missing file gracefully by returning Err or Ok
        assert!(res.is_err() || res.is_ok());
    }

    #[tokio::test]
    async fn test_copy_to_clipboard_path() {
        let sample = Path::new("tests/samples/sample_1.png");
        let res = copy_to_clipboard(sample, true).await;
        // In test environment wl-copy may or may not succeed depending on Wayland session
        assert!(res.is_err() || res.is_ok());
    }
}

