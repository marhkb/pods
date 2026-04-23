use std::cell::OnceCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::Properties;
use glib::clone;
use glib::closure;
use gtk::CompositeTemplate;
use gtk::gio;
use gtk::glib;

use crate::engine;
use crate::model;
use crate::utils;
use crate::view;
use crate::widget;

const ACTION_ADD_PORT_MAPPING: &str = "container-create-opts-dialog.add-port-mapping";
const ACTION_ADD_VOLUME: &str = "container-create-opts-dialog.add-volume";
const ACTION_ADD_ENV_VAR: &str = "container-create-opts-dialog.add-env-var";
const ACTION_ADD_LABEL: &str = "container-create-opts-dialog.add-label";
const ACTION_CREATE_AND_RUN: &str = "container-create-opts-dialog.create-and-run";
const ACTION_CREATE: &str = "container-create-opts-dialog.create";

mod imp {
    use super::*;

    #[derive(Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainerCreateOptsDialog)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/container_create_opts_dialog.ui")]
    pub(crate) struct ContainerCreateOptsDialog {
        pub(super) port_mappings: OnceCell<gio::ListStore>,
        pub(super) volumes: OnceCell<gio::ListStore>,
        pub(super) env_vars: OnceCell<gio::ListStore>,
        pub(super) labels: OnceCell<gio::ListStore>,

        #[property(get, set, construct_only, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[property(get, set, construct_only)]
        pub(super) opts: OnceCell<model::BoxedContainerCreateOpts>,

        #[template_child]
        pub(super) create_button: TemplateChild<adw::SplitButton>,
        #[template_child]
        pub(super) name_entry_row: TemplateChild<widget::RandomNameEntryRow>,
        #[template_child]
        pub(super) image_suggestion_entry_row: TemplateChild<view::ImageSuggestionEntryRow>,
        #[template_child]
        pub(super) pod_selection_combo_row: TemplateChild<view::PodSelectionComboRow>,
        #[template_child]
        pub(super) pull_latest_image_switch_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) command_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) terminal_switch_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) privileged_switch_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) memory_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) mem_value: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub(super) mem_drop_down: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub(super) port_mapping_preferences_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) port_mapping_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) volume_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) env_var_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) labels_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) health_check_command_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) health_check_interval_value: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub(super) health_check_timeout_value: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub(super) health_check_start_period_value: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub(super) health_check_retries_value: TemplateChild<gtk::Adjustment>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerCreateOptsDialog {
        const NAME: &'static str = "PdsContainerCreateOptsDialog";
        type Type = super::ContainerCreateOptsDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action(ACTION_ADD_PORT_MAPPING, None, |widget, _, _| {
                widget.add_port_mapping(None);
            });
            klass.install_action(ACTION_ADD_VOLUME, None, |widget, _, _| {
                widget.add_mount(None);
            });
            klass.install_action(ACTION_ADD_ENV_VAR, None, |widget, _, _| {
                widget.add_env_var(None);
            });
            klass.install_action(ACTION_ADD_LABEL, None, |widget, _, _| {
                widget.add_label(None);
            });
            klass.install_action(ACTION_CREATE_AND_RUN, None, |widget, _, _| {
                widget.close_and_create(true);
            });
            klass.install_action(ACTION_CREATE, None, |widget, _, _| {
                widget.close_and_create(false);
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerCreateOptsDialog {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            gtk::ClosureExpression::new::<bool>(
                [
                    self.pod_selection_combo_row.property_expression("active"),
                    self.pod_selection_combo_row
                        .property_expression("selected-pod"),
                ],
                closure!(
                    |_: Option<Self::Type>, active: bool, pod: Option<&model::Pod>| !active
                        || pod.is_none()
                ),
            )
            .bind(
                &self.port_mapping_preferences_group.get(),
                "visible",
                Some(obj),
            );

            bind_model(
                &self.port_mapping_list_box,
                self.port_mappings(),
                |item| {
                    view::PortMappingRow::from(item.downcast_ref::<model::PortMapping>().unwrap())
                        .upcast()
                },
                ACTION_ADD_PORT_MAPPING,
                &gettext("Add Port Mapping"),
            );

            bind_model(
                &self.volume_list_box,
                self.volumes(),
                |item| view::MountRow::from(item.downcast_ref::<model::Mount>().unwrap()).upcast(),
                ACTION_ADD_VOLUME,
                &gettext("Add Volume"),
            );

            bind_model(
                &self.env_var_list_box,
                self.env_vars(),
                |item| {
                    view::KeyValRow::from(item.downcast_ref::<model::KeyVal>().unwrap()).upcast()
                },
                ACTION_ADD_ENV_VAR,
                &gettext("Add Environment Variable"),
            );

            bind_model(
                &self.labels_list_box,
                self.labels(),
                |item| {
                    view::KeyValRow::from(item.downcast_ref::<model::KeyVal>().unwrap()).upcast()
                },
                ACTION_ADD_LABEL,
                &gettext("Add Label"),
            );

            let mut opts = self.obj().opts();

            self.name_entry_row.set_text(&opts.name);
            self.image_suggestion_entry_row.set_text(&opts.image);
            self.pod_selection_combo_row.select_pod(opts.pod.as_deref());
            self.pull_latest_image_switch_row
                .set_active(opts.pull_latest);
            self.terminal_switch_row.set_active(opts.terminal);
            self.privileged_switch_row.set_active(opts.privileged);
            self.memory_switch.set_active(opts.memory_limit.is_some());
            self.mem_value
                .set_value(opts.memory_limit.unwrap_or(512) as f64 / 1000.0);
            self.command_entry_row.set_text(
                &opts
                    .cmd
                    .as_ref()
                    .map(|cmd| cmd.join(" "))
                    .unwrap_or_default(),
            );

            opts.port_mappings.iter().for_each(|port_mapping| {
                obj.add_port_mapping(Some(model::PortMapping::from(port_mapping)));
            });
            opts.volumes.iter().for_each(|volume_opts| {
                if let Some(mount) = obj
                    .client()
                    .map(|client| model::Mount::new(&client, Some(volume_opts.to_owned())))
                {
                    obj.add_mount(Some(mount));
                }
            });
            opts.env.iter().for_each(|(key, val)| {
                obj.add_env_var(Some(model::KeyVal::from((key.as_str(), val.as_str()))));
            });
            opts.labels.iter().for_each(|(key, val)| {
                obj.add_label(Some(model::KeyVal::from((key.as_str(), val.as_str()))));
            });

            let health_config = opts.health_config.take().unwrap_or_default();
            self.health_check_command_entry_row.set_text(
                &health_config
                    .test
                    .as_ref()
                    .map(|test| test.join(" "))
                    .unwrap_or_default(),
            );
            self.health_check_interval_value
                .set_value(health_config.interval.unwrap_or_default() as f64);
            self.health_check_timeout_value
                .set_value(health_config.timeout.unwrap_or_default() as f64);
            self.health_check_start_period_value
                .set_value(health_config.start_period.unwrap_or_default() as f64);
            self.health_check_retries_value
                .set_value(health_config.retries.unwrap_or_default() as f64);
        }
    }

    impl WidgetImpl for ContainerCreateOptsDialog {
        fn map(&self) {
            self.parent_map();
            self.name_entry_row.grab_focus();
        }

        fn root(&self) {
            self.parent_root();
            utils::root(&*self.obj()).set_default_widget(Some(&*self.create_button));
        }

        fn unroot(&self) {
            utils::root(&*self.obj()).set_default_widget(gtk::Widget::NONE);
            self.parent_unroot()
        }
    }

    impl AdwDialogImpl for ContainerCreateOptsDialog {}

    #[gtk::template_callbacks]
    impl ContainerCreateOptsDialog {
        #[template_callback]
        fn on_name_entry_row_notify_text(&self) {
            let enabled = !self.name_entry_row.text().is_empty();

            let obj = &*self.obj();
            obj.action_set_enabled(ACTION_CREATE_AND_RUN, enabled);
            obj.action_set_enabled(ACTION_CREATE, enabled);
        }

        pub(super) fn port_mappings(&self) -> &gio::ListStore {
            self.port_mappings
                .get_or_init(gio::ListStore::new::<model::PortMapping>)
        }

        pub(super) fn volumes(&self) -> &gio::ListStore {
            self.volumes
                .get_or_init(gio::ListStore::new::<model::Mount>)
        }

        pub(super) fn env_vars(&self) -> &gio::ListStore {
            self.env_vars
                .get_or_init(gio::ListStore::new::<model::KeyVal>)
        }

        pub(super) fn labels(&self) -> &gio::ListStore {
            self.labels
                .get_or_init(gio::ListStore::new::<model::KeyVal>)
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainerCreateOptsDialog(ObjectSubclass<imp::ContainerCreateOptsDialog>)
        @extends gtk::Widget, adw::Dialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl ContainerCreateOptsDialog {
    pub(crate) fn new(
        client: &model::Client,
        opts: Option<model::BoxedContainerCreateOpts>,
    ) -> Self {
        glib::Object::builder()
            .property("client", client)
            .property("opts", opts.clone().unwrap_or_default())
            .build()
    }

    pub(crate) fn close_and_create(&self, run: bool) {
        self.close();

        let Some(action_list) = self.client().map(|client| client.action_list2()) else {
            return;
        };

        view::ActionDialog::from(&action_list.create_container(self.container_create_opts(), run))
            .present(Some(self));
    }

    fn container_create_opts(&self) -> engine::opts::ContainerCreateOpts {
        let imp = self.imp();

        engine::opts::ContainerCreateOpts {
            cmd: Some(imp.command_entry_row.text().trim())
                .filter(|cmd| !cmd.is_empty())
                .map(|cmd| {
                    cmd.split(' ')
                        .filter(|part| !part.is_empty())
                        .map(ToOwned::to_owned)
                        .collect()
                }),
            env: imp
                .env_vars()
                .iter::<model::KeyVal>()
                .map(Result::unwrap)
                .map(|entry| (entry.key(), entry.value()))
                .collect(),
            health_config: Some(imp.health_check_command_entry_row.text().trim())
                .filter(|test| !test.is_empty())
                .map(|test| engine::dto::HealthConfig {
                    interval: Some(imp.health_check_interval_value.value() as i64 * 1_000_000_000),
                    retries: Some(imp.health_check_retries_value.value() as i64),
                    start_period: Some(
                        imp.health_check_start_period_value.value() as i64 * 1_000_000_000,
                    ),
                    test: Some(test.split(' ').map(str::to_string).collect()),
                    timeout: Some(imp.health_check_timeout_value.value() as i64 * 1_000_000_000),
                }),
            image: imp.image_suggestion_entry_row.text().into(),
            labels: imp
                .labels()
                .iter::<model::KeyVal>()
                .map(Result::unwrap)
                .map(|entry| (entry.key(), entry.value()))
                .collect(),
            memory_limit: imp.memory_switch.is_active().then(|| {
                imp.mem_value.value() as u64 * 1000_u64.pow(imp.mem_drop_down.selected() + 1)
            }),
            mounts: imp
                .volumes()
                .iter::<model::Mount>()
                .map(Result::unwrap)
                .filter(|mount| mount.mount_type() == model::MountType::Bind)
                .map(|mount| engine::opts::ContainerCreateMountOpts {
                    container_path: mount.container_path(),
                    host_path: mount.host_path(),
                    read_only: !mount.writable(),
                    selinux: mount.selinux().into(),
                })
                .collect(),
            name: imp.name_entry_row.text().into(),
            pod: imp
                .pod_selection_combo_row
                .active()
                .then(|| imp.pod_selection_combo_row.selected_pod())
                .flatten()
                .map(|pod| pod.name()),
            port_mappings: imp
                .port_mappings()
                .iter::<model::PortMapping>()
                .map(Result::unwrap)
                .map(Into::into)
                .collect(),
            pull_latest: imp.pull_latest_image_switch_row.is_active(),
            privileged: imp.privileged_switch_row.is_active(),
            terminal: imp.terminal_switch_row.is_active(),
            volumes: imp
                .volumes()
                .iter::<model::Mount>()
                .map(Result::unwrap)
                .filter(|mount| mount.mount_type() == model::MountType::Volume)
                .map(|mount| engine::opts::ContainerCreateVolumeOpts {
                    container_path: mount.container_path(),
                    read_only: !mount.writable(),
                    selinux: mount.selinux().into(),
                    volume: mount
                        .volume()
                        .as_ref()
                        .map(model::Volume::name)
                        .unwrap_or_default(),
                })
                .collect(),
        }
    }

    fn add_port_mapping(&self, port_mapping: Option<model::PortMapping>) -> model::PortMapping {
        add_port_mapping(self.imp().port_mappings(), port_mapping)
    }

    fn add_mount(&self, mount: Option<model::Mount>) -> Option<model::Mount> {
        mount
            .or_else(|| self.client().map(|client| model::Mount::new(&client, None)))
            .map(|mount| add_mount(self.imp().volumes(), mount))
    }

    fn add_env_var(&self, entry: Option<model::KeyVal>) {
        add_key_val(self.imp().env_vars(), entry);
    }

    fn add_label(&self, entry: Option<model::KeyVal>) {
        add_key_val(self.imp().labels(), entry);
    }
}

fn bind_model<F>(
    list_box: &gtk::ListBox,
    model: &gio::ListStore,
    widget_func: F,
    action_name: &str,
    label: &str,
) where
    F: Fn(&glib::Object) -> gtk::Widget + 'static,
{
    list_box.bind_model(Some(model), widget_func);
    list_box.append(
        &gtk::ListBoxRow::builder()
            .action_name(action_name)
            .selectable(false)
            .child(
                &gtk::Label::builder()
                    .label(label)
                    .margin_top(12)
                    .margin_bottom(12)
                    .build(),
            )
            .build(),
    );
}

fn add_port_mapping(
    model: &gio::ListStore,
    port_mapping: Option<model::PortMapping>,
) -> model::PortMapping {
    let port_mapping = port_mapping.unwrap_or_default();

    port_mapping.connect_remove_request(clone!(
        #[weak]
        model,
        move |port_mapping| {
            if let Some(pos) = model.find(port_mapping) {
                model.remove(pos);
            }
        }
    ));

    model.append(&port_mapping);

    port_mapping
}

fn add_mount(model: &gio::ListStore, mount: model::Mount) -> model::Mount {
    mount.connect_remove_request(clone!(
        #[weak]
        model,
        move |mount| {
            if let Some(pos) = model.find(mount) {
                model.remove(pos);
            }
        }
    ));

    model.append(&mount);
    mount
}

fn add_key_val(model: &gio::ListStore, entry: Option<model::KeyVal>) {
    let entry = entry.unwrap_or_default();

    entry.connect_remove_request(clone!(
        #[weak]
        model,
        move |entry| {
            if let Some(pos) = model.find(entry) {
                model.remove(pos);
            }
        }
    ));

    model.append(&entry);
}
