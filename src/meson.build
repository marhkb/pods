blueprints = custom_target(
  'blueprints',
  input: files(
    'view/action_page.blp',
    'view/action_row.blp',
    'view/actions_button.blp',
    'view/actions_sidebar.blp',
    'view/client_view.blp',
    'view/connection_chooser_page.blp',
    'view/connection_creation_page.blp',
    'view/connection_custom_info_page.blp',
    'view/connection_row.blp',
    'view/connections_sidebar.blp',
    'view/container_card.blp',
    'view/container_commit_page.blp',
    'view/container_creation_page.blp',
    'view/container_details_page.blp',
    'view/container_files_get_page.blp',
    'view/container_files_put_page.blp',
    'view/container_health_check_log_row.blp',
    'view/container_health_check_page.blp',
    'view/container_log_page.blp',
    'view/container_menu_button.blp',
    'view/container_properties_group.blp',
    'view/container_renamer.blp',
    'view/container_resources.blp',
    'view/container_row.blp',
    'view/container_terminal.blp',
    'view/container_terminal_page.blp',
    'view/container_volume_row.blp',
    'view/containers_count_bar.blp',
    'view/containers_grid_view.blp',
    'view/containers_group.blp',
    'view/containers_list_view.blp',
    'view/containers_panel.blp',
    'view/containers_prune_page.blp',
    'view/containers_row.blp',
    'view/device_row.blp',
    'view/image_build_page.blp',
    'view/image_details_page.blp',
    'view/image_history_page.blp',
    'view/image_menu_button.blp',
    'view/image_pull_page.blp',
    'view/image_row.blp',
    'view/image_search_page.blp',
    'view/image_search_response_row.blp',
    'view/image_selection_combo_row.blp',
    'view/image_selection_page.blp',
    'view/images_panel.blp',
    'view/images_prune_page.blp',
    'view/images_row.blp',
    'view/info_panel.blp',
    'view/info_row.blp',
    'view/key_val_row.blp',
    'view/mount_row.blp',
    'view/network_card.blp',
    'view/network_creation_page.blp',
    'view/network_row.blp',
    'view/networks_grid_view.blp',
    'view/networks_list_view.blp',
    'view/networks_panel.blp',
    'view/networks_row.blp',
    'view/pod_creation_page.blp',
    'view/pod_details_page.blp',
    'view/pod_menu_button.blp',
    'view/pod_row.blp',
    'view/pod_selection_page.blp',
    'view/pods_panel.blp',
    'view/pods_prune_page.blp',
    'view/pods_row.blp',
    'view/port_mapping_row.blp',
    'view/repo_tag_add_dialog.blp',
    'view/repo_tag_push_page.blp',
    'view/repo_tag_row.blp',
    'view/repo_tag_selection_page.blp',
    'view/repo_tag_simple_row.blp',
    'view/scalable_text_view_page.blp',
    'view/search_panel.blp',
    'view/shortcuts.blp',
    'view/top_page.blp',
    'view/top_page_action_bar.blp',
    'view/value_row.blp',
    'view/volume_creation_page.blp',
    'view/volume_details_page.blp',
    'view/volume_row.blp',
    'view/volume_selection_page.blp',
    'view/volumes_group.blp',
    'view/volumes_panel.blp',
    'view/volumes_prune_page.blp',
    'view/volumes_row.blp',
    'view/welcome_page.blp',
    'view/window.blp',
    'widget/circular_progress_bar.blp',
    'widget/count_badge.blp',
    'widget/date_time_row.blp',
    'widget/main_menu_button.blp',
    'widget/property_row.blp',
    'widget/property_widget_row.blp',
    'widget/random_name_entry_row.blp',
    'widget/source_view_search_widget.blp',
    'widget/spinner.blp',
    'widget/text_search_entry.blp',
    'widget/zoom_control.blp',
  ),
  output: '.',
  command: [
    find_program('blueprint-compiler'),
    'batch-compile',
    # '--minify',
    '@OUTPUT@',
    '@CURRENT_SOURCE_DIR@',
    '@INPUT@',
  ],
)

# UI resources
ui_resources = gnome.compile_resources(
  'ui-resources',
  'resources.gresource.xml',
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
# Copy the config.rs output to the source directory.
run_command(
  'cp',
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
    'PATH=/app/bin:/usr/bin:/usr/lib/sdk/llvm21/bin:/usr/lib/sdk/rust-stable/bin',
  ],
  timeout: 300, # Give cargo more time
)
