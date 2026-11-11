cargo = find_program('cargo')
fs = import('fs')
shell = find_program('sh')
rustc = find_program('rustc', required: false)
if rustc.found()
  rustc_path = rustc.full_path()
else
  rustc_path = fs.parent(cargo.full_path()) / 'rustc'
  if not fs.exists(rustc_path)
    error('Could not find rustc in PATH or next to Cargo')
  endif
endif

rust_sources = files(
  'main.rs',
  'lib.rs',
  'app/mod.rs',
  'app/settings.rs',
  'app/application_menu.rs',
  'app/libpanel.rs',
  'app/runtime.rs',
  'app/window/mod.rs',
  'app/window/actions.rs',
  'app/window/dialogs.rs',
  'app/window/documents.rs',
  'app/window/editor.rs',
  'app/window/latex_tools.rs',
  'app/window/preferences.rs',
  'app/window/presentation.rs',
  'app/window/project_files.rs',
  'app/window/references.rs',
  'commands.rs',
  'history.rs',
  'i18n.rs',
  'project.rs',
  'search.rs',
  'storage.rs',
  'tests.rs',
  'ui/mod.rs',
  'ui/editor.rs',
  'ui/pdf_preview.rs',
  'references/mod.rs',
  'references/bibtex.rs',
  'references/exchange.rs',
  'references/guides.rs',
  'references/library.rs',
  'latex/mod.rs',
  'latex/build.rs',
  'latex/diagnostics.rs',
  'latex/templates.rs',
  '../build.rs',
  '../data/ovenbird.css',
  '../Cargo.toml',
  '../Cargo.lock',
  '../cargo-cache/CACHE_VERSION',
)

cargo_profile = 'debug'
cargo_profile_args = ''
cargo_feature_args = ''
test_cargo_feature_args = []
if get_option('buildtype') == 'release'
  cargo_profile = 'release'
  cargo_profile_args = '--release'
endif
if poppler.found()
  cargo_feature_args = '--features poppler-preview'
  test_cargo_feature_args = ['--features', 'poppler-preview']
endif

cargo_home = meson.project_build_root() / 'cargo-home'
cargo_target_dir = meson.project_build_root() / 'cargo-target'
template_dir = get_option('prefix') / get_option('datadir') / application_id / 'templates'

cargo_build_script = '''
set -eu
if ! cmp -s "$OVENBIRD_CARGO_CACHE/CACHE_VERSION" "$CARGO_HOME/.ovenbird-registry-cache-version"; then
  rm -rf "$CARGO_HOME/registry"
  mkdir -p "$CARGO_HOME/registry"
  cp -a "$OVENBIRD_CARGO_CACHE/registry/index" "$CARGO_HOME/registry/"
  cp -a "$OVENBIRD_CARGO_CACHE/registry/cache" "$CARGO_HOME/registry/"
  if [ -d "$OVENBIRD_CARGO_CACHE/registry/src" ]; then
    cp -a "$OVENBIRD_CARGO_CACHE/registry/src" "$CARGO_HOME/registry/"
  fi
  cp "$OVENBIRD_CARGO_CACHE/CACHE_VERSION" "$CARGO_HOME/.ovenbird-registry-cache-version"
fi
"$OVENBIRD_CARGO" build --manifest-path "$OVENBIRD_MANIFEST" --target-dir "$CARGO_TARGET_DIR" --locked --offline $OVENBIRD_PROFILE_ARGS $OVENBIRD_CARGO_FEATURES
cp "$CARGO_TARGET_DIR/$OVENBIRD_PROFILE/ovenbird" "@OUTPUT@"
'''

ovenbird_binary = custom_target(
  'ovenbird-rust',
  input: rust_sources,
  output: 'ovenbird',
  command: [shell, '-c', cargo_build_script],
  env: {
    'CARGO_HOME': cargo_home,
    'CARGO_TARGET_DIR': cargo_target_dir,
    'RUSTC': rustc_path,
    'OVENBIRD_CARGO': cargo.full_path(),
    'OVENBIRD_CARGO_CACHE': meson.project_source_root() / 'cargo-cache',
    'OVENBIRD_MANIFEST': meson.project_source_root() / 'Cargo.toml',
    'OVENBIRD_PROFILE': cargo_profile,
    'OVENBIRD_PROFILE_ARGS': cargo_profile_args,
    'OVENBIRD_CARGO_FEATURES': cargo_feature_args,
    'OVENBIRD_TEMPLATE_DIR': template_dir,
    'CARGO_NET_OFFLINE': 'true',
  },
  build_by_default: true,
  install: true,
  install_dir: get_option('bindir'),
)

install_subdir('templates', install_dir: datadir)

test(
  'rust-unit',
  cargo,
  args: [
    'test', '--manifest-path', meson.project_source_root() / 'Cargo.toml',
    '--target-dir', cargo_target_dir, '--locked', '--offline', '--all-targets',
  ] + test_cargo_feature_args,
  env: [
    'CARGO_HOME=' + cargo_home,
    'CARGO_TARGET_DIR=' + cargo_target_dir,
    'CARGO_NET_OFFLINE=true',
    'RUSTC=' + rustc_path,
  ],
  depends: [ovenbird_binary],
  suite: 'rust',
)
