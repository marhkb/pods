install_data(
  '@0@.svg'.format(application_id),
  install_dir: iconsdir / 'hicolor' / 'scalable' / 'apps',
)

install_data(
  '@0@-symbolic.svg'.format(base_id),
  install_dir: iconsdir / 'hicolor' / 'symbolic' / 'apps',
  rename: '@0@-symbolic.svg'.format(application_id),
)

theme_conf = configuration_data()
theme_conf.set('app-id', application_id)
configure_file(
  input: 'theme' / 'index.theme.in',
  output: 'index.theme',
  configuration: theme_conf,
  install: true,
  install_dir: iconsdir / application_id,
)

install_subdir(
  'theme' / 'scalable',
  install_dir: iconsdir / application_id,
)
