// Einstellungsfenster der Erweiterung (GNOME 45+, libadwaita).

import Adw from 'gi://Adw';
import Gdk from 'gi://Gdk';
import Gio from 'gi://Gio';
import Gtk from 'gi://Gtk';

import {ExtensionPreferences} from 'resource:///org/gnome/Shell/Extensions/js/extensions/prefs.js';

function comboRow(settings, key, title, subtitle, options) {
    const row = new Adw.ComboRow({
        title,
        subtitle,
        model: Gtk.StringList.new(options.map(([, label]) => label)),
    });
    const sync = () => {
        const i = options.findIndex(([value]) => value === settings.get_string(key));
        row.selected = Math.max(0, i);
    };
    sync();
    settings.connect(`changed::${key}`, sync);
    row.connect('notify::selected', () => {
        const value = options[row.selected]?.[0];
        if (value && value !== settings.get_string(key))
            settings.set_string(key, value);
    });
    return row;
}

function spinRow(settings, key, title, subtitle, lower, upper, step, digits = 0) {
    const row = new Adw.SpinRow({
        title,
        subtitle,
        digits,
        adjustment: new Gtk.Adjustment({lower, upper, step_increment: step, page_increment: step * 10}),
    });
    settings.bind(key, row, 'value', Gio.SettingsBindFlags.DEFAULT);
    return row;
}

function switchRow(settings, key, title, subtitle) {
    const row = new Adw.SwitchRow({title, subtitle});
    settings.bind(key, row, 'active', Gio.SettingsBindFlags.DEFAULT);
    return row;
}

function colorRow(settings, key, title) {
    const row = new Adw.ActionRow({title});
    const button = new Gtk.ColorDialogButton({
        dialog: new Gtk.ColorDialog({with_alpha: false}),
        valign: Gtk.Align.CENTER,
    });
    const sync = () => {
        const rgba = new Gdk.RGBA();
        if (rgba.parse(settings.get_string(key)))
            button.rgba = rgba;
    };
    sync();
    settings.connect(`changed::${key}`, sync);
    button.connect('notify::rgba', () => {
        const c = button.rgba;
        const hex = '#' + [c.red, c.green, c.blue]
            .map(v => Math.round(v * 255).toString(16).padStart(2, '0')).join('');
        if (hex !== settings.get_string(key))
            settings.set_string(key, hex);
    });
    row.add_suffix(button);
    row.activatable_widget = button;
    return row;
}

function shortcutRow(settings) {
    const row = new Adw.EntryRow({
        title: 'Tastenkürzel zum Ein-/Ausblenden (z. B. <Super><Alt>c oder F8)',
        show_apply_button: true,
    });
    const sync = () => {
        row.text = settings.get_strv('toggle-shortcut')[0] ?? '';
    };
    sync();
    settings.connect('changed::toggle-shortcut', sync);
    row.connect('apply', () => {
        const text = row.text.trim();
        if (text === '') {
            settings.set_strv('toggle-shortcut', []);
            return;
        }
        const [ok, key] = Gtk.accelerator_parse(text);
        if (ok && key !== 0) {
            settings.set_strv('toggle-shortcut', [text]);
            row.remove_css_class('error');
        } else {
            row.add_css_class('error');
        }
    });
    return row;
}

export default class RobloxCrosshairPreferences extends ExtensionPreferences {
    fillPreferencesWindow(window) {
        const s = this.getSettings();
        window.set_default_size(560, 760);
        const page = new Adw.PreferencesPage({title: 'Fadenkreuz', icon_name: 'find-location-symbolic'});

        const show = new Adw.PreferencesGroup({
            title: 'Anzeige',
            description: 'Reines Bild-Overlay – greift nicht ins Spiel ein. Klicks gehen durch zum Spiel.',
        });
        show.add(switchRow(s, 'visible', 'Fadenkreuz eingeschaltet', 'Auch per Tastenkürzel oder im Schnellmenü oben rechts'));
        show.add(comboRow(s, 'show-when', 'Anzeigen', null, [
            ['roblox-focused', 'Nur wenn Roblox im Vordergrund ist'],
            ['roblox-running', 'Solange Roblox läuft'],
            ['always', 'Immer'],
        ]));
        show.add(comboRow(s, 'follow', 'Position', 'Ohne Shift-Lock zielt Roblox auf den Mauszeiger, mit Shift-Lock auf die Mitte', [
            ['auto', 'Automatisch (Mauszeiger – Mitte, wenn er versteckt ist)'],
            ['pointer', 'Immer am Mauszeiger'],
            ['center', 'Immer in der Mitte'],
        ]));
        show.add(comboRow(s, 'center-on', 'Mitte von', 'Gilt, wenn das Fadenkreuz in der Mitte steht', [
            ['monitor', 'Bildschirm'],
            ['window', 'Roblox-Fenster'],
        ]));
        show.add(switchRow(s, 'quick-toggle', 'Schalter im Schnellmenü', 'Oben rechts im Systemmenü'));
        page.add(show);

        const look = new Adw.PreferencesGroup({title: 'Aussehen', description: 'Größen in Pixeln'});
        look.add(comboRow(s, 'shape', 'Form', null, [
            ['cross-gap', 'Kreuz mit Lücke'],
            ['cross', 'Kreuz'],
            ['dot', 'Punkt'],
        ]));
        look.add(colorRow(s, 'color', 'Farbe'));
        look.add(spinRow(s, 'size', 'Größe', 'Gesamtgröße (Punkt: Durchmesser)', 1, 500, 1));
        look.add(spinRow(s, 'thickness', 'Linienstärke', null, 1, 100, 1));
        look.add(spinRow(s, 'gap', 'Lücke in der Mitte', 'Nur bei „Kreuz mit Lücke“', 0, 500, 1));
        look.add(spinRow(s, 'opacity', 'Deckkraft', '1 = voll sichtbar, 0,5 = halb durchsichtig', 0.1, 1, 0.05, 2));
        look.add(switchRow(s, 'outline', 'Dunkler Rand', 'Damit es auf hellen und dunklen Wänden sichtbar ist'));
        look.add(spinRow(s, 'outline-thickness', 'Randstärke', null, 0, 20, 1));
        look.add(colorRow(s, 'outline-color', 'Randfarbe'));
        page.add(look);

        const pos = new Adw.PreferencesGroup({
            title: 'Position',
            description: 'Verschieben, falls die Mitte im Spiel nicht ganz passt',
        });
        pos.add(spinRow(s, 'offset-x', 'Nach rechts', 'Negativ = nach links', -5000, 5000, 1));
        pos.add(spinRow(s, 'offset-y', 'Nach unten', 'Negativ = nach oben', -5000, 5000, 1));
        page.add(pos);

        const keys = new Adw.PreferencesGroup({title: 'Tastenkürzel'});
        keys.add(shortcutRow(s));
        page.add(keys);

        window.add(page);
    }
}
