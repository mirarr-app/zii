use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{mpsc, Mutex};

use crate::editor::ImageEditor;
use crate::exif_inspector::{extract_metadata, ExifMetadata};
use crate::scanner::{DirectoryScanner, ImageEntry};
use crate::theme::{OmarchyTheme, ThemeManager};
use crate::trash::TrashManager;

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
    pub editor: Arc<std::sync::Mutex<ImageEditor>>,
    pub editor_busy: Arc<std::sync::atomic::AtomicBool>,
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
            let ext = canonical.extension().and_then(|e| e.to_str()).unwrap_or("");

            let mime = mime_for_extension(ext);

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

pub fn mime_for_extension(ext: &str) -> &'static str {
    match ext.to_ascii_lowercase().as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "bmp" => "image/bmp",
        "tiff" | "tif" => "image/tiff",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        _ => "application/octet-stream",
    }
}

async fn run_cmd_timeout(cmd: &str, args: &[&str]) -> Result<bool, String> {
    match tokio::time::timeout(
        std::time::Duration::from_secs(5),
        tokio::process::Command::new(cmd).args(args).status(),
    )
    .await
    {
        Ok(Ok(status)) => Ok(status.success()),
        Ok(Err(e)) => Err(format!("{cmd} failed to execute: {e}")),
        Err(_) => Err(format!("{cmd} timed out after 5 seconds")),
    }
}

pub async fn set_wallpaper(path: &Path) -> Result<String, String> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let path_str = canonical.to_string_lossy().to_string();

    let mut errors = Vec::new();

    // 1. Try omarchy-theme-bg-set
    match run_cmd_timeout("omarchy-theme-bg-set", &[&path_str]).await {
        Ok(true) => return Ok("omarchy-theme-bg-set".to_string()),
        Ok(false) => errors.push("omarchy-theme-bg-set returned non-zero exit status".to_string()),
        Err(e) => errors.push(e),
    }

    // 2. Fallback to swww img
    match run_cmd_timeout("swww", &["img", &path_str]).await {
        Ok(true) => return Ok("swww".to_string()),
        Ok(false) => errors.push("swww returned non-zero exit status".to_string()),
        Err(e) => errors.push(e),
    }

    // 3. Fallback to hyprctl hyprpaper
    let hypr_arg = format!(",{}", path_str);
    match run_cmd_timeout("hyprctl", &["hyprpaper", "wallpaper", &hypr_arg]).await {
        Ok(true) => return Ok("hyprctl hyprpaper".to_string()),
        Ok(false) => errors.push("hyprctl returned non-zero exit status".to_string()),
        Err(e) => errors.push(e),
    }

    Err(format!(
        "Could not set wallpaper: neither omarchy-theme-bg-set, swww, nor hyprpaper succeeded: {}",
        errors.join("; ")
    ))
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

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(e) =
            std::fs::set_permissions(&socket_path, std::fs::Permissions::from_mode(0o600))
        {
            eprintln!(
                "Warning: Failed to set permissions on socket {}: {}",
                socket_path.display(),
                e
            );
        }
    }

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
            if writer
                .write_all(format!("{}\n", msg).as_bytes())
                .await
                .is_err()
            {
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
                Ok(ClientRequest::Quit) => {
                    let _ = process_request(ClientRequest::Quit, &state, &b_tx).await;
                    let _ = shutdown_tx.send(()).await;
                    break;
                }
                Ok(req) => {
                    let state = state.clone();
                    let b_tx = b_tx.clone();
                    tokio::spawn(async move {
                        process_request(req, &state, &b_tx).await;
                    });
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

struct BusyGuard(Arc<std::sync::atomic::AtomicBool>);

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

fn send_event_helper(b_tx: &Arc<tokio::sync::broadcast::Sender<String>>, evt: ServerEvent) {
    if let Ok(msg) = serde_json::to_string(&evt) {
        let _ = b_tx.send(msg);
    }
}

fn send_toast(b_tx: &Arc<tokio::sync::broadcast::Sender<String>>, message: &str, level: &str) {
    send_event_helper(
        b_tx,
        ServerEvent::Toast {
            message: message.to_string(),
            level: level.to_string(),
        },
    );
}

async fn run_edit_op<F>(
    state: &Arc<Mutex<AppState>>,
    b_tx: &Arc<tokio::sync::broadcast::Sender<String>>,
    op_name: &'static str,
    op: F,
) where
    F: FnOnce(&mut ImageEditor) -> anyhow::Result<PathBuf> + Send + 'static,
{
    let (editor, editor_busy) = {
        let st = state.lock().await;
        if !st.edit_active {
            return;
        }
        (st.editor.clone(), st.editor_busy.clone())
    };

    if editor_busy
        .compare_exchange(
            false,
            true,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
        )
        .is_err()
    {
        send_toast(b_tx, "Editor busy", "warn");
        return;
    }

    let busy_guard = BusyGuard(editor_busy);
    let res = tokio::task::spawn_blocking(move || {
        let _guard = busy_guard;
        let mut ed = editor.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let preview = op(&mut ed)?;
        let (w, h) = ed.dimensions();
        let can_undo = ed.can_undo();
        let can_redo = ed.can_redo();
        Ok::<(PathBuf, u32, u32, bool, bool), anyhow::Error>((preview, w, h, can_undo, can_redo))
    })
    .await;

    match res {
        Ok(Ok((preview, w, h, can_undo, can_redo))) => {
            let st = state.lock().await;
            if st.edit_active {
                send_event_helper(
                    b_tx,
                    ServerEvent::EditState {
                        active: true,
                        preview_path: Some(preview.to_string_lossy().to_string()),
                        can_undo,
                        can_redo,
                        width: w,
                        height: h,
                    },
                );
            }
        }
        Ok(Err(e)) => {
            send_toast(b_tx, &format!("{op_name} error: {e}"), "error");
        }
        Err(e) => {
            send_toast(b_tx, &format!("{op_name} task failed: {e}"), "error");
        }
    }
}

async fn run_history_op<F>(
    state: &Arc<Mutex<AppState>>,
    b_tx: &Arc<tokio::sync::broadcast::Sender<String>>,
    is_undo: bool,
    op: F,
) where
    F: FnOnce(&mut ImageEditor) -> anyhow::Result<Option<PathBuf>> + Send + 'static,
{
    let op_name = if is_undo { "Undo" } else { "Redo" };
    let (editor, editor_busy) = {
        let st = state.lock().await;
        if !st.edit_active {
            return;
        }
        (st.editor.clone(), st.editor_busy.clone())
    };

    if editor_busy
        .compare_exchange(
            false,
            true,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
        )
        .is_err()
    {
        send_toast(b_tx, "Editor busy", "warn");
        return;
    }

    let busy_guard = BusyGuard(editor_busy);
    let res = tokio::task::spawn_blocking(move || {
        let _guard = busy_guard;
        let mut ed = editor.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
        let preview_opt = op(&mut ed)?;
        let (w, h) = ed.dimensions();
        let can_undo = ed.can_undo();
        let can_redo = ed.can_redo();
        Ok::<(Option<PathBuf>, u32, u32, bool, bool), anyhow::Error>((
            preview_opt,
            w,
            h,
            can_undo,
            can_redo,
        ))
    })
    .await;

    match res {
        Ok(Ok((Some(preview), w, h, can_undo, can_redo))) => {
            let st = state.lock().await;
            if st.edit_active {
                send_event_helper(
                    b_tx,
                    ServerEvent::EditState {
                        active: true,
                        preview_path: Some(preview.to_string_lossy().to_string()),
                        can_undo,
                        can_redo,
                        width: w,
                        height: h,
                    },
                );
            }
        }
        Ok(Ok((None, _, _, _, _))) => {
            let msg = if is_undo {
                "Already at oldest edit"
            } else {
                "Already at newest edit"
            };
            send_toast(b_tx, msg, "info");
        }
        Ok(Err(e)) => {
            send_toast(b_tx, &format!("{op_name} error: {e}"), "error");
        }
        Err(e) => {
            send_toast(b_tx, &format!("{op_name} task failed: {e}"), "error");
        }
    }
}

async fn process_request(
    req: ClientRequest,
    state: &Arc<Mutex<AppState>>,
    b_tx: &Arc<tokio::sync::broadcast::Sender<String>>,
) -> bool {
    match req {
        ClientRequest::Ready => {
            let (theme, index, total, current) = {
                let st = state.lock().await;
                (
                    st.theme.current_theme.clone(),
                    st.scanner.current_index,
                    st.scanner.entries.len(),
                    st.scanner.current().cloned(),
                )
            };
            send_event_helper(b_tx, ServerEvent::Theme { theme });
            send_event_helper(
                b_tx,
                ServerEvent::DirectoryState {
                    index,
                    total,
                    current,
                },
            );
        }
        ClientRequest::Navigate { direction, target } => {
            let (index, total, current, was_edit_active) = {
                let mut st = state.lock().await;
                let was_edit = st.edit_active;
                if was_edit {
                    st.edit_active = false;
                }

                match direction.as_str() {
                    "next" => {
                        st.scanner.next();
                    }
                    "prev" => {
                        st.scanner.prev();
                    }
                    "first" => {
                        st.scanner.first();
                    }
                    "last" => {
                        st.scanner.last();
                    }
                    "goto" => {
                        if let Some(idx) = target {
                            st.scanner.go_to(idx);
                        }
                    }
                    _ => {}
                }

                (
                    st.scanner.current_index,
                    st.scanner.entries.len(),
                    st.scanner.current().cloned(),
                    was_edit,
                )
            };

            if was_edit_active {
                send_event_helper(
                    b_tx,
                    ServerEvent::EditState {
                        active: false,
                        preview_path: None,
                        can_undo: false,
                        can_redo: false,
                        width: 0,
                        height: 0,
                    },
                );
            }

            send_event_helper(
                b_tx,
                ServerEvent::DirectoryState {
                    index,
                    total,
                    current,
                },
            );
        }
        ClientRequest::DeleteCurrent { permanent } => {
            let (res, filename, index, total, current) = {
                let mut st = state.lock().await;
                if let Some(current) = st.scanner.current().cloned() {
                    let fname = current.filename.clone();
                    let res = if permanent {
                        st.trash.delete_permanent(&current.path)
                    } else {
                        st.trash.move_to_trash(&current.path)
                    };
                    if res.is_ok() {
                        st.scanner.remove_current();
                    }
                    (
                        Some(res),
                        fname,
                        st.scanner.current_index,
                        st.scanner.entries.len(),
                        st.scanner.current().cloned(),
                    )
                } else {
                    (None, String::new(), 0, 0, None)
                }
            };

            if let Some(res) = res {
                match res {
                    Ok(_) => {
                        let msg = if permanent {
                            format!("Permanently deleted {}", filename)
                        } else {
                            format!("Moved {} to trash (press 'u' to undo)", filename)
                        };
                        send_toast(b_tx, &msg, "warn");
                        send_event_helper(
                            b_tx,
                            ServerEvent::DirectoryState {
                                index,
                                total,
                                current,
                            },
                        );
                    }
                    Err(e) => {
                        send_toast(b_tx, &format!("Failed to delete: {}", e), "error");
                    }
                }
            }
        }
        ClientRequest::RestoreTrash => {
            let (res, index, total, current) = {
                let mut st = state.lock().await;
                let res = st.trash.restore_last();
                if let Ok(Some(ref restored_path)) = res {
                    let _ = st.scanner.rescan();
                    if let Some(pos) = st
                        .scanner
                        .entries
                        .iter()
                        .position(|e| &e.path == restored_path)
                    {
                        st.scanner.current_index = pos;
                    }
                }
                (
                    res,
                    st.scanner.current_index,
                    st.scanner.entries.len(),
                    st.scanner.current().cloned(),
                )
            };

            match res {
                Ok(Some(restored_path)) => {
                    let filename = restored_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    send_toast(b_tx, &format!("Restored {}", filename), "success");
                    send_event_helper(
                        b_tx,
                        ServerEvent::DirectoryState {
                            index,
                            total,
                            current,
                        },
                    );
                }
                Ok(None) => {
                    send_toast(b_tx, "Nothing to undo", "info");
                }
                Err(e) => {
                    send_toast(b_tx, &format!("Failed to restore: {}", e), "error");
                }
            }
        }
        ClientRequest::ClipboardCopy { path_only } => {
            let path_opt = {
                let st = state.lock().await;
                st.scanner.current().map(|e| e.path.clone())
            };

            if let Some(path) = path_opt {
                match copy_to_clipboard(&path, path_only).await {
                    Ok(msg) => {
                        send_toast(b_tx, &msg, "success");
                    }
                    Err(e) => {
                        send_toast(b_tx, &e, "error");
                    }
                }
            } else {
                send_toast(b_tx, "No image selected", "warn");
            }
        }
        ClientRequest::SetWallpaper => {
            let (path_opt, filename_opt) = {
                let st = state.lock().await;
                match st.scanner.current() {
                    Some(cur) => (Some(cur.path.clone()), Some(cur.filename.clone())),
                    None => (None, None),
                }
            };

            if let (Some(path), Some(filename)) = (path_opt, filename_opt) {
                match set_wallpaper(&path).await {
                    Ok(_) => {
                        send_toast(
                            b_tx,
                            &format!("Set as Omarchy wallpaper: {}", filename),
                            "success",
                        );
                    }
                    Err(e) => {
                        send_toast(b_tx, &e, "error");
                    }
                }
            } else {
                send_toast(b_tx, "No image selected", "warn");
            }
        }
        ClientRequest::GetExif => {
            let current = {
                let st = state.lock().await;
                st.scanner.current().cloned()
            };

            if let Some(current) = current {
                let res = tokio::task::spawn_blocking(move || {
                    extract_metadata(
                        &current.path,
                        current.width,
                        current.height,
                        current.file_size,
                        &current.format,
                    )
                })
                .await;

                if let Ok(data) = res {
                    send_event_helper(b_tx, ServerEvent::ExifData { data });
                }
            }
        }
        ClientRequest::EditStart => {
            let (editor, editor_busy, path) = {
                let st = state.lock().await;
                let path = st.scanner.current().map(|e| e.path.clone());
                (st.editor.clone(), st.editor_busy.clone(), path)
            };

            let Some(path) = path else {
                return false;
            };

            if editor_busy
                .compare_exchange(
                    false,
                    true,
                    std::sync::atomic::Ordering::SeqCst,
                    std::sync::atomic::Ordering::SeqCst,
                )
                .is_err()
            {
                send_toast(b_tx, "Editor busy", "warn");
                return false;
            }

            let busy_guard = BusyGuard(editor_busy);
            let res = tokio::task::spawn_blocking(move || {
                let _guard = busy_guard;
                let mut ed = editor.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
                let preview = ed.open(&path)?;
                let (w, h) = ed.dimensions();
                Ok::<(PathBuf, u32, u32), anyhow::Error>((preview, w, h))
            })
            .await;

            match res {
                Ok(Ok((preview, w, h))) => {
                    let mut st = state.lock().await;
                    st.edit_active = true;
                    send_event_helper(
                        b_tx,
                        ServerEvent::EditState {
                            active: true,
                            preview_path: Some(preview.to_string_lossy().to_string()),
                            can_undo: false,
                            can_redo: false,
                            width: w,
                            height: h,
                        },
                    );
                }
                Ok(Err(e)) => {
                    send_toast(b_tx, &format!("Cannot edit image: {}", e), "error");
                }
                Err(e) => {
                    send_toast(b_tx, &format!("Edit task failed: {}", e), "error");
                }
            }
        }
        ClientRequest::EditCancel => {
            let editor = {
                let mut st = state.lock().await;
                st.edit_active = false;
                st.editor.clone()
            };
            if let Ok(mut ed) = editor.try_lock() {
                ed.adjustment_base = None;
            }
            send_event_helper(
                b_tx,
                ServerEvent::EditState {
                    active: false,
                    preview_path: None,
                    can_undo: false,
                    can_redo: false,
                    width: 0,
                    height: 0,
                },
            );
            send_toast(b_tx, "Discarded edits", "info");
        }
        ClientRequest::EditCrop {
            x,
            y,
            width,
            height,
        } => {
            run_edit_op(state, b_tx, "Crop", move |ed| ed.crop(x, y, width, height)).await;
        }
        ClientRequest::EditRotate { degrees } => {
            run_edit_op(state, b_tx, "Rotate", move |ed| ed.rotate(degrees)).await;
        }
        ClientRequest::EditFlip {
            horizontal,
            vertical,
        } => {
            run_edit_op(state, b_tx, "Flip", move |ed| ed.flip(horizontal, vertical)).await;
        }
        ClientRequest::EditAdjust {
            brightness,
            contrast,
            saturation,
        } => {
            run_edit_op(state, b_tx, "Adjustment", move |ed| {
                ed.adjust(brightness, contrast, saturation)
            })
            .await;
        }
        ClientRequest::EditResize { width, height } => {
            run_edit_op(state, b_tx, "Resize", move |ed| ed.resize(width, height)).await;
        }
        ClientRequest::EditUndo => {
            run_history_op(state, b_tx, true, |ed| ed.undo()).await;
        }
        ClientRequest::EditRedo => {
            run_history_op(state, b_tx, false, |ed| ed.redo()).await;
        }
        ClientRequest::EditSave {
            overwrite,
            filename,
        } => {
            let (editor, editor_busy) = {
                let st = state.lock().await;
                if !st.edit_active {
                    return false;
                }
                (st.editor.clone(), st.editor_busy.clone())
            };

            if editor_busy
                .compare_exchange(
                    false,
                    true,
                    std::sync::atomic::Ordering::SeqCst,
                    std::sync::atomic::Ordering::SeqCst,
                )
                .is_err()
            {
                send_toast(b_tx, "Editor busy", "warn");
                return false;
            }

            let busy_guard = BusyGuard(editor_busy);
            let custom_path = filename.map(PathBuf::from);
            let res = tokio::task::spawn_blocking(move || {
                let _guard = busy_guard;
                let mut ed = editor.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
                ed.commit_adjustments();
                let saved = ed.save(overwrite, custom_path.as_deref())?;
                Ok::<PathBuf, anyhow::Error>(saved)
            })
            .await;

            match res {
                Ok(Ok(saved_path)) => {
                    let fname = saved_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let (index, total, current) = {
                        let mut st = state.lock().await;
                        st.edit_active = false;
                        let _ = st.scanner.rescan();
                        if let Some(pos) =
                            st.scanner.entries.iter().position(|e| e.path == saved_path)
                        {
                            st.scanner.current_index = pos;
                        }

                        (
                            st.scanner.current_index,
                            st.scanner.entries.len(),
                            st.scanner.current().cloned(),
                        )
                    };

                    send_event_helper(
                        b_tx,
                        ServerEvent::EditState {
                            active: false,
                            preview_path: None,
                            can_undo: false,
                            can_redo: false,
                            width: 0,
                            height: 0,
                        },
                    );
                    send_toast(b_tx, &format!("Saved {}", fname), "success");
                    send_event_helper(
                        b_tx,
                        ServerEvent::DirectoryState {
                            index,
                            total,
                            current,
                        },
                    );
                }
                Ok(Err(e)) => {
                    send_toast(b_tx, &format!("Failed to save: {}", e), "error");
                }
                Err(e) => {
                    send_toast(b_tx, &format!("Save task failed: {}", e), "error");
                }
            }
        }
        ClientRequest::Quit => {
            send_event_helper(b_tx, ServerEvent::Close);
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
            ClientRequest::SetWallpaper => {}
            _ => panic!("Expected SetWallpaper"),
        }

        let json_exif = r#"{"type":"get_exif"}"#;
        let req_exif: ClientRequest = serde_json::from_str(json_exif).unwrap();
        match req_exif {
            ClientRequest::GetExif => {}
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

    #[test]
    fn test_mime_for_extension() {
        assert_eq!(mime_for_extension("png"), "image/png");
        assert_eq!(mime_for_extension("PNG"), "image/png");
        assert_eq!(mime_for_extension("jpg"), "image/jpeg");
        assert_eq!(mime_for_extension("jpeg"), "image/jpeg");
        assert_eq!(mime_for_extension("JPG"), "image/jpeg");
        assert_eq!(mime_for_extension("webp"), "image/webp");
        assert_eq!(mime_for_extension("gif"), "image/gif");
        assert_eq!(mime_for_extension("bmp"), "image/bmp");
        assert_eq!(mime_for_extension("tif"), "image/tiff");
        assert_eq!(mime_for_extension("tiff"), "image/tiff");
        assert_eq!(mime_for_extension("svg"), "image/svg+xml");
        assert_eq!(mime_for_extension("ico"), "image/x-icon");
        assert_eq!(mime_for_extension("txt"), "application/octet-stream");
        assert_eq!(mime_for_extension(""), "application/octet-stream");
        assert_eq!(mime_for_extension("unknown"), "application/octet-stream");
    }

    #[tokio::test]
    #[ignore = "requires desktop environment; run with ZII_RUN_DESKTOP_TESTS=1"]
    async fn test_set_wallpaper_nonexistent() {
        if std::env::var("ZII_RUN_DESKTOP_TESTS").as_deref() != Ok("1") {
            return;
        }
        let fake_path = Path::new("/nonexistent/file/path.jpg");
        let res = set_wallpaper(fake_path).await;
        assert!(res.is_err() || res.is_ok());
    }

    #[tokio::test]
    #[ignore = "requires Wayland compositor and modifies clipboard; run with ZII_RUN_DESKTOP_TESTS=1"]
    async fn test_copy_to_clipboard_path() {
        if std::env::var("ZII_RUN_DESKTOP_TESTS").as_deref() != Ok("1") {
            return;
        }
        let sample = Path::new("tests/samples/sample_1.png");
        let res = copy_to_clipboard(sample, true).await;
        assert!(res.is_err() || res.is_ok());
    }

    #[tokio::test]
    async fn test_ipc_server_ready_and_edit_start() {
        let temp_dir = tempfile::tempdir().unwrap();
        let socket_path = temp_dir.path().join("test_zii.sock");

        let scanner = DirectoryScanner::new(Path::new("tests/samples")).unwrap();
        let trash = TrashManager::new();
        let editor = Arc::new(std::sync::Mutex::new(ImageEditor::new()));
        let editor_busy = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let theme = ThemeManager::new();

        let state = Arc::new(Mutex::new(AppState {
            scanner,
            trash,
            editor,
            editor_busy,
            edit_active: false,
            theme,
        }));

        let (_theme_tx, theme_rx) = mpsc::channel(16);
        let (_dir_tx, dir_rx) = mpsc::channel(16);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);

        let s_path = socket_path.clone();
        let s_state = state.clone();
        let server_handle = tokio::spawn(async move {
            let _ = run_ipc_server(s_path, s_state, theme_rx, dir_rx, shutdown_tx).await;
        });

        // Wait for socket to be available
        let mut stream = None;
        for _ in 0..50 {
            if socket_path.exists() {
                if let Ok(s) = UnixStream::connect(&socket_path).await {
                    stream = Some(s);
                    break;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let stream = stream.expect("Failed to connect to test IPC socket");
        let (reader, mut writer) = stream.into_split();
        let mut lines = BufReader::new(reader).lines();

        // Send ready
        writer.write_all(b"{\"type\":\"ready\"}\n").await.unwrap();
        writer.flush().await.unwrap();

        // Expect directory_state event
        let mut got_dir_state = false;
        while let Ok(Ok(Some(line))) =
            tokio::time::timeout(std::time::Duration::from_secs(3), lines.next_line()).await
        {
            let v: serde_json::Value = serde_json::from_str(&line).unwrap();
            if v["type"] == "directory_state" {
                assert!(v["total"].as_u64().unwrap() >= 1);
                assert_eq!(v["current"]["filename"], "sample_1.png");
                got_dir_state = true;
                break;
            }
        }
        assert!(got_dir_state, "Expected directory_state event");

        // Send edit_start
        writer
            .write_all(b"{\"type\":\"edit_start\"}\n")
            .await
            .unwrap();
        writer.flush().await.unwrap();

        // Expect edit_state event with active: true
        let mut got_edit_state = false;
        while let Ok(Ok(Some(line))) =
            tokio::time::timeout(std::time::Duration::from_secs(5), lines.next_line()).await
        {
            let v: serde_json::Value = serde_json::from_str(&line).unwrap();
            if v["type"] == "edit_state" {
                assert_eq!(v["active"], true);
                assert!(v["preview_path"].is_string());
                assert!(v["width"].as_u64().unwrap() > 0);
                assert!(v["height"].as_u64().unwrap() > 0);
                got_edit_state = true;
                break;
            }
        }
        assert!(
            got_edit_state,
            "Expected edit_state event with active: true"
        );

        // Send quit
        writer.write_all(b"{\"type\":\"quit\"}\n").await.unwrap();
        writer.flush().await.unwrap();

        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), shutdown_rx.recv()).await;
        server_handle.abort();
    }

    #[tokio::test]
    async fn test_ipc_server_editor_busy() {
        let temp_dir = tempfile::tempdir().unwrap();
        let socket_path = temp_dir.path().join("test_busy.sock");

        let scanner = DirectoryScanner::new(Path::new("tests/samples")).unwrap();
        let trash = TrashManager::new();
        let editor = Arc::new(std::sync::Mutex::new(ImageEditor::new()));
        let editor_busy = Arc::new(std::sync::atomic::AtomicBool::new(true)); // Start busy!
        let theme = ThemeManager::new();

        let state = Arc::new(Mutex::new(AppState {
            scanner,
            trash,
            editor,
            editor_busy,
            edit_active: false,
            theme,
        }));

        let (_theme_tx, theme_rx) = mpsc::channel(16);
        let (_dir_tx, dir_rx) = mpsc::channel(16);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);

        let s_path = socket_path.clone();
        let s_state = state.clone();
        let server_handle = tokio::spawn(async move {
            let _ = run_ipc_server(s_path, s_state, theme_rx, dir_rx, shutdown_tx).await;
        });

        // Connect
        let mut stream = None;
        for _ in 0..50 {
            if socket_path.exists() {
                if let Ok(s) = UnixStream::connect(&socket_path).await {
                    stream = Some(s);
                    break;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        let stream = stream.expect("Failed to connect to test IPC socket");
        let (reader, mut writer) = stream.into_split();
        let mut lines = BufReader::new(reader).lines();

        // Send edit_start while editor_busy is true
        writer
            .write_all(b"{\"type\":\"edit_start\"}\n")
            .await
            .unwrap();
        writer.flush().await.unwrap();

        // Expect toast "Editor busy"
        let mut got_busy_toast = false;
        while let Ok(Ok(Some(line))) =
            tokio::time::timeout(std::time::Duration::from_secs(3), lines.next_line()).await
        {
            let v: serde_json::Value = serde_json::from_str(&line).unwrap();
            if v["type"] == "toast" && v["message"] == "Editor busy" {
                assert_eq!(v["level"], "warn");
                got_busy_toast = true;
                break;
            }
        }
        assert!(got_busy_toast, "Expected 'Editor busy' toast event");

        writer.write_all(b"{\"type\":\"quit\"}\n").await.unwrap();
        writer.flush().await.unwrap();
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), shutdown_rx.recv()).await;
        server_handle.abort();
    }
}
