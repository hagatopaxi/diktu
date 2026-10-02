project('parlotte', version: '0.1.0', license: 'GPL-3.0-or-later', meson_version: '>= 1.0')

gnome = import('gnome')
app_id = 'fr.gwenael_leger.Parlotte'
datadir = get_option('prefix') / get_option('datadir')

cargo = find_program('cargo')
cargo_options = ['--manifest-path', meson.project_source_root() / 'Cargo.toml',
                 '--target-dir', meson.project_build_root() / 'target']
if get_option('offline')
  cargo_options += ['--offline']
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
  output: 'parlotte',
  console: true,
  install: true,
  install_dir: get_option('bindir'),
  command: [cargo, 'build', cargo_options, '-p', 'parlotte', '&&',
            'cp', meson.project_build_root() / 'target' / rust_target / 'parlotte', '@OUTPUT@'],
)

install_data('data' / app_id + '.gschema.xml', install_dir: datadir / 'glib-2.0' / 'schemas')
test('gschema', find_program('glib-compile-schemas'),
  args: ['--strict', '--dry-run', meson.project_source_root() / 'data'])

gnome.post_install(glib_compile_schemas: true)
