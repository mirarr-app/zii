mod scanner;
mod theme;
mod trash;
mod editor;
mod ipc;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use scanner::DirectoryScanner;
use theme::ThemeManager;
use trash::TrashManager;
use editor::ImageEditor;
use ipc::{run_ipc_server, AppState};

fn get_socket_path(pid: u32) -> PathBuf {
    if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        if !runtime_dir.is_empty() {
            let p = PathBuf::from(runtime_dir);
            if p.exists() {
                return p.join(format!("zii_{}.sock", pid));
            }
        }
    }
    std::env::temp_dir().join(format!("zii_{}.sock", pid))
}

fn resolve_ui_path() -> PathBuf {
    // a. ZII_UI_PATH env var
    if let Ok(custom) = std::env::var("ZII_UI_PATH") {
        let p = PathBuf::from(custom);
        if p.exists() {
            return p.canonicalize().unwrap_or(p);
        }
        return p;
    }

    // b. ./ui/shell.qml (relative to current working dir)
    let cwd_ui = Path::new("./ui/shell.qml");
    if cwd_ui.exists() {
        return cwd_ui.canonicalize().unwrap_or_else(|_| cwd_ui.to_path_buf());
    }

    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    // c. ../ui/shell.qml relative to executable
    let exe_parent_ui = exe_dir.join("../ui/shell.qml");
    if exe_parent_ui.exists() {
        return exe_parent_ui.canonicalize().unwrap_or(exe_parent_ui);
    }

    // d. ui/shell.qml relative to executable
    let exe_child_ui = exe_dir.join("ui/shell.qml");
    if exe_child_ui.exists() {
        return exe_child_ui.canonicalize().unwrap_or(exe_child_ui);
    }

    // e. ~/.local/share/zii/ui/shell.qml
    let local_share_ui = {
        let data_home = std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                let home = std::env::var("HOME").unwrap_or_default();
                if !home.is_empty() {
                    PathBuf::from(home).join(".local/share")
                } else {
                    PathBuf::from(".").join(".local/share")
                }
            });
        data_home.join("zii/ui/shell.qml")
    };
    if local_share_ui.exists() {
        return local_share_ui.canonicalize().unwrap_or(local_share_ui);
    }

    // f. /usr/share/zii/ui/shell.qml
    let usr_share_ui = Path::new("/usr/share/zii/ui/shell.qml");
    if usr_share_ui.exists() {
        return usr_share_ui.canonicalize().unwrap_or_else(|_| usr_share_ui.to_path_buf());
    }

    // g. compile-time development fallback
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/shell.qml"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let target = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from(".")
    };

    let target_path = if target.exists() {
        target.canonicalize().unwrap_or(target)
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    let scanner = DirectoryScanner::new(&target_path)?;
    let trash = TrashManager::new();
    let editor = ImageEditor::new();
    let theme = ThemeManager::new();

    let pid = std::process::id();
    let socket_path = get_socket_path(pid);

    let app_state = Arc::new(Mutex::new(AppState {
        scanner,
        trash,
        editor,
        edit_active: false,
        theme,
    }));

    let (theme_tx, theme_rx) = mpsc::channel(16);
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);

    // Start Omarchy theme watcher
    if let Err(e) = ThemeManager::start_watcher(theme_tx) {
        eprintln!("Theme watcher warning: {:?}", e);
    }

    // Spawn IPC server
    let s_path = socket_path.clone();
    let s_state = app_state.clone();
    let s_shutdown_tx = shutdown_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = run_ipc_server(s_path, s_state, theme_rx, s_shutdown_tx).await {
            eprintln!("IPC Server error: {:?}", e);
        }
    });

    let ui_path = resolve_ui_path();

    println!("Starting Zii with target: {:?}", target_path);
    println!("Socket: {:?}", socket_path);
    println!("Loading UI: {:?}", ui_path);

    // Launch Quickshell child process using tokio::process
    let mut qs_cmd = tokio::process::Command::new("quickshell");
    qs_cmd.arg("-p").arg(&ui_path);
    qs_cmd.env("ZII_SOCKET", &socket_path);

    let mut qs_child = match qs_cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            eprintln!("Failed to spawn quickshell: {:?}", e);
            let _ = std::fs::remove_file(&socket_path);
            let st = app_state.lock().await;
            st.editor.cleanup();
            st.trash.cleanup();
            return Err(e.into());
        }
    };

    // Wait for either quickshell to terminate or shutdown_rx from IPC
    tokio::select! {
        _ = shutdown_rx.recv() => {
            println!("Received shutdown signal from UI.");
            let _ = qs_child.kill().await;
        }
        status = qs_child.wait() => {
            match status {
                Ok(s) => println!("Quickshell exited with status: {}", s),
                Err(e) => eprintln!("Error waiting for quickshell: {:?}", e),
            }
        }
    }

    // Cleanup socket, preview cache, and trash backup
    let _ = std::fs::remove_file(&socket_path);
    {
        let st = app_state.lock().await;
        st.editor.cleanup();
        st.trash.cleanup();
    }
    println!("Zii closed cleanly.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_path() {
        let pid = 999999;
        let socket = get_socket_path(pid);
        let s = socket.to_string_lossy();
        assert!(s.contains("zii_999999.sock"));
    }

    #[test]
    fn test_resolve_ui_path_fallback() {
        let path = resolve_ui_path();
        assert!(path.ends_with("ui/shell.qml"));
    }
}
