use std::cell::OnceCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::Properties;
use glib::clone;
use gtk::CompositeTemplate;
use gtk::gio;
use gtk::glib;

use crate::engine;
use crate::model;
use crate::view;
use crate::widget;

const ACTION_ADD_LABEL: &str = "pod-create-opts-dialog.add-label";
const ACTION_ADD_HOST: &str = "pod-create-opts-dialog.add-host";
const ACTION_ADD_PORT_MAPPING: &str = "pod-create-opts-dialog.add-port-mapping";
const ACTION_ADD_DEVICE: &str = "pod-create-opts-dialog.add-device";
const ACTION_CREATE: &str = "pod-create-opts-dialog.create";

mod imp {
    use super::*;

    #[derive(Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::PodCreateOptsDialog)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/pod_create_opts_dialog.ui")]
    pub(crate) struct PodCreateOptsDialog {
        pub(super) labels: OnceCell<gio::ListStore>,
        pub(super) hosts: OnceCell<gio::ListStore>,
        pub(super) port_mappings: OnceCell<gio::ListStore>,
        pub(super) devices: OnceCell<gio::ListStore>,

        #[property(get, set, construct_only, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[property(get, set, construct_only)]
        pub(super) opts: OnceCell<model::BoxedPodCreateOpts>,

        #[template_child]
        pub(super) name_entry_row: TemplateChild<widget::RandomNameEntryRow>,
        #[template_child]
        pub(super) hostname_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) create_command_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) hosts_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) labels_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) port_mapping_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) devices_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) enable_hosts_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) disable_resolv_switch_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) disable_infra_switch_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) infra_settings_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) infra_name_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) infra_pull_latest_image_switch_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) infra_image_suggestion_entry_row: TemplateChild<view::ImageSuggestionEntryRow>,
        #[template_child]
        pub(super) infra_common_pid_file_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) infra_command_entry_row: TemplateChild<adw::EntryRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodCreateOptsDialog {
        const NAME: &'static str = "PdsPodCreateOptsDialog";
        type Type = super::PodCreateOptsDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action(ACTION_ADD_LABEL, None, |widget, _, _| {
                widget.add_label(None);
            });
            klass.install_action(ACTION_ADD_HOST, None, |widget, _, _| {
                widget.add_host(None);
            });
            klass.install_action(ACTION_ADD_PORT_MAPPING, None, |widget, _, _| {
                widget.add_port_mapping(None);
            });
            klass.install_action(ACTION_ADD_DEVICE, None, |widget, _, _| {
                widget.add_device(None);
            });
            klass.install_action(ACTION_CREATE, None, |widget, _, _| {
                widget.close_and_create();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PodCreateOptsDialog {
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

            bind_model(
                &self.labels_list_box,
                self.labels(),
                |item| {
                    view::KeyValRow::from(item.downcast_ref::<model::KeyVal>().unwrap()).upcast()
                },
                ACTION_ADD_LABEL,
                &gettext("Add Label"),
            );

            bind_model(
                &self.hosts_list_box,
                self.hosts(),
                |item| {
                    view::KeyValRow::new(
                        &gettext("Hostname"),
                        &gettext("IP"),
                        item.downcast_ref::<model::KeyVal>().unwrap(),
                    )
                    .upcast()
                },
                ACTION_ADD_HOST,
                &gettext("Add Host"),
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
                &self.devices_list_box,
                self.devices(),
                |item| {
                    view::DeviceRow::from(item.downcast_ref::<model::Device>().unwrap()).upcast()
                },
                ACTION_ADD_DEVICE,
                &gettext("Add Device"),
            );

            let opts = obj.opts();

            self.name_entry_row.set_text(&opts.name);
            self.hostname_entry_row.set_text(&opts.hostname);
            self.create_command_entry_row.set_text(
                &opts
                    .create_cmd
                    .as_ref()
                    .map(|cmd| cmd.join(" "))
                    .unwrap_or_default(),
            );

            if let engine::opts::PodHostManagement::Pod { ref hosts } = opts.host_management {
                self.enable_hosts_switch.set_active(true);
                hosts
                    .iter()
                    .for_each(|host| obj.add_host(Some(host.into())));
            }

            if let engine::opts::PodHostManagement::Pod { ref hosts } = opts.host_management {
                hosts
                    .iter()
                    .for_each(|host| obj.add_host(Some(host.into())));
            }

            opts.labels.iter().for_each(|(key, val)| {
                obj.add_label(Some(model::KeyVal::from((key.as_str(), val.as_str()))))
            });

            opts.port_mappings.iter().for_each(|port_mapping| {
                obj.add_port_mapping(Some(port_mapping.into()));
            });

            opts.devices.iter().for_each(|device| {
                obj.add_device(Some(device.into()));
            });

            match &opts.infra {
                engine::opts::PodInfra::Infra {
                    command,
                    common_pid_file,
                    image,
                    name,
                    no_manage_resolv_conf,
                    pull_latest,
                } => {
                    self.disable_infra_switch_row.set_active(false);
                    self.infra_command_entry_row.set_text(
                        &command
                            .as_ref()
                            .map(|cmd| cmd.join(" "))
                            .unwrap_or_default(),
                    );
                    self.infra_common_pid_file_entry_row
                        .set_text(common_pid_file.as_deref().unwrap_or_default());
                    self.infra_image_suggestion_entry_row
                        .set_text(image.as_deref().unwrap_or_default());
                    self.infra_name_entry_row
                        .set_text(name.as_deref().unwrap_or_default());
                    self.disable_resolv_switch_row
                        .set_active(*no_manage_resolv_conf);
                    self.infra_pull_latest_image_switch_row
                        .set_active(*pull_latest);
                }
                engine::opts::PodInfra::NoInfra => {
                    self.disable_infra_switch_row.set_active(true);
                }
            }
        }
    }

    impl WidgetImpl for PodCreateOptsDialog {
        fn map(&self) {
            self.parent_map();
            self.name_entry_row.grab_focus();
        }
    }

    impl AdwDialogImpl for PodCreateOptsDialog {}

    #[gtk::template_callbacks]
    impl PodCreateOptsDialog {
        #[template_callback]
        fn on_name_entry_row_changed(&self) {
            self.obj()
                .action_set_enabled(ACTION_CREATE, !self.name_entry_row.text().is_empty());
        }

        #[template_callback]
        fn on_disable_resolv_switch_row_active_changed(&self) {
            if self.disable_resolv_switch_row.is_active() {
                self.disable_infra_switch_row.set_active(false);
                self.infra_settings_box.set_visible(true);
            } else {
                self.hosts_list_box.set_visible(true);
            }
        }

        #[template_callback]
        fn on_enable_hosts_switch_active_changed(&self) {
            if self.enable_hosts_switch.is_active() {
                self.hosts_list_box.set_visible(true);
            } else {
                self.hosts_list_box.set_visible(false);
            }
        }

        #[template_callback]
        fn on_disable_infra_switch_row_active_changed(&self) {
            if self.disable_infra_switch_row.is_active() {
                self.infra_settings_box.set_visible(false);
                self.disable_resolv_switch_row.set_active(false);
            } else {
                self.infra_settings_box.set_visible(true);
            }
        }

        pub(super) fn labels(&self) -> &gio::ListStore {
            self.labels
                .get_or_init(gio::ListStore::new::<model::KeyVal>)
        }

        pub(super) fn hosts(&self) -> &gio::ListStore {
            self.hosts.get_or_init(gio::ListStore::new::<model::KeyVal>)
        }

        pub(super) fn port_mappings(&self) -> &gio::ListStore {
            self.port_mappings
                .get_or_init(gio::ListStore::new::<model::PortMapping>)
        }

        pub(super) fn devices(&self) -> &gio::ListStore {
            self.devices
                .get_or_init(gio::ListStore::new::<model::Device>)
        }
    }
}

glib::wrapper! {
    pub(crate) struct PodCreateOptsDialog(ObjectSubclass<imp::PodCreateOptsDialog>)
        @extends gtk::Widget, adw::Dialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl PodCreateOptsDialog {
    pub(crate) fn new(client: &model::Client, opts: Option<model::BoxedPodCreateOpts>) -> Self {
        glib::Object::builder()
            .property("client", client)
            .property("opts", opts.clone().unwrap_or_default())
            .build()
    }

    pub(crate) fn close_and_create(&self) {
        self.close();

        let Some(action_list) = self.client().map(|client| client.action_list()) else {
            return;
        };

        view::ActionDialog::from(&action_list.create_pod(self.create_opts())).present(Some(self));
    }

    fn create_opts(&self) -> engine::opts::PodCreateOpts {
        let imp = self.imp();

        engine::opts::PodCreateOpts {
            create_cmd: Some(imp.create_command_entry_row.text().trim())
                .filter(|cmd| !cmd.is_empty())
                .map(|cmd| {
                    cmd.split(' ')
                        .filter(|part| !part.is_empty())
                        .map(ToOwned::to_owned)
                        .collect()
                }),
            devices: imp
                .devices()
                .iter::<model::Device>()
                .map(Result::unwrap)
                .map(Into::into)
                .collect(),
            hostname: imp.hostname_entry_row.text().into(),
            host_management: if imp.enable_hosts_switch.is_active() {
                engine::opts::PodHostManagement::Pod {
                    hosts: imp
                        .hosts()
                        .iter::<model::KeyVal>()
                        .map(Result::unwrap)
                        .map(|entry| engine::opts::PodHost {
                            ip: entry.value(),
                            name: entry.key(),
                        })
                        .collect(),
                }
            } else {
                engine::opts::PodHostManagement::Containers
            },
            infra: if imp.disable_infra_switch_row.is_active() {
                engine::opts::PodInfra::NoInfra
            } else {
                engine::opts::PodInfra::Infra {
                    command: Some(imp.infra_command_entry_row.text().trim())
                        .filter(|cmd| !cmd.is_empty())
                        .map(|cmd| {
                            cmd.split(' ')
                                .filter(|part| !part.is_empty())
                                .map(ToOwned::to_owned)
                                .collect()
                        }),
                    common_pid_file: Some(imp.infra_common_pid_file_entry_row.text().trim())
                        .filter(|common_pid_file| !common_pid_file.is_empty())
                        .map(Into::into),
                    image: Some(imp.infra_image_suggestion_entry_row.text())
                        .filter(|image| !image.is_empty())
                        .map(Into::into),
                    name: Some(imp.infra_name_entry_row.text().trim())
                        .filter(|name| !name.is_empty())
                        .map(Into::into),
                    no_manage_resolv_conf: imp.disable_resolv_switch_row.is_active(),
                    pull_latest: imp.infra_pull_latest_image_switch_row.is_active(),
                }
            },
            labels: imp
                .labels()
                .iter::<model::KeyVal>()
                .map(Result::unwrap)
                .map(|entry| (entry.key(), entry.value()))
                .collect(),
            name: imp.name_entry_row.text().into(),
            port_mappings: imp
                .port_mappings()
                .iter::<model::PortMapping>()
                .map(Result::unwrap)
                .map(Into::into)
                .collect(),
        }
    }

    fn add_label(&self, label: Option<model::KeyVal>) {
        add_key_val(self.imp().labels(), label);
    }

    fn add_host(&self, host: Option<model::KeyVal>) {
        add_key_val(self.imp().hosts(), host);
    }

    fn add_port_mapping(&self, port_mapping: Option<model::PortMapping>) -> model::PortMapping {
        add_port_mapping(self.imp().port_mappings(), port_mapping)
    }

    fn add_device(&self, device: Option<model::Device>) -> model::Device {
        add_device(self.imp().devices(), device)
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

fn add_key_val(model: &gio::ListStore, key_val: Option<model::KeyVal>) -> model::KeyVal {
    let key_val = key_val.unwrap_or_default();

    key_val.connect_remove_request(clone!(
        #[weak]
        model,
        move |key_val| {
            if let Some(pos) = model.find(key_val) {
                model.remove(pos);
            }
        }
    ));

    model.append(&key_val);

    key_val
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

fn add_device(model: &gio::ListStore, device: Option<model::Device>) -> model::Device {
    let device = device.unwrap_or_default();

    device.connect_remove_request(clone!(
        #[weak]
        model,
        move |device| {
            if let Some(pos) = model.find(device) {
                model.remove(pos);
            }
        }
    ));

    model.append(&device);

    device
}
