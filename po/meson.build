source_files = run_command(
  'sh',
  '-c',
  'cd "$0" && find src \( -name "*.rs" -o -name "*.blp" \) ! -path src/config.rs | LC_ALL=C sort',
  meson.project_source_root(),
  check: true,
).stdout().strip().split('\n')

i18n.gettext(
  gettext_package,
  args: ['--keyword=gettext_f', '--keyword=ngettext_f:1,2'] + source_files,
  preset: 'glib',
)
