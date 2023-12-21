use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::clone;
use glib::Properties;
use gtk::gio;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::podman;
use crate::utils;
use crate::view;
use crate::widget;

const ACTION_CREATE: &str = "pod-creation-page.create";
const ACTION_ADD_LABEL: &str = "pod-creation-page.add-label";
const ACTION_ADD_HOST: &str = "pod-creation-page.add-host";
const ACTION_ADD_DEVICE: &str = "pod-creation-page.add-device";
const ACTION_ADD_INFRA_CMD_ARGS: &str = "pod-creation-page.add-infra-cmd-arg";
const ACTION_ADD_POD_CREATE_CMD_ARGS: &str = "pod-creation-page.add-pod-create-cmd-arg";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::PodCreationPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/pod_creation_page.ui")]
    pub(crate) struct PodCreationPage {
        pub(super) labels: OnceCell<gio::ListStore>,
        pub(super) hosts: OnceCell<gio::ListStore>,
        pub(super) devices: OnceCell<gio::ListStore>,
        pub(super) pod_create_cmd_args: OnceCell<gio::ListStore>,
        pub(super) infra_cmd_args: OnceCell<gio::ListStore>,
        pub(super) command_row_handler:
            RefCell<Option<(glib::SignalHandlerId, glib::WeakRef<model::Image>)>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[property(get, set, construct_only, nullable)]
        pub(super) infra_image: glib::WeakRef<model::Image>,
        #[property(get, set, construct_only)]
        pub(super) show_view_artifact: Cell<bool>,
        #[template_child]
        pub(super) navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub(super) create_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) name_entry_row: TemplateChild<widget::RandomNameEntryRow>,
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
        pub(super) infra_image_selection_combo_row: TemplateChild<view::ImageSelectionComboRow>,
        #[template_child]
        pub(super) infra_common_pid_file_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) infra_command_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) infra_command_arg_list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodCreationPage {
        const NAME: &'static str = "PdsPodCreationPage";
        type Type = super::PodCreationPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

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
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PodCreationPage {
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
                &self.devices_list_box,
                self.devices(),
                |item| {
                    view::DeviceRow::from(item.downcast_ref::<model::Device>().unwrap()).upcast()
                },
                ACTION_ADD_DEVICE,
                &gettext("Add Device"),
            );

            bind_model(
                &self.pod_create_command_arg_list_box,
                self.pod_create_cmd_args(),
                |item| {
                    view::ValueRow::new(item.downcast_ref().unwrap(), &gettext("Argument")).upcast()
                },
                ACTION_ADD_POD_CREATE_CMD_ARGS,
                &gettext("Add Argument"),
            );

            bind_model(
                &self.infra_command_arg_list_box,
                self.infra_cmd_args(),
                |item| {
                    view::ValueRow::new(item.downcast_ref().unwrap(), &gettext("Argument")).upcast()
                },
                ACTION_ADD_INFRA_CMD_ARGS,
                &gettext("Add Argument"),
            );

            self.infra_image_selection_combo_row
                .set_client(obj.client());

            obj.update_infra_command_row();
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for PodCreationPage {
        fn root(&self) {
            self.parent_root();

            let widget = &*self.obj();

            glib::idle_add_local(
                clone!(@weak widget => @default-return glib::ControlFlow::Break, move || {
                    widget.imp().name_entry_row.grab_focus();
                    glib::ControlFlow::Break
                }),
            );
            utils::root(widget.upcast_ref()).set_default_widget(Some(&*self.create_button));
        }

        fn unroot(&self) {
            utils::root(self.obj().upcast_ref()).set_default_widget(gtk::Widget::NONE);
            self.parent_unroot()
        }
    }

    #[gtk::template_callbacks]
    impl PodCreationPage {
        #[template_callback]
        fn on_name_entry_row_changed(&self) {
            self.obj().on_name_changed();
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

        #[template_callback]
        fn on_infra_image_selection_combo_row_notify_subtitle(&self) {
            self.obj().update_infra_command_row();
        }

        pub(super) fn labels(&self) -> &gio::ListStore {
            self.labels
                .get_or_init(gio::ListStore::new::<model::KeyVal>)
        }

        pub(super) fn hosts(&self) -> &gio::ListStore {
            self.hosts.get_or_init(gio::ListStore::new::<model::KeyVal>)
        }

        pub(super) fn devices(&self) -> &gio::ListStore {
            self.devices
                .get_or_init(gio::ListStore::new::<model::Device>)
        }

        pub(super) fn pod_create_cmd_args(&self) -> &gio::ListStore {
            self.pod_create_cmd_args
                .get_or_init(gio::ListStore::new::<model::Value>)
        }

        pub(super) fn infra_cmd_args(&self) -> &gio::ListStore {
            self.infra_cmd_args
                .get_or_init(gio::ListStore::new::<model::Value>)
        }
    }
}

glib::wrapper! {
    pub(crate) struct PodCreationPage(ObjectSubclass<imp::PodCreationPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Client> for PodCreationPage {
    fn from(client: &model::Client) -> Self {
        Self::new(client, true)
    }
}

impl PodCreationPage {
    pub(crate) fn new(client: &model::Client, show_view_artifact: bool) -> Self {
        glib::Object::builder()
            .property("client", client)
            .property("show-view-artifact", show_view_artifact)
            .build()
    }

    fn on_name_changed(&self) {
        self.action_set_enabled(ACTION_CREATE, self.imp().name_entry_row.text().len() > 0);
    }

    fn add_label(&self) {
        add_key_val(self.imp().labels());
    }

    fn add_host(&self) {
        add_key_val(self.imp().hosts());
    }

    fn add_device(&self) {
        add_device(self.imp().devices());
    }

    fn add_pod_create_cmd_arg(&self) {
        add_value(self.imp().pod_create_cmd_args());
    }

    fn add_infra_cmd_arg(&self) {
        add_value(self.imp().infra_cmd_args());
    }

    fn finish(&self) {
        let imp = self.imp();

        if !imp.disable_infra_switch_row.is_active() {
            match imp.infra_image_selection_combo_row.mode() {
                view::ImageSelectionMode::Local => {
                    let image = imp.infra_image_selection_combo_row.subtitle().unwrap();
                    if imp.infra_pull_latest_image_switch_row.is_active() {
                        self.pull_and_create(image.as_str());
                    } else {
                        self.create(Some(image.as_str()));
                    }
                }
                view::ImageSelectionMode::Remote => {
                    self.pull_and_create(
                        imp.infra_image_selection_combo_row
                            .subtitle()
                            .unwrap()
                            .as_str(),
                    );
                }
                view::ImageSelectionMode::Unset => self.create(None),
            }
        } else {
            self.create(None)
        }
    }

    fn create(&self, infra_image: Option<&str>) {
        let imp = self.imp();

        let opts = self.opts();
        let opts = if let Some(infra_image) = infra_image {
            opts.infra_image(infra_image)
        } else {
            opts
        };

        let page = view::ActionPage::new(
            &self
                .client()
                .unwrap()
                .action_list()
                .create_pod(imp.name_entry_row.text().as_str(), opts.build()),
            self.show_view_artifact(),
        );

        imp.navigation_view.push(
            &adw::NavigationPage::builder()
                .can_pop(false)
                .child(&page)
                .build(),
        );
    }

    fn pull_and_create(&self, reference: &str) {
        let imp = self.imp();

        let pull_opts = podman::opts::PullOpts::builder()
            .reference(reference)
            .quiet(false)
            .build();

        let page = view::ActionPage::new(
            &self
                .client()
                .unwrap()
                .action_list()
                .create_pod_download_infra(
                    imp.name_entry_row.text().as_str(),
                    pull_opts,
                    self.opts(),
                ),
            self.show_view_artifact(),
        );

        imp.navigation_view.push(
            &adw::NavigationPage::builder()
                .can_pop(false)
                .child(&page)
                .build(),
        );
    }

    fn opts(&self) -> podman::opts::PodCreateOptsBuilder {
        let imp = self.imp();

        let mut opts = podman::opts::PodCreateOpts::builder()
            .name(imp.name_entry_row.text().as_str())
            .hostname(imp.hostname_entry_row.text().as_str())
            .labels(
                imp.labels()
                    .iter::<model::KeyVal>()
                    .map(Result::unwrap)
                    .map(|entry| (entry.key(), entry.value())),
            );

        if imp.disable_infra_switch_row.is_active() {
            opts = opts.no_infra(true);
        } else {
            let infra_name = imp.infra_name_entry_row.text();
            if !infra_name.is_empty() {
                opts = opts.infra_name(infra_name.as_str());
            }

            if imp.disable_resolv_switch_row.is_active() {
                opts = opts.no_manage_resolv_conf(true);
            }

            let infra_command = imp.infra_command_entry_row.text();
            if !infra_command.is_empty() {
                let args = imp
                    .infra_cmd_args()
                    .iter::<model::Value>()
                    .map(Result::unwrap)
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
                imp.hosts()
                    .iter::<model::KeyVal>()
                    .map(Result::unwrap)
                    .map(|entry| format!("{}:{}", entry.key(), entry.value())),
            )
        } else {
            opts = opts.no_manage_hosts(true);
        }

        let create_cmd = imp.pod_create_command_entry_row.text();
        if !create_cmd.is_empty() {
            let args = imp
                .pod_create_cmd_args()
                .iter::<model::Value>()
                .map(Result::unwrap)
                .map(|value| value.value());
            let mut cmd = vec![create_cmd.to_string()];
            cmd.extend(args);
            opts = opts.pod_create_command(cmd);
        }

        let devices: Vec<_> = imp
            .devices()
            .iter::<model::Device>()
            .map(Result::unwrap)
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

    fn update_infra_command_row(&self) {
        let imp = self.imp();

        match imp.infra_image_selection_combo_row.image() {
            Some(image) => match image.data() {
                Some(details) => imp
                    .infra_command_entry_row
                    .set_text(&details.config().cmd().unwrap_or_default()),
                None => {
                    if let Some((handler, image)) = imp.command_row_handler.take() {
                        if let Some(image) = image.upgrade() {
                            image.disconnect(handler);
                        }
                    }
                    let handler =
                        image.connect_data_notify(clone!(@weak self as obj => move |image| {
                            obj.imp().infra_command_entry_row.set_text(
                                &image.data().unwrap().config().cmd().unwrap_or_default()
                            );
                        }));
                    let image_weak = glib::WeakRef::new();
                    image_weak.set(Some(&image));
                    imp.command_row_handler.replace(Some((handler, image_weak)));

                    image.inspect(|_| {});
                }
            },
            None => imp.infra_command_entry_row.set_text(""),
        }
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
