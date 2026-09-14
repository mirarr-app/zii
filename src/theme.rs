use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use notify::{Watcher, RecursiveMode, Event};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmarchyTheme {
    pub name: String,
    pub background: String,
    pub dark_background: String,
    pub darker_background: String,
    pub lighter_background: String,
    pub foreground: String,
    pub dark_foreground: String,
    pub light_foreground: String,
    pub bright_foreground: String,
    pub accent: String,
    pub selection: String,
    pub muted: String,
    pub red: String,
    pub yellow: String,
    pub green: String,
    pub cyan: String,
    pub blue: String,
    pub magenta: String,
}

impl Default for OmarchyTheme {
    fn default() -> Self {
        Self {
            name: "Default Dark".to_string(),
            background: "#1e1e2e".to_string(),
            dark_background: "#181825".to_string(),
            darker_background: "#11111b".to_string(),
            lighter_background: "#313244".to_string(),
            foreground: "#cdd6f4".to_string(),
            dark_foreground: "#6c7086".to_string(),
            light_foreground: "#bac2de".to_string(),
            bright_foreground: "#ffffff".to_string(),
            accent: "#89b4fa".to_string(),
            selection: "#45475a".to_string(),
            muted: "#585b70".to_string(),
            red: "#f38ba8".to_string(),
            yellow: "#f9e2af".to_string(),
            green: "#a6e3a1".to_string(),
            cyan: "#94e2d5".to_string(),
            blue: "#89b4fa".to_string(),
            magenta: "#f5c2e7".to_string(),
        }
    }
}

pub struct ThemeManager {
    pub current_theme: OmarchyTheme,
    state_theme_dir: PathBuf,
}

impl ThemeManager {
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/parsa".to_string());
        let state_theme_dir = PathBuf::from(home)
            .join(".local/state/omarchy/current/theme");

        let mut mgr = Self {
            current_theme: OmarchyTheme::default(),
            state_theme_dir,
        };
        mgr.reload();
        mgr
    }

    pub fn reload(&mut self) {
        let colors_path = self.state_theme_dir.join("colors.toml");
        let name_path = self.state_theme_dir.parent().map(|p| p.join("theme.name"));

        let theme_name = if let Some(np) = name_path {
            std::fs::read_to_string(np)
                .unwrap_or_else(|_| "Omarchy".to_string())
                .trim()
                .to_string()
        } else {
            "Omarchy".to_string()
        };

        if let Ok(content) = std::fs::read_to_string(&colors_path) {
            if let Ok(val) = content.parse::<toml::Value>() {
                let get_str = |key: &str, def: &str| -> String {
                    val.get(key)
                        .and_then(|v| v.as_str())
                        .unwrap_or(def)
                        .to_string()
                };

                self.current_theme = OmarchyTheme {
                    name: theme_name,
                    background: get_str("background", &self.current_theme.background),
                    dark_background: get_str("dark_background", &self.current_theme.dark_background),
                    darker_background: get_str("darker_background", &self.current_theme.darker_background),
                    lighter_background: get_str("lighter_background", &self.current_theme.lighter_background),
                    foreground: get_str("foreground", &self.current_theme.foreground),
                    dark_foreground: get_str("dark_foreground", &self.current_theme.dark_foreground),
                    light_foreground: get_str("light_foreground", &self.current_theme.light_foreground),
                    bright_foreground: get_str("bright_foreground", &self.current_theme.bright_foreground),
                    accent: get_str("accent", &self.current_theme.accent),
                    selection: get_str("selection", &self.current_theme.selection),
                    muted: get_str("muted", &self.current_theme.muted),
                    red: get_str("red", &self.current_theme.red),
                    yellow: get_str("yellow", &self.current_theme.yellow),
                    green: get_str("green", &self.current_theme.green),
                    cyan: get_str("cyan", &self.current_theme.cyan),
                    blue: get_str("blue", &self.current_theme.blue),
                    magenta: get_str("magenta", &self.current_theme.magenta),
                };
            }
        }
    }

    pub fn start_watcher(tx: mpsc::Sender<OmarchyTheme>) -> anyhow::Result<()> {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home/parsa".to_string());
        let watch_dir = PathBuf::from(home).join(".local/state/omarchy/current");

        if !watch_dir.exists() {
            return Ok(());
        }

        tokio::task::spawn_blocking(move || {
            let (notify_tx, notify_rx) = std::sync::mpsc::channel();
            let mut watcher = match notify::recommended_watcher(move |res: Result<Event, notify::Error>| {
                if let Ok(evt) = res {
                    let _ = notify_tx.send(evt);
                }
            }) {
                Ok(w) => w,
                Err(e) => {
                    eprintln!("Theme watcher error: {:?}", e);
                    return;
                }
            };

            if let Err(e) = watcher.watch(&watch_dir, RecursiveMode::Recursive) {
                eprintln!("Failed to watch Omarchy theme dir: {:?}", e);
                return;
            }

            let mut theme_mgr = ThemeManager::new();
            while let Ok(_event) = notify_rx.recv() {
                // Debounce slightly
                std::thread::sleep(std::time::Duration::from_millis(50));
                while notify_rx.try_recv().is_ok() {}

                theme_mgr.reload();
                let _ = tx.blocking_send(theme_mgr.current_theme.clone());
            }
        });

        Ok(())
    }
}
