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
    let socket_path = std::env::temp_dir().join(format!("zii_{}.sock", pid));

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

    // Determine path to Quickshell UI
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    // Check UI search paths:
    // 1. ZII_UI_PATH env var
    // 2. Relative to working dir: ./ui/shell.qml
    // 3. Relative to binary: ../ui/shell.qml or share/zii/ui/shell.qml
    // 4. Source dir
    let ui_path = if let Ok(custom) = std::env::var("ZII_UI_PATH") {
        PathBuf::from(custom)
    } else if Path::new("ui/shell.qml").exists() {
        PathBuf::from("ui/shell.qml").canonicalize().unwrap_or_else(|_| PathBuf::from("ui/shell.qml"))
    } else if exe_dir.join("../ui/shell.qml").exists() {
        exe_dir.join("../ui/shell.qml").canonicalize().unwrap_or_else(|_| exe_dir.join("../ui/shell.qml"))
    } else if exe_dir.join("ui/shell.qml").exists() {
        exe_dir.join("ui/shell.qml").canonicalize().unwrap_or_else(|_| exe_dir.join("ui/shell.qml"))
    } else {
        PathBuf::from("/home/parsa/Work/zi/ui/shell.qml")
    };

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

    // Cleanup socket
    let _ = std::fs::remove_file(&socket_path);
    println!("Zii closed cleanly.");

    Ok(())
}
