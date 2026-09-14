use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, Mutex};
use serde::{Deserialize, Serialize};

use crate::scanner::{DirectoryScanner, ImageEntry};
use crate::theme::{OmarchyTheme, ThemeManager};
use crate::trash::TrashManager;
use crate::editor::ImageEditor;

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

pub async fn run_ipc_server(
    socket_path: PathBuf,
    state: Arc<Mutex<AppState>>,
    mut theme_rx: mpsc::Receiver<OmarchyTheme>,
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
        ClientRequest::EditAdjust { brightness, contrast } => {
            if st.edit_active {
                match st.editor.adjust(brightness, contrast) {
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
