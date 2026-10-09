project('ovenbird', 'rust', version: '0.1.0', meson_version: '>= 1.2.0', license: 'GPL-3.0-or-later')

gnome = import('gnome')
i18n = import('i18n')

application_id = 'io.github.diegopn.ovenbird'
datadir = get_option('datadir') / application_id

dependency('gtk4', version: '>= 4.20')
dependency('libadwaita-1', version: '>= 1.8')
libpanel = dependency('libpanel-1', version: '>= 1.10.4')
dependency('gtksourceview-5', version: '>= 5.18')
poppler = dependency('poppler-glib', version: '>= 0.18', required: false)

subdir('src')
subdir('data')
subdir('po')

gnome.post_install(
  gtk_update_icon_cache: true,
  update_desktop_database: true,
)
