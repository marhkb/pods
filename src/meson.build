# All Blueprint files below view/ and widget/. Adding or removing one requires a reconfigure
# (e.g. `meson setup --reconfigure`).
blueprint_files = run_command(
  'sh',
  '-c',
  'cd "$0" && find view widget -name "*.blp" | LC_ALL=C sort',
  meson.current_source_dir(),
  check: true,
).stdout().strip().split('\n')

blueprints = custom_target(
  'blueprints',
  input: files(blueprint_files),
  # use as output for rebuilding
  output: 'blueprints.stamp',
  command: [
    'sh',
    '-c',
    'compiler="$0" out="$1" src="$2" stamp="$3"; shift 3; "$compiler" batch-compile "$out" "$src" "$@" && touch "$stamp"',
    find_program('blueprint-compiler'),
    meson.current_build_dir(),
    '@CURRENT_SOURCE_DIR@',
    '@OUTPUT@',
    '@INPUT@',
  ],
)

# UI resources
ui_files = []
foreach blueprint_file : blueprint_files
  # The shortcuts dialog is registered under its own prefix in the template.
  if blueprint_file != 'view/shortcuts.blp'
    ui_files += '    <file compressed="true" preprocess="xml-stripblanks">@0@</file>'.format(
      blueprint_file.replace('.blp', '.ui'),
    )
  endif
endforeach

ui_resources_conf = configuration_data()
ui_resources_conf.set('UI_FILES', '\n'.join(ui_files))

gnome.compile_resources(
  'ui-resources',
  configure_file(
    input: 'resources.gresource.xml.in',
    output: 'resources.gresource.xml',
    configuration: ui_resources_conf,
  ),
  gresource_bundle: true,
  source_dir: meson.current_build_dir(),
  install: true,
  install_dir: pkgdatadir,
  dependencies: blueprints,
)

global_conf = configuration_data()
global_conf.set_quoted('APP_ID', application_id)
global_conf.set_quoted('PKGDATADIR', pkgdatadir)
global_conf.set_quoted('PROFILE', profile)
global_conf.set_quoted('VERSION', version + version_suffix)
global_conf.set_quoted('GETTEXT_PACKAGE', gettext_package)
global_conf.set_quoted('LOCALEDIR', localedir)
configure_file(input: 'config.rs.in', output: 'config.rs', configuration: global_conf)
# Copy the config.rs output to the source directory. Only copy it if it changed, so that a
# reconfigure doesn't touch the file and trigger a rebuild of the crate.
run_command(
  'sh',
  '-c',
  'cmp -s "$0" "$1" || cp "$0" "$1"',
  meson.project_build_root() / 'src' / 'config.rs',
  meson.project_source_root() / 'src' / 'config.rs',
  check: true,
)

cargo_options = ['--manifest-path', meson.project_source_root() / 'Cargo.toml']
cargo_options += ['--target-dir', meson.project_build_root() / 'src']

if get_option('profile') == 'default'
  cargo_options += ['--release']
  rust_target = 'release'
  message('Building in release mode')
else
  rust_target = 'debug'
  message('Building in debug mode')
endif

cargo_env = ['CARGO_HOME=' + meson.project_build_root() / 'cargo-home']

custom_target(
  'cargo-build',
  build_by_default: true,
  build_always_stale: true,
  output: meson.project_name(),
  console: true,
  install: true,
  install_dir: bindir,
  depends: resources,
  command: [
    'env',
    cargo_env,
    cargo,
    'build',
    cargo_options,
    '&&',
    'cp',
    'src' / rust_target / meson.project_name(),
    '@OUTPUT@',
  ],
)

cargo = find_program('cargo', required: true)
cargo_target_dir = meson.project_build_root() / 'target'
cargo_home = meson.project_build_root() / 'cargo-home'
manifest_path = meson.project_source_root() / 'Cargo.toml'
test(
  'clippy',
  cargo,
  args: [
    'clippy',
    '--manifest-path=@0@'.format(manifest_path),
    '--target-dir=@0@'.format(cargo_target_dir),
    '--',
    '-D', 'warnings',
  ],
  env: [
    'CARGO_HOME=@0@'.format(cargo_home),
    'PATH=/app/bin:/usr/bin:/usr/lib/sdk/llvm22/bin:/usr/lib/sdk/rust-stable/bin',
  ],
  timeout: 300, # Give cargo more time
)
