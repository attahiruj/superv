<p align="center">
  <img src="packaging/io.superv.Clipboard.svg" alt="superv icon" width="128" height="128">
</p>

<h1 align="center">superv</h1>

<p align="center">
  A Windows <code>Win+V</code>-style clipboard history for Linux, with an emoji and symbol picker.<br>
  Built with Rust and GTK4. Works on X11 and Wayland (GNOME, KDE, wlroots compositors).
</p>

<p align="center">
  <a href="https://github.com/attahiruj/superv/actions/workflows/ci.yml"><img src="https://github.com/attahiruj/superv/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT license"></a>
  <a href="https://github.com/attahiruj/superv/releases"><img src="https://img.shields.io/github/v/release/attahiruj/superv?include_prereleases" alt="Latest release"></a>
  <img src="https://img.shields.io/badge/Rust-GTK4-orange.svg" alt="Rust and GTK4">
</p>

## Features

- Clipboard history for text and images, shown in a popup opened with a hotkey
- Pin items so they survive pruning and "Clear all"
- Search history
- Emoji and symbol picker tab
- Duplicate copies move to the top instead of piling up
- Records everything you copy, including passwords. Only empty items and items over 4 MB are skipped
- Local only: history is a SQLite file on your machine
- Runs as a small background daemon that starts at login (or on demand via D-Bus)

## How capture works

| Environment | Mechanism |
|---|---|
| X11 | GDK clipboard change signal |
| wlroots / KDE Wayland | `wl-paste --watch` (needs `wl-clipboard`) |
| GNOME Wayland | Bundled Shell extension (`gnome-extension/`) pushes changes to the daemon over D-Bus, because Mutter has no data-control protocol. It also provides the global shortcut and synthesizes `Ctrl+V` for pasting. |

## Build requirements

- Rust (stable) and Cargo
- GTK 4.14+ and [gtk4-layer-shell](https://github.com/wmww/gtk4-layer-shell) development files
- `wl-clipboard` on non-GNOME Wayland
- systemd (user session), only for `install.sh`

Package names (approximate):

| Distro | Packages |
|---|---|
| Debian/Ubuntu | `libgtk-4-dev libgtk4-layer-shell-dev` (layer-shell may need a newer release than your distro ships) |
| Fedora | `gtk4-devel gtk4-layer-shell-devel` |
| Arch | `gtk4 gtk4-layer-shell` |

## Install

### Ubuntu / Debian (.deb)

Download the latest `.deb` from the [releases page](https://github.com/attahiruj/superv/releases) and open it with App Center / Software, or:

```sh
sudo apt install ./superv_*_amd64.deb
```

That's all. superv starts at login. On GNOME it installs and enables its Shell extension the first time it runs and asks you to **log out and back in once**; after that `Super+V` opens the history.

### Snap / Flatpak

Manifests live in `packaging/snap` and `packaging/flatpak`. They are drafts and have not been built or tested yet.

### From source

```sh
git clone https://github.com/attahiruj/superv.git
cd superv
./install.sh
```

This is for development; it installs the binary with `cargo install`, installs the desktop file, D-Bus service and systemd user unit, starts the daemon, and on GNOME installs and enables the Shell extension (you may need to log out and in first).

Read `install.sh` before running it; it writes to `~/.local`, `~/.config/systemd/user` and `~/.cargo/bin`.

## Usage

After installing from a package, superv starts at login and records in the background. On GNOME, log out and back in once after the first run so the Shell extension loads.

**Open the history:** press the hotkey.

- **GNOME:** `Super+V`. While superv's extension is enabled it takes this key over from GNOME's notification tray (restored if you disable the extension). To change it:

  ```sh
  GSETTINGS_SCHEMA_DIR=~/.local/share/gnome-shell/extensions/superv@hiro/schemas \
    gsettings set org.gnome.shell.extensions.superv toggle "['<Super><Shift>v']"
  ```
- **Other desktops:** bind `superv --toggle` to a hotkey in your compositor or desktop settings, e.g. in Sway: `bindsym $mod+Shift+v exec superv --toggle`.

**In the popup:**

- Click an item to copy it back to the clipboard and close the popup. If `paste_command` is set (default on GNOME), it is also pasted into the previously focused app.
- Use the pin button to keep an item; pinned items survive the item limit and "Clear all".
- Type in the search box to filter, use the pin toggle next to it to show only pinned items, and the delete button on an item to remove it.
- **Clear all** removes every unpinned item.
- The **Emoji** tab searches emoji and symbols; picking one copies it.
- `Esc` or clicking away closes the popup.

**Command line:**

```
superv --daemon   start in the background (normally automatic)
superv --toggle   show or hide the popup
superv --help     show usage
```

## Configuration

Optional file at `~/.config/superv/config.toml`:

```toml
# Max unpinned items to keep (default 25)
max_items = 25

# Delete unpinned history at startup (default false). Pinned items always persist.
clear_unpinned_on_start = false

# Command run after selecting an item, to paste into the previously focused app.
# Defaults to the GNOME extension's paste method on GNOME, unset elsewhere
# (so items are only copied, and you paste with Ctrl+V yourself).
# Examples: "wtype -M ctrl v -m ctrl" (wlroots), "ydotool key 29:1 47:1 47:0 29:0"
paste_command = "wtype -M ctrl v -m ctrl"
```

History is stored at `~/.local/share/superv/history.db`.

## Project layout

```
crates/superv-core   Store (SQLite), item model, recording filters. No GUI deps; unit tested.
crates/superv        GTK4 app: UI, clipboard watchers, config, emoji data
gnome-extension      GNOME Shell extension (clipboard bridge, hotkey, paste)
data                 Dev-install desktop entry, D-Bus service and systemd unit (used by install.sh)
packaging            .deb assets, icon, AppStream metadata, draft Snap and Flatpak manifests
```

## Development

```sh
cargo build
cargo test -p superv-core      # no GTK needed
cargo fmt --all
cargo clippy --workspace --all-targets
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full workflow.

## Contributing

Contributions are welcome: bug reports, compositor testing reports, features and docs.
Please read [CONTRIBUTING.md](CONTRIBUTING.md) and the [Code of Conduct](CODE_OF_CONDUCT.md).
Report security issues as described in [SECURITY.md](SECURITY.md).

## License

[MIT](LICENSE)
