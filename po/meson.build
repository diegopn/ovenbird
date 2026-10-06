translations = i18n.gettext('ovenbird', preset: 'glib', languages: ['pt_BR', 'es'])
i18n_gjs = find_program('gjs')

test('i18n-pt-br', i18n_gjs, args: ['-m', join_paths(meson.project_source_root(), 'tests/i18n.js')],
  depends: translations[0],
  env: ['LC_ALL=pt_BR.UTF-8', 'LANGUAGE=pt_BR', 'OVENBIRD_LOCALEDIR=' + meson.current_build_dir(),
    'OVENBIRD_EXPECTED_TITLE=Novo documento', 'OVENBIRD_EXPECTED_INTRODUCTION=Introdução',
    'OVENBIRD_EXPECTED_REFERENCE_SINGULAR=1 referência local',
    'OVENBIRD_EXPECTED_REFERENCE_PLURAL=2 referências locais'])
test('i18n-spanish', i18n_gjs, args: ['-m', join_paths(meson.project_source_root(), 'tests/i18n.js')],
  depends: translations[0],
  env: ['LC_ALL=es_ES.UTF-8', 'LANGUAGE=es', 'OVENBIRD_LOCALEDIR=' + meson.current_build_dir(),
    'OVENBIRD_EXPECTED_TITLE=Nuevo documento', 'OVENBIRD_EXPECTED_INTRODUCTION=Introducción',
    'OVENBIRD_EXPECTED_REFERENCE_SINGULAR=1 referencia local',
    'OVENBIRD_EXPECTED_REFERENCE_PLURAL=2 referencias locales'])
test('i18n-english-fallback', i18n_gjs, args: ['-m', join_paths(meson.project_source_root(), 'tests/i18n.js')],
  depends: translations[0],
  env: ['LC_ALL=fr_FR.UTF-8', 'LANGUAGE=fr_FR', 'OVENBIRD_LOCALEDIR=' + meson.current_build_dir(),
    'OVENBIRD_EXPECTED_TITLE=New document', 'OVENBIRD_EXPECTED_INTRODUCTION=Introduction',
    'OVENBIRD_EXPECTED_REFERENCE_SINGULAR=1 local reference',
    'OVENBIRD_EXPECTED_REFERENCE_PLURAL=2 local references'])
