import GLib from 'gi://GLib';

const Gettext = imports.gettext;
export const DOMAIN = 'ovenbird';

Gettext.setlocale(Gettext.LocaleCategory.ALL, '');

function localeDirectoryFromModule() {
    const [modulePath] = GLib.filename_from_uri(import.meta.url);
    let shareDirectory = GLib.path_get_dirname(modulePath);
    for (let level = 0; level < 3; level++)
        shareDirectory = GLib.path_get_dirname(shareDirectory);
    return GLib.build_filenamev([shareDirectory, 'locale']);
}

function defaultLocaleDirectory() {
    const candidates = [GLib.getenv('OVENBIRD_LOCALEDIR'), localeDirectoryFromModule(),
        ...GLib.get_system_data_dirs().map(path => GLib.build_filenamev([path, 'locale'])),
        '/app/share/locale'].filter(Boolean);
    return candidates.find(path => GLib.file_test(path, GLib.FileTest.IS_DIR)) || candidates[0];
}

export function bindTranslations(localeDirectory = defaultLocaleDirectory()) {
    Gettext.bindtextdomain(DOMAIN, localeDirectory);
    Gettext.textdomain(DOMAIN);
}

bindTranslations();

export const _ = Gettext.domain(DOMAIN).gettext;
export const ngettext = Gettext.domain(DOMAIN).ngettext;
