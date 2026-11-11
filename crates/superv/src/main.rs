mod config;
mod emoji;
mod gnome_ext;
mod ui;
mod watch;

use gtk4::{self as gtk, gio, glib, prelude::*};
use std::cell::OnceCell;
use std::rc::Rc;
use superv_core::{NewItem, Store, StoreConfig};

const APP_ID: &str = "io.superv.Clipboard";

fn main() -> glib::ExitCode {
    let app = gtk::Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    let ui: Rc<OnceCell<Rc<ui::Ui>>> = Rc::new(OnceCell::new());

    // Primary instance: open store, export actions, start watchers, stay resident.
    {
        let ui = ui.clone();
        app.connect_startup(move |app| {
            let cfg = config::load();
            let mut store = Store::open(
                config::db_path(),
                StoreConfig {
                    max_items: cfg.max_items,
                    ..Default::default()
                },
            )
            .expect("cannot open history database");
            if cfg.clear_unpinned_on_start {
                let _ = store.clear();
            }
            let u = ui::Ui::new(app, store, cfg);
            let _ = ui.set(u.clone());
            std::mem::forget(app.hold()); // run as a daemon with no window open

            let a = gio::SimpleAction::new("toggle", None);
            let uu = u.clone();
            a.connect_activate(move |_, _| uu.toggle());
            app.add_action(&a);

            // Used by the GNOME Shell extension.
            let a = gio::SimpleAction::new("add-text", Some(glib::VariantTy::STRING));
            let uu = u.clone();
            a.connect_activate(move |_, p| {
                if let Some(t) = p.and_then(|p| p.get::<String>()) {
                    uu.record(NewItem::text(t));
                }
            });
            app.add_action(&a);
            let a = gio::SimpleAction::new("add-image", Some(glib::VariantTy::BYTE_STRING));
            let uu = u.clone();
            a.connect_activate(move |_, p| {
                if let Some(b) = p.map(|p| {
                    p.fixed_array::<u8>()
                        .map(|s| s.to_vec())
                        .unwrap_or_default()
                }) {
                    uu.record(NewItem::image(b));
                }
            });
            app.add_action(&a);

            let uu = u.clone();
            watch::start(move |item| uu.record(item));

            // GNOME: install + enable the bundled Shell extension (needs one re-login).
            if gnome_ext::ensure() == gnome_ext::Outcome::NeedsRelogin {
                let n = gio::Notification::new("superv: one more step");
                n.set_body(Some("Log out and back in to finish setup. Then press Super+V to open your clipboard history."));
                app.send_notification(Some("setup"), &n);
            }
        });
    }

    // `superv --toggle` / `superv --daemon`; remote invocations are forwarded here.
    {
        let ui = ui.clone();
        app.connect_command_line(move |app, cl| {
            let args: Vec<String> = cl.arguments().iter().map(|a| a.to_string_lossy().into_owned()).collect();
            if args.iter().any(|a| a == "--help" || a == "-h") {
                println!("Usage: superv [--daemon | --toggle]\n  --daemon  start in the background\n  --toggle  show/hide the clipboard popup (bind this to a hotkey)");
                return 0.into();
            }
            if args.iter().any(|a| a == "--toggle") {
                if let Some(u) = ui.get() {
                    u.toggle();
                }
            }
            let _ = app;
            0.into()
        });
    }
    app.run()
}
