import './core/i18n.js';
import Gio from 'gi://Gio';
import GObject from 'gi://GObject';
import Gtk from 'gi://Gtk?version=4.0';
import Adw from 'gi://Adw?version=1';
import GtkSource from 'gi://GtkSource?version=5';
import { LocalLibrary } from './core/library.js';
import { OvenbirdWindow } from './window.js';

export const OvenbirdApplication = GObject.registerClass(
class OvenbirdApplication extends Adw.Application {
    constructor() {
        super({
            application_id: 'org.ovenbird.Ovenbird',
            flags: Gio.ApplicationFlags.HANDLES_OPEN,
        });
        this.library = null;
    }

    vfunc_startup() {
        super.vfunc_startup();
        GtkSource.init();
        this.library = new LocalLibrary();
        this.set_accels_for_action('win.save', ['<primary>s']);
        this.set_accels_for_action('win.open', ['<primary>o']);
        this.set_accels_for_action('win.compile', ['<primary><shift>r']);
        this.set_accels_for_action('win.find', ['<primary>f']);
        this.set_accels_for_action('win.toggle-sidebar', ['F9']);
        this.set_accels_for_action('win.undo', ['<primary>z']);
        this.set_accels_for_action('win.redo', ['<primary><shift>z']);
        this.set_accels_for_action('win.cite', ['<primary><shift>c']);
    }

    vfunc_activate() {
        let window = this.active_window;
        if (!window) window = new OvenbirdWindow({ application: this });
        window.present();
    }

    vfunc_open(files) {
        let window = this.active_window;
        if (!window) window = new OvenbirdWindow({ application: this });
        window.present();
        if (files.length > 0) window.openDocument(files[0]);
    }
});
