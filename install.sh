#!/bin/sh
# Install superv for the current user.
set -e
PATH="$HOME/.cargo/bin:$PATH"
cd "$(dirname "$0")"
cargo install --path crates/superv --locked
mkdir -p ~/.local/share/applications ~/.local/share/dbus-1/services ~/.config/systemd/user
cp data/io.superv.Clipboard.desktop ~/.local/share/applications/
sed "s|Exec=superv|Exec=$HOME/.cargo/bin/superv|" data/io.superv.Clipboard.service > ~/.local/share/dbus-1/services/io.superv.Clipboard.service
cp data/superv.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now superv.service
if echo "$XDG_CURRENT_DESKTOP" | grep -qi gnome; then
  d=~/.local/share/gnome-shell/extensions/superv@hiro
  mkdir -p "$d"; cp -r gnome-extension/* "$d"/
  glib-compile-schemas "$d/schemas"
  gnome-extensions enable superv@hiro || echo "Log out/in, then: gnome-extensions enable superv@hiro"
fi
echo "Done. Bind 'superv --toggle' to a hotkey on non-GNOME desktops."
