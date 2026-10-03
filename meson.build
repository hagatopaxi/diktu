project('diktu', version: '0.1.0', license: 'GPL-3.0-or-later', meson_version: '>= 1.0')

gnome = import('gnome')
base_id = 'fr.gwenael_leger.Diktu'
if get_option('profile') == 'development'
  app_id = base_id + '.Devel'
  app_name = 'Diktu dev'
  bin_name = 'diktu-dev'
  schema_path = '/fr/gwenael_leger/Diktu/Devel/'
else
  app_id = base_id
  app_name = 'Diktu'
  bin_name = 'diktu'
  schema_path = '/fr/gwenael_leger/Diktu/'
endif
datadir = get_option('prefix') / get_option('datadir')

cargo = find_program('cargo')
cargo_options = ['--manifest-path', meson.project_source_root() / 'Cargo.toml',
                 '--target-dir', meson.project_build_root() / 'target']
if get_option('offline')
  cargo_options += ['--offline']
endif
features = []
if get_option('shared_sherpa_onnx')
  features += 'diktu-core/shared-sherpa'
endif
if get_option('profile') == 'development'
  features += 'diktu/devel'
endif
if features.length() > 0
  cargo_options += ['--features', ','.join(features)]
endif
if get_option('buildtype') == 'release'
  cargo_options += ['--release']
  rust_target = 'release'
else
  rust_target = 'debug'
endif

custom_target('cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: bin_name,
  console: true,
  install: true,
  install_dir: get_option('bindir'),
  command: [cargo, 'build', cargo_options, '-p', 'diktu', '&&',
            'cp', meson.project_build_root() / 'target' / rust_target / 'diktu', '@OUTPUT@'],
)

# The schema and metainfo carry the app ID: rewritten for the development profile.
rename_id = ['sed', '-e', 's|"' + base_id + '"|"' + app_id + '"|',
             '-e', 's|/fr/gwenael_leger/Diktu/|' + schema_path + '|',
             '-e', 's|<id>' + base_id + '</id>|<id>' + app_id + '</id>|',
             '-e', 's|>' + base_id + '.desktop<|>' + app_id + '.desktop<|',
             '-e', 's|<name>Diktu</name>|<name>' + app_name + '</name>|',
             '@INPUT@']
configure_file(input: 'data' / base_id + '.gschema.xml', output: app_id + '.gschema.xml',
  command: rename_id, capture: true, install_dir: datadir / 'glib-2.0' / 'schemas')
metainfo = configure_file(input: 'data' / base_id + '.metainfo.xml', output: app_id + '.metainfo.xml',
  command: rename_id, capture: true, install_dir: datadir / 'metainfo')
# Absolute Exec: xdg-desktop-portal resolves it with its own PATH, which lacks ~/.local/bin.
desktop_file = configure_file(
  input: 'data' / base_id + '.desktop.in',
  output: app_id + '.desktop',
  configuration: {'BINDIR': get_option('prefix') / get_option('bindir'), 'BIN': bin_name,
                  'NAME': app_name, 'APP_ID': app_id},
  install_dir: datadir / 'applications',
)
install_data('data' / 'icons' / app_id + '.svg', install_dir: datadir / 'icons' / 'hicolor' / 'scalable' / 'apps')

test('gschema', find_program('glib-compile-schemas'),
  args: ['--strict', '--dry-run', meson.project_source_root() / 'data'])
desktop_file_validate = find_program('desktop-file-validate', required: false)
if desktop_file_validate.found()
  test('desktop', desktop_file_validate, args: [desktop_file])
endif
appstreamcli = find_program('appstreamcli', required: false)
if appstreamcli.found()
  test('metainfo', appstreamcli,
    args: ['validate', '--no-net', metainfo])
endif

gnome.post_install(glib_compile_schemas: true, gtk_update_icon_cache: true, update_desktop_database: true)
