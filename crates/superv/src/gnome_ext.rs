//! First-run setup of the bundled GNOME Shell extension.
//!
//! GNOME Wayland needs the extension for background clipboard capture, the global
//! hotkey and paste. Sandboxed formats (Flatpak/Snap) cannot write to system
//! directories, so every package format relies on this one mechanism: the app embeds
//! the extension, copies it to the user's extension directory, and enables it.
//! GNOME Shell only discovers new extensions at login, so a one-time re-login is needed.
use gtk4::{gio, glib, prelude::*};
use std::path::Path;
use std::process::Command;

pub const UUID: &str = "superv@hiro";

const FILES: &[(&str, &str)] = &[
    (
        "extension.js",
        include_str!("../../../gnome-extension/extension.js"),
    ),
    (
        "metadata.json",
        include_str!("../../../gnome-extension/metadata.json"),
    ),
    (
        "schemas/org.gnome.shell.extensions.superv.gschema.xml",
        include_str!(
            "../../../gnome-extension/schemas/org.gnome.shell.extensions.superv.gschema.xml"
        ),
    ),
];

#[derive(Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing to do (not GNOME, or already installed and enabled).
    Unchanged,
    /// Installed, updated or newly enabled: the user must log out and in once.
    NeedsRelogin,
}

pub fn is_gnome() -> bool {
    std::env::var("XDG_CURRENT_DESKTOP")
        .map(|d| d.to_lowercase().contains("gnome"))
        .unwrap_or(false)
}

/// Install/update the extension files and make sure it is in `enabled-extensions`.
pub fn ensure() -> Outcome {
    if !is_gnome() {
        return Outcome::Unchanged;
    }
    let dir = glib::user_data_dir()
        .join("gnome-shell")
        .join("extensions")
        .join(UUID);
    let mut changed = match write_files(&dir) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("superv: cannot install GNOME extension: {e}");
            return Outcome::Unchanged;
        }
    };
    if changed {
        // Needed for the user-configurable hotkey; harmless if the tool is missing.
        let _ = Command::new("glib-compile-schemas")
            .arg(dir.join("schemas"))
            .status();
    }
    changed |= enable();
    if changed {
        Outcome::NeedsRelogin
    } else {
        Outcome::Unchanged
    }
}

fn write_files(dir: &Path) -> std::io::Result<bool> {
    let mut changed = false;
    for (name, content) in FILES {
        let path = dir.join(name);
        if std::fs::read_to_string(&path)
            .map(|c| c == *content)
            .unwrap_or(false)
        {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, content)?;
        changed = true;
    }
    Ok(changed)
}

/// Append our UUID to org.gnome.shell `enabled-extensions`. Returns true if it was added.
fn enable() -> bool {
    let Some(source) = gio::SettingsSchemaSource::default() else {
        return false;
    };
    if source.lookup("org.gnome.shell", true).is_none() {
        return false;
    }
    let settings = gio::Settings::new("org.gnome.shell");
    let mut list: Vec<String> = settings
        .strv("enabled-extensions")
        .iter()
        .map(|s| s.to_string())
        .collect();
    if list.iter().any(|u| u == UUID) {
        return false;
    }
    list.push(UUID.to_string());
    let refs: Vec<&str> = list.iter().map(String::as_str).collect();
    settings.set_strv("enabled-extensions", refs).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_files_is_idempotent() {
        let dir = std::env::temp_dir().join(format!("superv-ext-{}", std::process::id()));
        assert!(write_files(&dir).unwrap());
        assert!(!write_files(&dir).unwrap());
        std::fs::write(dir.join("extension.js"), "stale").unwrap();
        assert!(write_files(&dir).unwrap());
        std::fs::remove_dir_all(dir).ok();
    }
}
