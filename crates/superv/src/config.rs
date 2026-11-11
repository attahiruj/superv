use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Max unpinned items kept (Windows keeps 25).
    pub max_items: usize,
    /// Delete unpinned history at startup (pinned items always persist).
    pub clear_unpinned_on_start: bool,
    /// Optional command run after selecting an item to paste it into the
    /// previously focused app, e.g. "wtype -M ctrl v -m ctrl" or "ydotool key 29:1 47:1 47:0 29:0".
    pub paste_command: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        // On GNOME the shell extension synthesizes Ctrl+V (no other unprivileged way on Wayland).
        let gnome = std::env::var("XDG_CURRENT_DESKTOP")
            .map(|d| d.to_lowercase().contains("gnome"))
            .unwrap_or(false);
        let paste_command = gnome.then(|| {
            "gdbus call --session --dest io.superv.Bridge --object-path /io/superv/Bridge --method io.superv.Bridge.Paste".to_string()
        });
        Self {
            max_items: 25,
            clear_unpinned_on_start: false,
            paste_command,
        }
    }
}

pub fn config_path() -> PathBuf {
    gtk4::glib::user_config_dir()
        .join("superv")
        .join("config.toml")
}

pub fn db_path() -> PathBuf {
    let dir = gtk4::glib::user_data_dir().join("superv");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("history.db")
}

pub fn load() -> Config {
    match std::fs::read_to_string(config_path()) {
        Ok(s) => toml::from_str(&s).unwrap_or_else(|e| {
            eprintln!("superv: bad config, using defaults: {e}");
            Config::default()
        }),
        Err(_) => Config::default(),
    }
}
