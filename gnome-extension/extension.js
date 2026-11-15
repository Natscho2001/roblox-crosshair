// Roblox Fadenkreuz — GNOME Shell extension (GNOME 45+).
//
// GNOME (Mutter) has no wlr-layer-shell, so an external program can't put a
// click-through overlay on screen. Inside the shell we can: the crosshair is a
// non-reactive actor in the top chrome that doesn't touch the input region, so
// every click and key still goes to the game. It only draws a picture — it never
// reads from or talks to Roblox.

import GObject from 'gi://GObject';
import Gio from 'gi://Gio';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import St from 'gi://St';

import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import {QuickToggle, SystemIndicator} from 'resource:///org/gnome/shell/ui/quickSettings.js';

import {actorPosition, computeLayout, isRobloxIds, parseColor} from './geometry.js';

function isRoblox(win) {
    if (!win || win.is_override_redirect?.())
        return false;
    return isRobloxIds([
        win.get_sandboxed_app_id?.(),
        win.get_gtk_application_id?.(),
        win.get_wm_class?.(),
        win.get_wm_class_instance?.(),
    ]);
}

const CrosshairToggle = GObject.registerClass(
class CrosshairToggle extends QuickToggle {
    _init(settings) {
        super._init({title: 'Fadenkreuz', iconName: 'find-location-symbolic', toggleMode: true});
        settings.bind('visible', this, 'checked', Gio.SettingsBindFlags.DEFAULT);
    }
});

const CrosshairIndicator = GObject.registerClass(
class CrosshairIndicator extends SystemIndicator {
    _init(settings) {
        super._init();
        this.quickSettingsItems.push(new CrosshairToggle(settings));
    }

    destroy() {
        this.quickSettingsItems.forEach(item => item.destroy());
        super.destroy();
    }
});

class Crosshair {
    constructor(settings) {
        this._settings = settings;
        this._signals = [];
        this._windows = new Map(); // Meta.Window -> [signal ids]
        this._unredirectDisabled = false;
        this._layout = null;

        this._area = new St.DrawingArea({reactive: false, can_focus: false, track_hover: false});
        this._area.connect('repaint', area => this._repaint(area));
        Main.layoutManager.addTopChrome(this._area, {
            affectsInputRegion: false, // click-through
            affectsStruts: false,
            trackFullscreen: false, // stay visible over fullscreen games
        });
        this._area.hide();

        this._connect(this._settings, 'changed', () => this._rebuild());
        this._connect(global.display, 'notify::focus-window', () => this._update());
        this._connect(global.display, 'window-created', (_d, win) => this._watch(win));
        this._connect(Main.layoutManager, 'monitors-changed', () => this._update());
        for (const actor of global.get_window_actors())
            this._watch(actor.meta_window);

        this._rebuild();
    }

    _connect(obj, signal, fn) {
        this._signals.push([obj, obj.connect(signal, fn)]);
    }

    _watch(win) {
        if (!win || this._windows.has(win))
            return;
        const ids = ['unmanaged', 'notify::wm-class', 'notify::gtk-application-id', 'notify::minimized',
            'position-changed', 'size-changed'].map(sig => win.connect(sig, () => {
            if (sig === 'unmanaged')
                this._unwatch(win);
            this._update();
        }));
        this._windows.set(win, ids);
    }

    _unwatch(win) {
        const ids = this._windows.get(win);
        if (!ids)
            return;
        ids.forEach(id => win.disconnect(id));
        this._windows.delete(win);
    }

    _robloxWindows() {
        return [...this._windows.keys()].filter(w => isRoblox(w) && !w.minimized);
    }

    _rebuild() {
        const s = this._settings;
        this._layout = computeLayout({
            shape: s.get_string('shape'),
            size: s.get_double('size'),
            thickness: s.get_double('thickness'),
            gap: s.get_double('gap'),
            outline: s.get_boolean('outline'),
            outlineThickness: s.get_double('outline-thickness'),
        });
        this._color = parseColor(s.get_string('color'));
        this._outlineColor = parseColor(s.get_string('outline-color'), [0, 0, 0]);
        this._area.set_size(this._layout.box, this._layout.box);
        this._area.opacity = Math.round(255 * Math.min(1, Math.max(0, s.get_double('opacity'))));
        this._area.queue_repaint();
        this._update();
    }

    _shouldShow() {
        if (!this._settings.get_boolean('visible'))
            return false;
        switch (this._settings.get_string('show-when')) {
        case 'always':
            return true;
        case 'roblox-running':
            return this._robloxWindows().length > 0;
        default: // roblox-focused
            return isRoblox(global.display.focus_window);
        }
    }

    _target() {
        const lm = Main.layoutManager;
        const focused = global.display.focus_window;
        const win = isRoblox(focused) ? focused : this._robloxWindows()[0];
        let cx, cy;
        if (win && this._settings.get_string('center-on') === 'window') {
            const r = win.get_frame_rect();
            [cx, cy] = [r.x + r.width / 2, r.y + r.height / 2];
        } else {
            let mon = lm.primaryMonitor;
            const idx = win ? win.get_monitor() : -1;
            if (idx >= 0 && lm.monitors[idx])
                mon = lm.monitors[idx];
            if (!mon)
                return null;
            [cx, cy] = [mon.x + mon.width / 2, mon.y + mon.height / 2];
        }
        return [cx + this._settings.get_double('offset-x'), cy + this._settings.get_double('offset-y')];
    }

    _update() {
        const target = this._shouldShow() ? this._target() : null;
        if (!target) {
            this._area.hide();
            this._setUnredirect(true);
            return;
        }
        const [x, y] = actorPosition(target[0], target[1], this._layout.box);
        this._area.set_position(x, y);
        // Keep it above everything else in the top chrome (e.g. after panels were re-added).
        this._area.get_parent()?.set_child_above_sibling(this._area, null);
        this._area.show();
        // A fullscreen game could otherwise be scanned out directly, hiding the overlay.
        this._setUnredirect(false);
    }

    _setUnredirect(allowed) {
        const comp = global.compositor;
        if (!comp?.disable_unredirect)
            return;
        if (!allowed && !this._unredirectDisabled) {
            comp.disable_unredirect();
            this._unredirectDisabled = true;
        } else if (allowed && this._unredirectDisabled) {
            comp.enable_unredirect();
            this._unredirectDisabled = false;
        }
    }

    _repaint(area) {
        const cr = area.get_context();
        const {rects, dot, outline} = this._layout;
        const fill = (rgb, grow) => {
            cr.setSourceRGBA(rgb[0], rgb[1], rgb[2], 1);
            if (dot) {
                cr.arc(dot.c, dot.c, dot.r + grow, 0, 2 * Math.PI);
            } else {
                for (const r of rects)
                    cr.rectangle(r.x - grow, r.y - grow, r.w + 2 * grow, r.h + 2 * grow);
            }
            cr.fill();
        };
        if (outline > 0)
            fill(this._outlineColor, outline);
        fill(this._color, 0);
        cr.$dispose();
    }

    destroy() {
        this._signals.forEach(([obj, id]) => obj.disconnect(id));
        this._signals = [];
        [...this._windows.keys()].forEach(w => this._unwatch(w));
        this._setUnredirect(true);
        Main.layoutManager.removeChrome(this._area);
        this._area.destroy();
        this._area = null;
    }
}

export default class RobloxCrosshairExtension extends Extension {
    enable() {
        this._settings = this.getSettings();
        this._crosshair = new Crosshair(this._settings);
        Main.wm.addKeybinding('toggle-shortcut', this._settings, Meta.KeyBindingFlags.NONE,
            Shell.ActionMode.NORMAL | Shell.ActionMode.OVERVIEW,
            () => this._settings.set_boolean('visible', !this._settings.get_boolean('visible')));
        this._syncIndicator();
        this._quickToggleId = this._settings.connect('changed::quick-toggle', () => this._syncIndicator());
    }

    _syncIndicator() {
        const want = this._settings.get_boolean('quick-toggle');
        if (want && !this._indicator) {
            this._indicator = new CrosshairIndicator(this._settings);
            Main.panel.statusArea.quickSettings.addExternalIndicator(this._indicator);
        } else if (!want && this._indicator) {
            this._indicator.destroy();
            this._indicator = null;
        }
    }

    disable() {
        Main.wm.removeKeybinding('toggle-shortcut');
        this._settings.disconnect(this._quickToggleId);
        this._indicator?.destroy();
        this._indicator = null;
        this._crosshair.destroy();
        this._crosshair = null;
        this._settings = null;
    }
}
