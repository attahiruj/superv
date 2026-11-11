use crate::{config::Config, emoji, watch};
use gtk4::{self as gtk, gdk, glib, prelude::*};
use gtk4_layer_shell::{KeyboardMode, Layer, LayerShell};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use superv_core::{Content, Item, NewItem, Store};

pub struct Ui {
    pub window: gtk::ApplicationWindow,
    store: RefCell<Store>,
    cfg: Config,
    list: gtk::ListBox,
    search: gtk::SearchEntry,
    pinned_only: gtk::ToggleButton,
    shown_once: Cell<bool>,
    /// Text we just put on the clipboard ourselves (emoji picks) that must not be recorded in history.
    suppress: RefCell<Option<(String, std::time::Instant)>>,
}

impl Ui {
    pub fn new(app: &gtk::Application, store: Store, cfg: Config) -> Rc<Self> {
        // The system theme may be 'notheme' (tiny widgets, odd title bar); use stock Adwaita sizing.
        if let Some(settings) = gtk::Settings::default() {
            settings.set_gtk_theme_name(Some("Adwaita"));
            settings.set_gtk_decoration_layout(Some(":minimize,close"));
        }
        let window = gtk::ApplicationWindow::builder()
            .application(app)
            .title("Clipboard")
            .default_width(480)
            .default_height(640)
            .resizable(true)
            .build();
        if gtk4_layer_shell::is_supported() {
            window.init_layer_shell();
            window.set_layer(Layer::Overlay);
            window.set_keyboard_mode(KeyboardMode::OnDemand);
        }

        let list = gtk::ListBox::builder()
            .selection_mode(gtk::SelectionMode::Single)
            .activate_on_single_click(true)
            .build();
        list.add_css_class("navigation-sidebar");
        let search = gtk::SearchEntry::builder()
            .placeholder_text("Search history")
            .hexpand(true)
            .build();

        let pinned_only = gtk::ToggleButton::builder()
            .icon_name("view-pin-symbolic")
            .tooltip_text("Show pinned only")
            .build();
        let ui = Rc::new(Self {
            window,
            store: RefCell::new(store),
            cfg,
            list,
            search,
            pinned_only,
            shown_once: Cell::new(false),
            suppress: RefCell::new(None),
        });
        ui.build_layout();
        ui.refresh();
        ui
    }

    fn build_layout(self: &Rc<Self>) {
        let stack = gtk::Stack::new();
        stack.add_titled(&self.clipboard_page(), Some("clip"), "Clipboard");
        stack.add_titled(&self.emoji_page(), Some("emoji"), "Emoji");
        let switcher = gtk::StackSwitcher::builder()
            .stack(&stack)
            .halign(gtk::Align::Center)
            .margin_top(8)
            .build();
        let root = gtk::Box::new(gtk::Orientation::Vertical, 6);
        root.append(&switcher);
        root.append(&stack);
        self.window.set_child(Some(&root));

        // Esc closes; closing hides instead of quitting.
        let keys = gtk::EventControllerKey::new();
        let w = self.window.clone();
        keys.connect_key_pressed(move |_, key, _, _| {
            if key == gdk::Key::Escape {
                w.set_visible(false);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
        self.window.add_controller(keys);
        self.window.connect_close_request(|w| {
            w.set_visible(false);
            glib::Propagation::Stop
        });
        // Dismiss on focus loss, like the Windows popup.
        let me = self.clone();
        self.window.connect_is_active_notify(move |w| {
            if me.shown_once.get() && !w.is_active() {
                w.set_visible(false);
            }
        });
    }

    fn clipboard_page(self: &Rc<Self>) -> gtk::Box {
        let clear = gtk::Button::with_label("Clear all");
        let me = self.clone();
        clear.connect_clicked(move |_| {
            let _ = me.store.borrow_mut().clear();
            me.refresh();
        });
        let top = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        top.set_margin_start(8);
        top.set_margin_end(8);
        top.append(&self.search);
        top.append(&self.pinned_only);
        top.append(&clear);

        let me = self.clone();
        self.search
            .connect_search_changed(move |_| me.list.invalidate_filter());
        let me = self.clone();
        self.pinned_only
            .connect_toggled(move |_| me.list.invalidate_filter());
        let me = self.clone();
        self.list.set_filter_func(move |row| {
            if me.pinned_only.is_active() {
                let pinned = unsafe {
                    row.data::<bool>("superv-pinned")
                        .map(|p| *p.as_ref())
                        .unwrap_or(false)
                };
                if !pinned {
                    return false;
                }
            }
            let q = me.search.text().to_lowercase();
            q.is_empty() || row.widget_name().to_lowercase().contains(&q)
        });
        let me = self.clone();
        self.list.connect_row_activated(move |_, row| {
            let id: i64 = unsafe {
                row.data::<i64>("superv-id")
                    .map(|p| *p.as_ref())
                    .unwrap_or(-1)
            };
            me.paste_id(id);
        });

        let scroll = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&self.list)
            .build();
        let page = gtk::Box::new(gtk::Orientation::Vertical, 6);
        page.append(&top);
        page.append(&scroll);
        page
    }

    fn emoji_page(self: &Rc<Self>) -> gtk::Box {
        let entries: Rc<Vec<emoji::Entry>> = Rc::new(
            emoji::symbols()
                .into_iter()
                .chain(emoji::emojis())
                .collect(),
        );
        let flow = gtk::FlowBox::builder()
            .selection_mode(gtk::SelectionMode::None)
            .max_children_per_line(8)
            .min_children_per_line(8)
            .activate_on_single_click(true)
            .build();
        for e in entries.iter() {
            let b = gtk::Label::new(Some(&e.glyph));
            b.add_css_class("title-3");
            b.set_tooltip_text(Some(&e.name));
            b.set_margin_top(4);
            b.set_margin_bottom(4);
            let child = gtk::FlowBoxChild::new();
            child.set_child(Some(&b));
            child.set_widget_name(&e.search);
            flow.insert(&child, -1);
        }
        let search = gtk::SearchEntry::builder()
            .placeholder_text("Search emoji and symbols")
            .margin_start(8)
            .margin_end(8)
            .build();
        let f2 = flow.clone();
        search.connect_search_changed(move |_| f2.invalidate_filter());
        let s2 = search.clone();
        flow.set_filter_func(move |c| {
            let q = s2.text().to_lowercase();
            q.is_empty() || c.widget_name().contains(&q)
        });
        let me = self.clone();
        let ents = entries.clone();
        flow.connect_child_activated(move |_, child| {
            let idx = child.index() as usize;
            if let Some(e) = ents.get(idx) {
                *me.suppress.borrow_mut() = Some((e.glyph.clone(), std::time::Instant::now()));
                me.set_and_close(Source::Text(e.glyph.clone()));
            }
        });
        let scroll = gtk::ScrolledWindow::builder()
            .vexpand(true)
            .hscrollbar_policy(gtk::PolicyType::Never)
            .child(&flow)
            .build();
        let page = gtk::Box::new(gtk::Orientation::Vertical, 6);
        page.append(&search);
        page.append(&scroll);
        page
    }

    /// Rebuild the history list from the store.
    pub fn refresh(self: &Rc<Self>) {
        while let Some(c) = self.list.first_child() {
            self.list.remove(&c);
        }
        let items = self.store.borrow().list().unwrap_or_default();
        if items.is_empty() {
            let l = gtk::Label::new(Some("Nothing copied yet"));
            l.set_margin_top(40);
            l.add_css_class("dim-label");
            let row = gtk::ListBoxRow::builder()
                .child(&l)
                .activatable(false)
                .selectable(false)
                .build();
            self.list.append(&row);
        }
        for item in items {
            self.list.append(&self.row(&item));
        }
    }

    fn row(self: &Rc<Self>, item: &Item) -> gtk::ListBoxRow {
        let hb = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        hb.set_margin_top(4);
        hb.set_margin_bottom(4);
        hb.set_margin_start(6);
        hb.set_margin_end(6);
        let search_text: String = match &item.content {
            Content::Text(t) => {
                let preview: String = t.lines().take(3).collect::<Vec<_>>().join("\n");
                let l = gtk::Label::builder()
                    .label(&preview)
                    .xalign(0.0)
                    .hexpand(true)
                    .ellipsize(gtk::pango::EllipsizeMode::End)
                    .lines(3)
                    .wrap(true)
                    .wrap_mode(gtk::pango::WrapMode::WordChar)
                    .max_width_chars(44)
                    .build();
                hb.append(&l);
                t.clone()
            }
            Content::Image(png) => {
                let bytes = glib::Bytes::from(png.as_slice());
                match gdk::Texture::from_bytes(&bytes) {
                    Ok(tex) => {
                        let pic = gtk::Picture::for_paintable(&tex);
                        pic.set_can_shrink(true);
                        pic.set_content_fit(gtk::ContentFit::Contain);
                        pic.set_size_request(-1, 140);
                        pic.set_hexpand(true);
                        hb.append(&pic);
                    }
                    Err(_) => hb.append(&gtk::Label::new(Some("[image]"))),
                }
                "image".into()
            }
        };
        let id = item.id;
        let pin = gtk::ToggleButton::builder()
            .icon_name("view-pin-symbolic")
            .active(item.pinned)
            .valign(gtk::Align::Center)
            .tooltip_text("Pin")
            .build();
        pin.add_css_class("flat");
        let me = self.clone();
        pin.connect_clicked(move |b| {
            let _ = me.store.borrow_mut().set_pinned(id, b.is_active());
            let me = me.clone();
            glib::idle_add_local_once(move || me.refresh());
        });
        let del = gtk::Button::builder()
            .icon_name("edit-delete-symbolic")
            .valign(gtk::Align::Center)
            .tooltip_text("Delete")
            .build();
        del.add_css_class("flat");
        let me = self.clone();
        del.connect_clicked(move |_| {
            let _ = me.store.borrow_mut().delete(id);
            let me = me.clone();
            glib::idle_add_local_once(move || me.refresh());
        });
        hb.append(&pin);
        hb.append(&del);
        // Row id carries the item id; widget name carries searchable text.
        let row = gtk::ListBoxRow::builder().child(&hb).build();
        row.set_widget_name(&search_text);
        unsafe {
            row.set_data("superv-id", id);
            row.set_data("superv-pinned", item.pinned);
        }
        row
    }

    fn paste_id(self: &Rc<Self>, id: i64) {
        let Some(item) = self.store.borrow().get(id).ok().flatten() else {
            return;
        };
        self.set_and_close(match item.content {
            Content::Text(t) => Source::Text(t),
            Content::Image(p) => Source::Png(p),
        });
        // Bump to top.
        let it = NewItem {
            content: self
                .store
                .borrow()
                .get(id)
                .ok()
                .flatten()
                .map(|i| i.content)
                .unwrap(),
            mimes: vec![],
        };
        let _ = self.store.borrow_mut().add(&it);
    }

    fn set_and_close(self: &Rc<Self>, src: Source) {
        if let Some(display) = gdk::Display::default() {
            let cb = display.clipboard();
            match src {
                Source::Text(t) => cb.set_text(&t),
                Source::Png(p) => {
                    if let Ok(tex) = gdk::Texture::from_bytes(&glib::Bytes::from(p.as_slice())) {
                        cb.set_texture(&tex);
                    }
                }
            }
        }
        self.window.set_visible(false);
        if let Some(cmd) = &self.cfg.paste_command {
            watch::auto_paste(cmd);
        }
    }

    /// Called for every observed clipboard change.
    pub fn record(self: &Rc<Self>, item: NewItem) {
        // Drop the echo of our own emoji write (watchers can't tell it from a real copy).
        if let superv_core::Content::Text(t) = &item.content {
            let mut sup = self.suppress.borrow_mut();
            if let Some((s, at)) = sup.as_ref() {
                if at.elapsed() < std::time::Duration::from_secs(3) && s == t {
                    *sup = None;
                    return;
                }
            }
        }
        if self.store.borrow_mut().add(&item).is_ok() {
            self.refresh();
        }
    }

    pub fn toggle(self: &Rc<Self>) {
        if self.window.is_visible() {
            self.window.set_visible(false);
        } else {
            self.refresh();
            self.search.set_text("");
            self.window.present();
            self.shown_once.set(true);
        }
    }
}

enum Source {
    Text(String),
    Png(Vec<u8>),
}
