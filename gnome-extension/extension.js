import Clutter from 'gi://Clutter';
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import St from 'gi://St';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

const APP = 'io.superv.Clipboard';
const PATH = '/io/superv/Clipboard';
const BRIDGE_NAME = 'io.superv.Bridge';
const BRIDGE_PATH = '/io/superv/Bridge';
const BRIDGE_XML = `<node><interface name="io.superv.Bridge">
  <method name="Paste"/>
</interface></node>`;

export default class SupervBridge extends Extension {
    enable() {
        this._settings = this.getSettings();
        this._freeShortcut();
        this._clip = St.Clipboard.get_default();
        const sel = global.display.get_selection();
        this._sel = sel;
        this._id = sel.connect('owner-changed', (_s, type) => {
            if (type === Meta.SelectionType.SELECTION_CLIPBOARD)
                this._onChanged();
        });
        Main.wm.addKeybinding('toggle', this._settings, Meta.KeyBindingFlags.NONE,
            Shell.ActionMode.ALL, () => this._activate('toggle', null));
        // Paste(): synthesize Ctrl+V into the focused window (Wayland has no other unprivileged way).
        const seat = Clutter.get_default_backend().get_default_seat();
        this._kbd = seat.create_virtual_device(Clutter.InputDeviceType.KEYBOARD_DEVICE);
        this._dbus = Gio.DBusExportedObject.wrapJSObject(BRIDGE_XML, this);
        this._dbus.export(Gio.DBus.session, BRIDGE_PATH);
        this._owner = Gio.bus_own_name_on_connection(Gio.DBus.session, BRIDGE_NAME,
            Gio.BusNameOwnerFlags.REPLACE, null, null);
    }

    Paste() {
        const k = (key, state) => this._kbd.notify_keyval(GLib.get_monotonic_time(), key, state);
        k(Clutter.KEY_Control_L, Clutter.KeyState.PRESSED);
        k(Clutter.KEY_v, Clutter.KeyState.PRESSED);
        k(Clutter.KEY_v, Clutter.KeyState.RELEASED);
        k(Clutter.KEY_Control_L, Clutter.KeyState.RELEASED);
    }

    // GNOME binds Super+V to the notification tray by default; release it while we are enabled
    // (and only that accelerator), then restore it in disable().
    _freeShortcut() {
        try {
            const ks = new Gio.Settings({schema_id: 'org.gnome.shell.keybindings'});
            const mine = this._settings.get_strv('toggle');
            const cur = ks.get_strv('toggle-message-tray');
            const rest = cur.filter(a => !mine.includes(a));
            if (rest.length !== cur.length) {
                this._trayKs = ks;
                this._trayOrig = cur;
                ks.set_strv('toggle-message-tray', rest);
            }
        } catch (e) {
            console.error(`superv: cannot free shortcut: ${e}`);
        }
    }

    _restoreShortcut() {
        if (!this._trayKs)
            return;
        try {
            const now = this._trayKs.get_strv('toggle-message-tray');
            const back = this._trayOrig.filter(a => !now.includes(a));
            this._trayKs.set_strv('toggle-message-tray', [...now, ...back]);
        } catch (e) {
            console.error(`superv: cannot restore shortcut: ${e}`);
        }
        this._trayKs = null;
        this._trayOrig = null;
    }

    disable() {
        this._restoreShortcut();
        if (this._owner) Gio.bus_unown_name(this._owner);
        this._owner = null;
        if (this._dbus) this._dbus.unexport();
        this._dbus = null;
        this._kbd = null;
        Main.wm.removeKeybinding('toggle');
        if (this._id) this._sel.disconnect(this._id);
        this._id = null;
        this._settings = null;
        this._clip = null;
    }

    _onChanged() {
        const mimes = this._sel.get_mimetypes(Meta.SelectionType.SELECTION_CLIPBOARD)
            .map(m => m.toLowerCase());
        const T = St.ClipboardType.CLIPBOARD;
        if (mimes.some(m => m.startsWith('text/plain') || m === 'utf8_string')) {
            this._clip.get_text(T, (_c, text) => {
                if (text) this._activate('add-text', new GLib.Variant('s', text));
            });
        } else if (mimes.includes('image/png')) {
            this._clip.get_content(T, 'image/png', (_c, bytes) => {
                if (bytes && bytes.get_size() > 0)
                    this._activate('add-image', GLib.Variant.new_from_bytes(
                        new GLib.VariantType('ay'), bytes, true));
            });
        }
    }

    // org.gtk.Actions.Activate on the superv GApplication (starts it via D-Bus activation if needed).
    _activate(action, param) {
        const params = param ? [param] : [];
        Gio.DBus.session.call(APP, PATH, 'org.gtk.Actions', 'Activate',
            new GLib.Variant('(sava{sv})', [action, params, {}]),
            null, Gio.DBusCallFlags.NONE, -1, null, (conn, res) => {
                try { conn.call_finish(res); } catch (e) { /* app not running yet */ }
            });
    }
}
