mod editor;
mod exif_inspector;
mod ipc;
mod scanner;
mod theme;
mod trash;

use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::signal::unix::{signal, SignalKind};
use tokio::sync::{mpsc, Mutex};

use editor::ImageEditor;
use ipc::{run_ipc_server, AppState};
use scanner::DirectoryScanner;
use theme::ThemeManager;
use trash::TrashManager;

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
        return cwd_ui
            .canonicalize()
            .unwrap_or_else(|_| cwd_ui.to_path_buf());
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

    // e. exe_dir/../share/zii/ui/shell.qml (covers any PREFIX)
    let exe_share_ui = exe_dir.join("../share/zii/ui/shell.qml");
    if exe_share_ui.exists() {
        return exe_share_ui.canonicalize().unwrap_or(exe_share_ui);
    }

    // f. ~/.local/share/zii/ui/shell.qml
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

    // g. Iterate XDG_DATA_DIRS (default /usr/local/share:/usr/share) checking <dir>/zii/ui/shell.qml
    let data_dirs = std::env::var("XDG_DATA_DIRS")
        .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
    if let Some(found) = find_ui_in_data_dirs(&data_dirs) {
        return found;
    }

    // h. compile-time development fallback
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/ui/shell.qml"))
}

/// Walk a colon-separated XDG_DATA_DIRS list and return the first
/// `<dir>/zii/ui/shell.qml` that exists.
fn find_ui_in_data_dirs(data_dirs: &str) -> Option<PathBuf> {
    data_dirs
        .split(':')
        .filter(|dir| !dir.is_empty())
        .map(|dir| Path::new(dir).join("zii/ui/shell.qml"))
        .find(|candidate| candidate.exists())
        .map(|candidate| candidate.canonicalize().unwrap_or(candidate))
}

#[derive(Debug, PartialEq, Eq)]
pub struct CliConfig {
    pub show_help: bool,
    pub show_version: bool,
    pub fullscreen: bool,
    pub targets: Vec<PathBuf>,
}

pub fn parse_cli_args<I, T>(args: I) -> CliConfig
where
    I: IntoIterator<Item = T>,
    T: AsRef<str>,
{
    let mut show_help = false;
    let mut show_version = false;
    let mut fullscreen = false;
    let mut targets = Vec::new();

    let mut iter = args.into_iter();
    let _bin_name = iter.next(); // Skip program name if present

    for arg in iter {
        let s = arg.as_ref();
        match s {
            "-h" | "--help" => show_help = true,
            "-v" | "--version" => show_version = true,
            "-f" | "--fullscreen" => fullscreen = true,
            other => {
                if !other.is_empty() {
                    targets.push(PathBuf::from(other));
                }
            }
        }
    }

    CliConfig {
        show_help,
        show_version,
        fullscreen,
        targets,
    }
}

fn print_help() {
    print!(
        "Zii {} - Fast, minimalist Wayland photo viewer & editor for Omarchy\n\n",
        env!("CARGO_PKG_VERSION")
    );
    print!(
        r#"USAGE:
    zii [OPTIONS] [PATH]...

ARGS:
    <PATH>...    Image file(s) or directory to open [default: .]

OPTIONS:
    -f, --fullscreen    Start in fullscreen mode
    -h, --help          Print help information
    -v, --version       Print version information

KEYBINDINGS (Normal Mode):
    h, Left             Previous image
    l, Right            Next image
    j, Down             Pan down
    k, Up               Pan up
    +, =, z             Zoom in
    -, _, Z             Zoom out
    0                   Reset zoom & fit to window
    1                   100% (1:1 pixel scale)
    f, F11              Toggle fullscreen
    Space               Play / pause animated GIF/WebP
    i                   Enter Edit Mode
    y                   Copy image to clipboard (wl-copy)
    Y                   Copy image path to clipboard
    W                   Set as Omarchy desktop wallpaper
    e, x                Toggle EXIF metadata inspector
    dd                  Move to trash
    Shift+D             Permanently delete
    u                   Restore from trash / Undo
    ?                   Toggle keyboard shortcuts help
    q, Esc              Quit

KEYBINDINGS (Edit Mode):
    c                   Interactive crop tool (0-4 aspect ratios, Enter to apply)
    r / R               Rotate 90° clockwise / counter-clockwise
    h / v               Flip horizontal / vertical
    a                   Adjustments panel (Brightness / Contrast / Saturation)
    Tab / Shift+Tab     Cycle active adjustment slider (also Down / Up)
    [ / ]               Decrease / increase active adjustment slider
    u / Ctrl+r          Undo / Redo edit step
    w                   Save and overwrite original
    s                   Save copy dialog
    Esc                 Cancel tool / Exit Edit Mode
"#
    );
}

async fn cleanup_app(socket_path: &Path, app_state: &Arc<Mutex<AppState>>) {
    let _ = std::fs::remove_file(socket_path);
    let st = app_state.lock().await;
    if let Ok(ed) = st.editor.lock() {
        ed.cleanup();
    }
    st.trash.cleanup();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let cli = parse_cli_args(&args);

    if cli.show_version {
        println!("zii {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if cli.show_help {
        print_help();
        return Ok(());
    }

    let scanner = DirectoryScanner::from_paths(&cli.targets)?;
    let watch_dir = scanner.current_dir.clone();
    let trash = TrashManager::new();
    let editor = ImageEditor::new();
    let theme = ThemeManager::new();

    let pid = std::process::id();
    let socket_path = get_socket_path(pid);

    let app_state = Arc::new(Mutex::new(AppState {
        scanner,
        trash,
        editor: Arc::new(std::sync::Mutex::new(editor)),
        editor_busy: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        edit_active: false,
        theme,
    }));

    let (theme_tx, theme_rx) = mpsc::channel(16);
    let (dir_tx, dir_rx) = mpsc::channel(16);
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);

    // Start Omarchy theme watcher
    if let Err(e) = ThemeManager::start_watcher(theme_tx) {
        eprintln!("Theme watcher warning: {:?}", e);
    }

    // Start live directory watcher
    if let Err(e) = DirectoryScanner::start_watcher(watch_dir, dir_tx) {
        eprintln!("Directory watcher warning: {:?}", e);
    }

    // Spawn IPC server
    let s_path = socket_path.clone();
    let s_state = app_state.clone();
    let s_shutdown_tx = shutdown_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = run_ipc_server(s_path, s_state, theme_rx, dir_rx, s_shutdown_tx).await {
            eprintln!("IPC Server error: {:?}", e);
        }
    });

    let ui_path = resolve_ui_path();

    println!("Starting Zii with {} targets", cli.targets.len());
    println!("Socket: {:?}", socket_path);
    println!("Loading UI: {:?}", ui_path);

    // Launch Quickshell child process using tokio::process
    let mut qs_cmd = tokio::process::Command::new("quickshell");
    qs_cmd.arg("-p").arg(&ui_path);
    qs_cmd.env("ZII_SOCKET", &socket_path);
    if cli.fullscreen {
        qs_cmd.env("ZII_FULLSCREEN", "1");
    }

    let mut sigterm = signal(SignalKind::terminate())?;
    let mut sighup = signal(SignalKind::hangup())?;

    let mut qs_child = match qs_cmd.spawn() {
        Ok(child) => child,
        Err(e) => {
            eprintln!("Failed to spawn quickshell: {:?}", e);
            cleanup_app(&socket_path, &app_state).await;
            return Err(e.into());
        }
    };

    // Wait for quickshell, shutdown_rx from IPC, or OS signals
    tokio::select! {
        _ = shutdown_rx.recv() => {
            println!("Received shutdown signal from UI.");
            let _ = qs_child.kill().await;
        }
        _ = tokio::signal::ctrl_c() => {
            println!("Received SIGINT (Ctrl+C).");
            let _ = qs_child.kill().await;
        }
        _ = sigterm.recv() => {
            println!("Received SIGTERM.");
            let _ = qs_child.kill().await;
        }
        _ = sighup.recv() => {
            println!("Received SIGHUP.");
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
    cleanup_app(&socket_path, &app_state).await;
    println!("Zii closed cleanly.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    static ENV_LOCK: StdMutex<()> = StdMutex::new(());

    #[test]
    fn test_resolve_ui_path_env_wins() {
        let _guard = ENV_LOCK.lock().unwrap();
        let temp = tempfile::NamedTempFile::new().unwrap();
        let temp_path = temp.path().to_path_buf();
        std::env::set_var("ZII_UI_PATH", &temp_path);
        let resolved = resolve_ui_path();
        std::env::remove_var("ZII_UI_PATH");
        assert_eq!(resolved, temp_path.canonicalize().unwrap_or(temp_path));
    }

    #[test]
    fn test_socket_path() {
        let pid = 999999;
        let socket = get_socket_path(pid);
        let s = socket.to_string_lossy();
        assert!(s.contains("zii_999999.sock"));
    }

    #[test]
    fn test_resolve_ui_path_fallback() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("ZII_UI_PATH");
        let path = resolve_ui_path();
        assert!(path.ends_with("ui/shell.qml"));
    }

    #[test]
    fn test_find_ui_in_data_dirs() {
        // Second entry has the UI, first is an empty dir, plus an empty segment
        // and a nonexistent dir to make sure they are skipped.
        let empty_share = tempfile::tempdir().unwrap();
        let good_share = tempfile::tempdir().unwrap();
        let qml_dir = good_share.path().join("zii/ui");
        std::fs::create_dir_all(&qml_dir).unwrap();
        let qml_path = qml_dir.join("shell.qml");
        std::fs::write(&qml_path, "import QtQuick\nItem {}\n").unwrap();

        let data_dirs = format!(
            "{}::/nonexistent/share:{}",
            empty_share.path().display(),
            good_share.path().display()
        );
        let found = find_ui_in_data_dirs(&data_dirs).expect("should find UI in second dir");
        assert_eq!(found, qml_path.canonicalize().unwrap());

        // Nothing matches -> None, so resolve_ui_path falls through to the next step.
        let none = find_ui_in_data_dirs(&format!(
            "{}:/nonexistent/share",
            empty_share.path().display()
        ));
        assert!(none.is_none());
    }

    #[test]
    fn test_parse_cli_args_help() {
        let args = vec!["zii", "--help"];
        let cfg = parse_cli_args(args);
        assert!(cfg.show_help);
        assert!(!cfg.fullscreen);
        assert!(!cfg.show_version);
    }

    #[test]
    fn test_parse_cli_args_version() {
        let args = vec!["zii", "-v"];
        let cfg = parse_cli_args(args);
        assert!(cfg.show_version);
        assert!(!cfg.show_help);
    }

    #[test]
    fn test_parse_cli_args_fullscreen_and_files() {
        let args = vec!["zii", "-f", "img1.png", "img2.jpg"];
        let cfg = parse_cli_args(args);
        assert!(cfg.fullscreen);
        assert_eq!(cfg.targets.len(), 2);
        assert_eq!(cfg.targets[0], PathBuf::from("img1.png"));
        assert_eq!(cfg.targets[1], PathBuf::from("img2.jpg"));
    }
}
