//! Clipboard change sources.
//!
//! * X11: GDK's clipboard `changed` signal works for background apps.
//! * wlroots/KDE Wayland: `wl-paste --watch` (wlr-data-control).
//! * GNOME Wayland: the bundled Shell extension calls the `add-text` /
//!   `add-image` app actions over D-Bus (Mutter has no data-control).
use gtk4::{gdk, glib, prelude::*};
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use superv_core::NewItem;

/// Start every applicable watcher; each delivers items to `on_item` on the main thread.
pub fn start(on_item: impl Fn(NewItem) + 'static + Clone) {
    let desktop = std::env::var("XDG_CURRENT_DESKTOP")
        .unwrap_or_default()
        .to_uppercase();
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let gnome = desktop.contains("GNOME");

    // GDK path: reliable on X11; on Wayland it only fires while focused (harmless).
    if let Some(display) = gdk::Display::default() {
        let cb = display.clipboard();
        let f = on_item.clone();
        cb.connect_changed(move |cb| {
            if cb.is_local() {
                return; // our own write
            }
            let cb = cb.clone();
            let f = f.clone();
            glib::spawn_future_local(async move {
                if let Some(item) = read_gdk(&cb).await {
                    f(item);
                }
            });
        });
    }

    if wayland && !gnome && have("wl-paste") {
        let (tx, rx) = async_channel::unbounded::<NewItem>();
        std::thread::spawn(move || wl_paste_loop(tx));
        glib::spawn_future_local(async move {
            while let Ok(item) = rx.recv().await {
                on_item(item);
            }
        });
    } else if wayland && gnome {
        eprintln!("superv: GNOME Wayland detected - enable the 'superv@hiro' Shell extension for background capture");
    }
}

async fn read_gdk(cb: &gdk::Clipboard) -> Option<NewItem> {
    let formats = cb.formats();
    let mimes: Vec<String> = formats.mime_types().iter().map(|m| m.to_string()).collect();
    let mut item = if formats.contains_type(String::static_type()) {
        let t = cb.read_text_future().await.ok().flatten()?;
        NewItem::text(t.to_string())
    } else if formats.contains_type(gdk::Texture::static_type()) {
        let tex = cb.read_texture_future().await.ok().flatten()?;
        NewItem::image(tex.save_to_png_bytes().to_vec())
    } else {
        return None;
    };
    item.mimes = mimes;
    Some(item)
}

fn have(bin: &str) -> bool {
    std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).any(|d| d.join(bin).is_file()))
        .unwrap_or(false)
}

fn wl_paste_loop(tx: async_channel::Sender<NewItem>) {
    // `wl-paste --watch echo` prints one line per clipboard change.
    let Ok(mut child) = Command::new("wl-paste")
        .args(["--watch", "echo"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return;
    };
    let out = child.stdout.take().unwrap();
    for _ in BufReader::new(out).lines() {
        if let Some(item) = wl_read() {
            if tx.send_blocking(item).is_err() {
                break;
            }
        }
    }
}

fn wl_read() -> Option<NewItem> {
    let types = Command::new("wl-paste").arg("--list-types").output().ok()?;
    let types: Vec<String> = String::from_utf8_lossy(&types.stdout)
        .lines()
        .map(|s| s.to_string())
        .collect();
    let get = |t: &str| {
        Command::new("wl-paste")
            .args(["--no-newline", "--type", t])
            .output()
            .ok()
            .map(|o| o.stdout)
    };
    let mut item = if types.iter().any(|t| t.starts_with("text/plain")) {
        let ty = types.iter().find(|t| t.starts_with("text/plain")).unwrap();
        NewItem::text(String::from_utf8_lossy(&get(ty)?).into_owned())
    } else if types.iter().any(|t| t == "image/png") {
        NewItem::image(get("image/png")?)
    } else {
        return None;
    };
    item.mimes = types;
    Some(item)
}

/// Run the optional auto-paste command shortly after the popup closes.
pub fn auto_paste(cmd: &str) {
    let cmd = cmd.to_string();
    glib::timeout_add_local_once(std::time::Duration::from_millis(180), move || {
        let _ = Command::new("sh").arg("-c").arg(&cmd).spawn();
    });
}
