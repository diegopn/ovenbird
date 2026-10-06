project('ovenbird', version: '0.1.0', meson_version: '>= 1.2.0')

gnome = import('gnome')
i18n = import('i18n')

application_id = 'org.ovenbird.Ovenbird'
datadir = get_option('datadir') / application_id

install_data(
  'src/main.js',
  'src/application.js',
  'src/window.js',
  install_dir: datadir / 'src',
)

install_subdir('src/core', install_dir: datadir / 'src')
install_data('src/ovenbird', install_dir: get_option('bindir'), install_mode: 'rwxr-xr-x')

subdir('data')
subdir('po')

gjs = find_program('gjs')
test('core', gjs, args: ['-m', join_paths(meson.project_source_root(), 'tests/core.js')])
test('build-path', gjs, args: ['-m', join_paths(meson.project_source_root(), 'tests/build.js')])

gnome.post_install(
  update_desktop_database: true,
)
