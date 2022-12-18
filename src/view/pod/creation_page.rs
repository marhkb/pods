use std::cell::RefCell;

use adw::subclass::prelude::*;
use adw::traits::ActionRowExt;
use adw::traits::BinExt;
use adw::traits::ComboRowExt;
use gettextrs::gettext;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::podman;
use crate::utils;
use crate::view;

const ACTION_CREATE: &str = "pod-creation-page.create";
const ACTION_ADD_LABEL: &str = "pod-creation-page.add-label";
const ACTION_ADD_HOST: &str = "pod-creation-page.add-host";
const ACTION_ADD_DEVICE: &str = "pod-creation-page.add-device";
const ACTION_ADD_INFRA_CMD_ARGS: &str = "pod-creation-page.add-infra-cmd-arg";
const ACTION_ADD_POD_CREATE_CMD_ARGS: &str = "pod-creation-page.add-pod-create-cmd-arg";
const ACTION_TOGGLE_INFRA: &str = "pod-creation-page.toggle-infra";
const ACTION_TOGGLE_HOSTS: &str = "pod-creation-page.toggle-hosts";
const ACTION_TOGGLE_RESOLV: &str = "pod-creation-page.toggle-resolv";
const ACTION_REMOVE_REMOTE_INFRA: &str = "pod-creation-page.infra-remove-remote";
const ACTION_SEARCH_INFRA: &str = "pod-creation-page.infra-search";

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/pod/creation-page.ui")]
    pub(crate) struct CreationPage {
        pub(super) client: glib::WeakRef<model::Client>,
        pub(super) infra_image: glib::WeakRef<model::Image>,
        pub(super) labels: gio::ListStore,
        pub(super) hosts: gio::ListStore,
        pub(super) devices: gio::ListStore,
        pub(super) pod_create_cmd_args: gio::ListStore,
        pub(super) infra_cmd_args: gio::ListStore,
        pub(super) command_row_handler:
            RefCell<Option<(glib::SignalHandlerId, glib::WeakRef<model::Image>)>>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) leaflet_overlay: TemplateChild<view::LeafletOverlay>,
        #[template_child]
        pub(super) create_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) name_entry_row: TemplateChild<view::RandomNameEntryRow>,
        #[template_child]
        pub(super) hostname_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) pod_create_command_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) pod_create_command_arg_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) hosts_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) labels_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) devices_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) enable_hosts_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) disable_resolv_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) disable_resolv_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) disable_infra_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) disable_infra_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) infra_settings_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) infra_name_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) infra_pull_latest_image_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) infra_pull_latest_image_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) infra_local_image_combo_row: TemplateChild<view::ImageLocalComboRow>,
        #[template_child]
        pub(super) infra_remote_image_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) infra_common_pid_file_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) infra_command_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) infra_command_arg_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) action_page_bin: TemplateChild<adw::Bin>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CreationPage {
        const NAME: &'static str = "PdsPodCreationPage";
        type Type = super::CreationPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action(ACTION_CREATE, None, |widget, _, _| {
                widget.finish();
            });
            klass.install_action(ACTION_ADD_LABEL, None, |widget, _, _| {
                widget.add_label();
            });
            klass.install_action(ACTION_ADD_HOST, None, |widget, _, _| {
                widget.add_host();
            });
            klass.install_action(ACTION_ADD_DEVICE, None, |widget, _, _| {
                widget.add_device();
            });
            klass.install_action(ACTION_ADD_POD_CREATE_CMD_ARGS, None, |widget, _, _| {
                widget.add_pod_create_cmd_arg();
            });
            klass.install_action(ACTION_ADD_INFRA_CMD_ARGS, None, |widget, _, _| {
                widget.add_infra_cmd_arg();
            });
            klass.install_action(ACTION_TOGGLE_INFRA, None, |widget, _, _| {
                widget.toggle_infra();
            });
            klass.install_action(ACTION_TOGGLE_HOSTS, None, |widget, _, _| {
                widget.toggle_hosts();
            });
            klass.install_action(ACTION_TOGGLE_RESOLV, None, |widget, _, _| {
                widget.toggle_resolv();
            });
            klass.install_action(ACTION_REMOVE_REMOTE_INFRA, None, move |widget, _, _| {
                widget.remove_remote();
            });
            klass.install_action(ACTION_SEARCH_INFRA, None, move |widget, _, _| {
                widget.search_image();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CreationPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::Client>("client")
                        .construct_only()
                        .build(),
                    glib::ParamSpecObject::builder::<model::Image>("infra-image")
                        .construct_only()
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "client" => self.client.set(value.get().unwrap()),
                "infra-image" => self.infra_image.set(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.obj();
            match pspec.name() {
                "client" => obj.client().to_value(),
                "infra-image" => obj.image().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            self.name_entry_row
                .connect_text_notify(clone!(@weak obj => move |_| obj.on_name_changed()));

            bind_model(
                &self.labels_list_box,
                &self.labels,
                |item| {
                    view::KeyValRow::from(item.downcast_ref::<model::KeyVal>().unwrap()).upcast()
                },
                ACTION_ADD_LABEL,
            );

            bind_model(
                &self.hosts_list_box,
                &self.hosts,
                |item| {
                    view::KeyValRow::new(
                        gettext("Hostname"),
                        gettext("IP"),
                        item.downcast_ref::<model::KeyVal>().unwrap(),
                    )
                    .upcast()
                },
                ACTION_ADD_HOST,
            );

            bind_model(
                &self.devices_list_box,
                &self.devices,
                |item| {
                    view::DeviceRow::from(item.downcast_ref::<model::Device>().unwrap()).upcast()
                },
                ACTION_ADD_DEVICE,
            );

            bind_model(
                &self.pod_create_command_arg_list_box,
                &self.pod_create_cmd_args,
                |item| {
                    view::ValueRow::new(item.downcast_ref().unwrap(), gettext("Argument")).upcast()
                },
                ACTION_ADD_POD_CREATE_CMD_ARGS,
            );

            bind_model(
                &self.infra_command_arg_list_box,
                &self.infra_cmd_args,
                |item| {
                    view::ValueRow::new(item.downcast_ref().unwrap(), gettext("Argument")).upcast()
                },
                ACTION_ADD_INFRA_CMD_ARGS,
            );

            self.infra_local_image_combo_row
                .set_client(obj.client().as_ref());
            self.infra_local_image_combo_row
                .connect_selected_item_notify(
                    clone!(@weak obj => move |_| obj.update_infra_command_row()),
                );
            obj.update_infra_command_row();
        }

        fn dispose(&self) {
            utils::ChildIter::from(&*self.obj()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for CreationPage {
        fn root(&self) {
            self.parent_root();

            let widget = &*self.obj();

            glib::idle_add_local(
                clone!(@weak widget => @default-return glib::Continue(false), move || {
                    widget.imp().name_entry_row.grab_focus();
                    glib::Continue(false)
                }),
            );
            utils::root(widget).set_default_widget(Some(&*self.create_button));
        }

        fn unroot(&self) {
            utils::root(&*self.obj()).set_default_widget(gtk::Widget::NONE);
            self.parent_unroot()
        }
    }
}

glib::wrapper! {
    pub(crate) struct CreationPage(ObjectSubclass<imp::CreationPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Client> for CreationPage {
    fn from(client: &model::Client) -> Self {
        glib::Object::builder().property("client", client).build()
    }
}

impl CreationPage {
    fn client(&self) -> Option<model::Client> {
        self.imp().client.upgrade()
    }

    fn on_name_changed(&self) {
        self.action_set_enabled(ACTION_CREATE, self.imp().name_entry_row.text().len() > 0);
    }

    fn image(&self) -> Option<model::Image> {
        self.imp().infra_image.upgrade()
    }

    fn add_label(&self) {
        add_key_val(&self.imp().labels);
    }

    fn add_host(&self) {
        add_key_val(&self.imp().hosts);
    }

    fn add_device(&self) {
        add_device(&self.imp().devices);
    }

    fn add_pod_create_cmd_arg(&self) {
        add_value(&self.imp().pod_create_cmd_args);
    }

    fn add_infra_cmd_arg(&self) {
        add_value(&self.imp().infra_cmd_args);
    }

    fn finish(&self) {
        let imp = self.imp();

        if imp.infra_remote_image_row.is_visible() {
            self.pull_and_create(imp.infra_remote_image_row.subtitle().unwrap().as_str());
        } else if let Some(image) = self.image().or_else(|| {
            imp.infra_local_image_combo_row
                .selected_item()
                .map(|item| item.downcast().unwrap())
        }) {
            if imp.infra_pull_latest_image_switch.is_active() {
                self.pull_and_create(image.repo_tags().get(0).unwrap().full());
            } else {
                let page =
                    view::ActionPage::from(&self.client().unwrap().action_list().create_pod(
                        imp.name_entry_row.text().as_str(),
                        self.create().infra_image(image.id()).build(),
                    ));

                imp.action_page_bin.set_child(Some(&page));
                imp.stack.set_visible_child(&*imp.action_page_bin);
            }
        } else {
            log::error!("Error while starting pod: no image selected");
            utils::show_error_toast(
                self,
                &gettext("Failed to create pod"),
                &gettext("no image selected"),
            )
        }
    }

    fn pull_and_create(&self, reference: &str) {
        let imp = self.imp();

        let pull_opts = podman::opts::PullOpts::builder()
            .reference(reference)
            .quiet(false)
            .build();

        let page = view::ActionPage::from(
            &self
                .client()
                .unwrap()
                .action_list()
                .create_pod_download_infra(
                    imp.name_entry_row.text().as_str(),
                    reference,
                    pull_opts,
                    self.create(),
                ),
        );

        imp.action_page_bin.set_child(Some(&page));
        imp.stack.set_visible_child(&*imp.action_page_bin);
    }

    fn create(&self) -> podman::opts::PodCreateOptsBuilder {
        let imp = self.imp();

        let mut opts = podman::opts::PodCreateOpts::builder()
            .name(imp.name_entry_row.text().as_str())
            .hostname(imp.hostname_entry_row.text().as_str())
            .labels(
                imp.labels
                    .iter::<glib::Object>()
                    .unwrap()
                    .map(|entry| entry.unwrap().downcast::<model::KeyVal>().unwrap())
                    .map(|entry| (entry.key(), entry.value())),
            );

        if imp.disable_infra_switch.is_active() {
            opts = opts.no_infra(true);
        } else {
            let infra_name = imp.infra_name_entry_row.text();
            if !infra_name.is_empty() {
                opts = opts.infra_name(infra_name.as_str());
            }

            if imp.disable_resolv_switch.is_active() {
                opts = opts.no_manage_resolv_conf(true);
            }

            let infra_command = imp.infra_command_entry_row.text();
            if !infra_command.is_empty() {
                let args = imp
                    .infra_cmd_args
                    .iter::<glib::Object>()
                    .unwrap()
                    .map(|value| value.unwrap().downcast::<model::Value>().unwrap())
                    .map(|value| value.value());
                let mut cmd = vec![infra_command.to_string()];
                cmd.extend(args);
                opts = opts.infra_command(cmd);
            }
            let infra_common_pid_file = imp.infra_common_pid_file_entry_row.text();
            if !infra_common_pid_file.is_empty() {
                opts = opts.infra_common_pid_file(infra_common_pid_file.as_str());
            }
        }

        if imp.enable_hosts_switch.is_active() {
            opts = opts.add_hosts(
                imp.hosts
                    .iter::<glib::Object>()
                    .unwrap()
                    .map(|entry| entry.unwrap().downcast::<model::KeyVal>().unwrap())
                    .map(|entry| format!("{}:{}", entry.key(), entry.value())),
            )
        } else {
            opts = opts.no_manage_hosts(true);
        }

        let create_cmd = imp.pod_create_command_entry_row.text();
        if !create_cmd.is_empty() {
            let args = imp
                .pod_create_cmd_args
                .iter::<glib::Object>()
                .unwrap()
                .map(|value| value.unwrap().downcast::<model::Value>().unwrap())
                .map(|value| value.value());
            let mut cmd = vec![create_cmd.to_string()];
            cmd.extend(args);
            opts = opts.pod_create_command(cmd);
        }

        let devices: Vec<_> = imp
            .devices
            .iter::<glib::Object>()
            .unwrap()
            .map(|device| device.unwrap().downcast::<model::Device>().unwrap())
            .map(|device| {
                format!(
                    "{}:{}:{}{}{}",
                    device.host_path(),
                    device.container_path(),
                    if device.readable() { "r" } else { "" },
                    if device.writable() { "w" } else { "" },
                    if device.mknod() { "m" } else { "" },
                )
            })
            .collect();
        if !devices.is_empty() {
            opts = opts.pod_devices(devices);
        }

        opts
    }

    fn toggle_infra(&self) {
        let imp = self.imp();
        if imp.disable_infra_switch.is_active() {
            imp.infra_settings_box.set_visible(false);
            imp.disable_resolv_switch.set_active(false);
        } else {
            imp.infra_settings_box.set_visible(true);
        }
    }

    fn toggle_hosts(&self) {
        let imp = self.imp();
        if imp.enable_hosts_switch.is_active() {
            imp.hosts_list_box.set_visible(true);
        } else {
            imp.hosts_list_box.set_visible(false);
        }
    }

    fn toggle_resolv(&self) {
        let imp = self.imp();
        if imp.disable_resolv_switch.is_active() {
            imp.disable_infra_switch.set_active(false);
            imp.infra_settings_box.set_visible(true);
        } else {
            imp.hosts_list_box.set_visible(true);
        }
    }

    fn remove_remote(&self) {
        let imp = self.imp();
        imp.infra_remote_image_row.set_subtitle("");
        imp.infra_remote_image_row.set_visible(false);
        imp.infra_local_image_combo_row.set_visible(true);
        imp.infra_pull_latest_image_row.set_visible(true);
    }

    fn search_image(&self) {
        if let Some(client) = self.client() {
            let image_selection_page = view::ImageSelectionPage::from(&client);
            image_selection_page.connect_image_selected(
                clone!(@weak self as obj => move |_, image| {
                    let imp = obj.imp();

                    imp.infra_local_image_combo_row.set_visible(false);
                    imp.infra_remote_image_row.set_visible(true);
                    imp.infra_remote_image_row.set_subtitle(&image);
                    imp.infra_pull_latest_image_row.set_visible(false);

                    imp.infra_command_entry_row.set_text("");
                }),
            );
            self.imp()
                .leaflet_overlay
                .show_details(&image_selection_page);
        }
    }

    fn update_infra_command_row(&self) {
        let imp = self.imp();

        match imp
            .infra_local_image_combo_row
            .selected_item()
            .as_ref()
            .map(|item| item.downcast_ref::<model::Image>().unwrap())
        {
            Some(image) => match image.data() {
                Some(details) => imp
                    .infra_command_entry_row
                    .set_text(details.config().cmd().unwrap_or("")),
                None => {
                    if let Some((handler, image)) = imp.command_row_handler.take() {
                        if let Some(image) = image.upgrade() {
                            image.disconnect(handler);
                        }
                    }
                    let handler = image.connect_notify_local(
                        Some("details"),
                        clone!(@weak self as obj => move |image, _| {
                            obj.imp().infra_command_entry_row.set_text(
                                image.data().unwrap().config().cmd().unwrap_or("")
                            );
                        }),
                    );
                    let image_weak = glib::WeakRef::new();
                    image_weak.set(Some(image));
                    imp.command_row_handler.replace(Some((handler, image_weak)));

                    image.inspect(|_| {});
                }
            },
            None => imp.infra_command_entry_row.set_text(""),
        }
    }
}

fn bind_model<F>(list_box: &gtk::ListBox, model: &gio::ListStore, widget_func: F, action_name: &str)
where
    F: Fn(&glib::Object) -> gtk::Widget + 'static,
{
    list_box.bind_model(Some(model), widget_func);
    list_box.append(
        &gtk::ListBoxRow::builder()
            .action_name(action_name)
            .selectable(false)
            .child(
                &gtk::Image::builder()
                    .icon_name("list-add-symbolic")
                    .margin_top(12)
                    .margin_bottom(12)
                    .build(),
            )
            .build(),
    );
}

fn add_key_val(model: &gio::ListStore) {
    let entry = model::KeyVal::default();

    entry.connect_remove_request(clone!(@weak model => move |entry| {
        if let Some(pos) = model.find(entry) {
            model.remove(pos);
        }
    }));

    model.append(&entry);
}

fn add_value(model: &gio::ListStore) {
    let value = model::Value::default();

    value.connect_remove_request(clone!(@weak model => move |value| {
        if let Some(pos) = model.find(value) {
            model.remove(pos);
        }
    }));

    model.append(&value);
}

fn add_device(model: &gio::ListStore) {
    let device = model::Device::default();

    device.connect_remove_request(clone!(@weak model => move |device| {
        if let Some(pos) = model.find(device) {
            model.remove(pos);
        }
    }));

    model.append(&device);
}
