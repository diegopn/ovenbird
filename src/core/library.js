import GLib from 'gi://GLib';
import { parseBibtex, serializeBibtex } from './bibtex.js';
import { _ } from './i18n.js';

export class LocalLibrary {
    constructor() {
        this.directory = GLib.build_filenamev([GLib.get_user_data_dir(), 'ovenbird']);
        GLib.mkdir_with_parents(this.directory, 0o755);
        this.path = GLib.build_filenamev([this.directory, 'library.bib']);
        this.entries = [];
        this.directives = [];
        this.load();
    }

    load() {
        try {
            const [ok, bytes] = GLib.file_get_contents(this.path);
            if (ok) {
                const parsed = parseBibtex(new TextDecoder().decode(bytes));
                this.entries = parsed;
                this.directives = parsed.directives || [];
            }
        } catch (error) {
            if (!error.matches?.(GLib.FileError, GLib.FileError.NOENT))
                logError(error, 'Could not load the local bibliography');
        }
    }

    save() {
        const content = serializeBibtex(this.entries, this.directives);
        GLib.file_set_contents(this.path, content);
    }

    add(entry) {
        if (this.entries.some(item => item.key.toLowerCase() === entry.key.toLowerCase()))
            throw new Error(_('Citation key “%s” already exists.').replace('%s', entry.key));
        this.entries.push({ ...entry, rawFields: {} });
        this.save();
    }

    update(previousKey, updated) {
        if (this.entries.some(item => item.key.toLowerCase() === updated.key.toLowerCase() && item.key !== previousKey))
            throw new Error(_('Citation key “%s” already exists.').replace('%s', updated.key));
        const index = this.entries.findIndex(item => item.key === previousKey);
        if (index < 0) throw new Error(_('The reference no longer exists.'));
        const previous = this.entries[index];
        const rawFields = {};
        for (const [name, value] of Object.entries(updated.fields || {})) {
            if (previous.fields?.[name] === value && previous.rawFields?.[name] !== undefined)
                rawFields[name] = previous.rawFields[name];
        }
        this.entries[index] = { ...updated, rawFields };
        this.save();
    }

    remove(key) {
        this.entries = this.entries.filter(entry => entry.key !== key);
        this.save();
    }
}
