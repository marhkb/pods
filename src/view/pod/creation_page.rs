use std::cell::RefCell;

use adw::subclass::prelude::*;
use adw::traits::ActionRowExt;
use adw::traits::ComboRowExt;
use gettextrs::gettext;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::podman;
use crate::utils;
use crate::utils::ToTypedListModel;
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
        pub(super) labels: RefCell<gio::ListStore>,
        pub(super) hosts: RefCell<gio::ListStore>,
        pub(super) devices: RefCell<gio::ListStore>,
        pub(super) pod_create_cmd_args: RefCell<gio::ListStore>,
        pub(super) infra_cmd_args: RefCell<gio::ListStore>,
        pub(super) command_row_handler:
            RefCell<Option<(glib::SignalHandlerId, glib::WeakRef<model::Image>)>>,
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
        pub(super) infra_local_image_combo_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub(super) infra_remote_image_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) infra_common_pid_file_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) infra_command_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) infra_command_arg_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) create_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) leaflet_overlay: TemplateChild<view::LeafletOverlay>,
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
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
                        .build(),
                    glib::ParamSpecObject::builder::<model::Image>("infra-image")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
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
            let obj = &*self.instance();
            match pspec.name() {
                "client" => obj.client().to_value(),
                "infra-image" => obj.image().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.instance();

            self.name_entry_row
                .connect_text_notify(clone!(@weak obj => move |_| obj.on_name_changed()));

            self.labels_list_box
                .bind_model(Some(&*self.labels.borrow()), |item| {
                    view::KeyValRow::from(item.downcast_ref::<model::KeyVal>().unwrap()).upcast()
                });
            self.labels_list_box.append(
                &gtk::ListBoxRow::builder()
                    .action_name(ACTION_ADD_LABEL)
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

            self.hosts_list_box
                .bind_model(Some(&*self.hosts.borrow()), |item| {
                    view::KeyValRow::new(
                        gettext("Hostname"),
                        gettext("IP"),
                        item.downcast_ref::<model::KeyVal>().unwrap(),
                    )
                    .upcast()
                });
            self.hosts_list_box.append(
                &gtk::ListBoxRow::builder()
                    .action_name(ACTION_ADD_HOST)
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

            self.devices_list_box
                .bind_model(Some(&*self.devices.borrow()), |item| {
                    view::DeviceRow::from(item.downcast_ref::<model::Device>().unwrap()).upcast()
                });
            self.devices_list_box.append(
                &gtk::ListBoxRow::builder()
                    .action_name(ACTION_ADD_DEVICE)
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

            self.pod_create_command_arg_list_box.bind_model(
                Some(&*self.pod_create_cmd_args.borrow()),
                |item| {
                    view::ValueRow::new(item.downcast_ref().unwrap(), gettext("Argument")).upcast()
                },
            );
            self.pod_create_command_arg_list_box.append(
                &gtk::ListBoxRow::builder()
                    .action_name(ACTION_ADD_POD_CREATE_CMD_ARGS)
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

            self.infra_command_arg_list_box.bind_model(
                Some(&*self.infra_cmd_args.borrow()),
                |item| {
                    view::ValueRow::new(item.downcast_ref().unwrap(), gettext("Argument")).upcast()
                },
            );
            self.infra_command_arg_list_box.append(
                &gtk::ListBoxRow::builder()
                    .action_name(ACTION_ADD_INFRA_CMD_ARGS)
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
            let image_tag_expr = model::Image::this_expression("repo-tags")
                .chain_closure::<String>(closure!(
                    |_: glib::Object, repo_tags: utils::BoxedStringVec| {
                        utils::escape(&utils::format_option(repo_tags.iter().next()))
                    }
                ));

            let filter_model = gtk::FilterListModel::new(
                Some(obj.client().unwrap().image_list()),
                Some(&gtk::CustomFilter::new(|obj| {
                    obj.downcast_ref::<model::Image>()
                        .unwrap()
                        .repo_tags()
                        .first()
                        .is_some()
                })),
            );

            self.infra_local_image_combo_row
                .set_model(Some(&filter_model));
            self.infra_local_image_combo_row
                .set_expression(Some(&image_tag_expr));
            self.infra_local_image_combo_row
                .connect_selected_item_notify(
                    clone!(@weak obj => move |_| obj.update_infra_command_row()),
                );
            obj.update_infra_command_row();
        }

        fn dispose(&self) {
            utils::ChildIter::from(&*self.instance()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for CreationPage {
        fn root(&self) {
            self.parent_root();

            let widget = &*self.instance();

            glib::idle_add_local(
                clone!(@weak widget => @default-return glib::Continue(false), move || {
                    widget.imp().name_entry_row.grab_focus();
                    glib::Continue(false)
                }),
            );
            utils::root(widget).set_default_widget(Some(&*self.create_button));
        }

        fn unroot(&self) {
            utils::root(&*self.instance()).set_default_widget(gtk::Widget::NONE);
            self.parent_unroot()
        }
    }
}

glib::wrapper! {
    pub(crate) struct CreationPage(ObjectSubclass<imp::CreationPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<Option<&model::Client>> for CreationPage {
    fn from(client: Option<&model::Client>) -> Self {
        glib::Object::new::<Self>(&[("client", &client)])
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
        let label = model::KeyVal::default();
        self.connect_label(&label);

        self.imp().labels.borrow().append(&label);
    }

    fn connect_label(&self, label: &model::KeyVal) {
        label.connect_remove_request(clone!(@weak self as obj => move |label| {
            let imp = obj.imp();

            let labels = imp.labels.borrow();
            if let Some(pos) = labels.find(label) {
                labels.remove(pos);
            }
        }));
    }

    fn add_host(&self) {
        let host = model::KeyVal::default();
        self.connect_host(&host);

        self.imp().hosts.borrow().append(&host);
    }

    fn connect_host(&self, host: &model::KeyVal) {
        host.connect_remove_request(clone!(@weak self as obj => move |host| {
            let imp = obj.imp();

            let hosts = imp.hosts.borrow();
            if let Some(pos) = hosts.find(host) {
                hosts.remove(pos);
            }
        }));
    }

    fn add_device(&self) {
        let device = model::Device::default();
        self.connect_device(&device);

        self.imp().devices.borrow().append(&device);
    }

    fn connect_device(&self, device: &model::Device) {
        device.connect_remove_request(clone!(@weak self as obj => move |device| {
            let imp = obj.imp();

            let devices = imp.devices.borrow();
            if let Some(pos) = devices.find(device) {
                devices.remove(pos);
            }
        }));
    }

    fn add_pod_create_cmd_arg(&self) {
        let arg = model::Value::default();
        self.connect_pod_create_cmd_arg(&arg);

        self.imp().pod_create_cmd_args.borrow().append(&arg);
    }

    fn connect_pod_create_cmd_arg(&self, cmd_arg: &model::Value) {
        cmd_arg.connect_remove_request(clone!(@weak self as obj => move |cmd_arg| {
            let imp = obj.imp();

            let cmd_args = imp.pod_create_cmd_args.borrow();
            if let Some(pos) = cmd_args.find(cmd_arg) {
                cmd_args.remove(pos);
            }
        }));
    }

    fn add_infra_cmd_arg(&self) {
        let arg = model::Value::default();
        self.connect_infra_cmd_arg(&arg);

        self.imp().infra_cmd_args.borrow().append(&arg);
    }

    fn connect_infra_cmd_arg(&self, cmd_arg: &model::Value) {
        cmd_arg.connect_remove_request(clone!(@weak self as obj => move |cmd_arg| {
            let imp = obj.imp();

            let cmd_args = imp.infra_cmd_args.borrow();
            if let Some(pos) = cmd_args.find(cmd_arg) {
                cmd_args.remove(pos);
            }
        }));
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
                self.pull_and_create(image.repo_tags().first().unwrap());
            } else {
                let page =
                    view::ActionPage::from(&self.client().unwrap().action_list().create_pod(
                        imp.name_entry_row.text().as_str(),
                        self.create().infra_image(image.id()).build(),
                    ));

                imp.leaflet_overlay.show_details(&page);
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

        imp.leaflet_overlay.show_details(&page);
    }

    fn create(&self) -> podman::opts::PodCreateOptsBuilder {
        let imp = self.imp();

        let mut opts = podman::opts::PodCreateOpts::builder()
            .name(imp.name_entry_row.text().as_str())
            .hostname(imp.hostname_entry_row.text().as_str())
            .labels(
                imp.labels
                    .borrow()
                    .to_owned()
                    .to_typed_list_model::<model::KeyVal>()
                    .into_iter()
                    .map(|label| (label.key(), label.value())),
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
                    .borrow()
                    .to_owned()
                    .to_typed_list_model::<model::Value>()
                    .into_iter()
                    .map(|arg| arg.value());
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
                    .borrow()
                    .to_owned()
                    .to_typed_list_model::<model::KeyVal>()
                    .into_iter()
                    .map(|host| format!("{}:{}", host.key(), host.value())),
            )
        } else {
            opts = opts.no_manage_hosts(true);
        }

        let create_cmd = imp.pod_create_command_entry_row.text();
        if !create_cmd.is_empty() {
            let args = imp
                .pod_create_cmd_args
                .borrow()
                .to_owned()
                .to_typed_list_model::<model::Value>()
                .into_iter()
                .map(|arg| arg.value());
            let mut cmd = vec![create_cmd.to_string()];
            cmd.extend(args);
            opts = opts.pod_create_command(cmd);
        }

        let devices: Vec<_> = imp
            .devices
            .borrow()
            .to_owned()
            .to_typed_list_model::<model::Device>()
            .into_iter()
            .map(|dev| {
                format!(
                    "{}:{}:{}{}{}",
                    dev.host_path(),
                    dev.container_path(),
                    if dev.readable() { "r" } else { "" },
                    if dev.writable() { "w" } else { "" },
                    if dev.mknod() { "m" } else { "" },
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
    }

    fn search_image(&self) {
        let image_selection_page = view::ImageSelectionPage::from(self.client().as_ref());
        image_selection_page.connect_image_selected(clone!(@weak self as obj => move |_, image| {
            let imp = obj.imp();

            imp.infra_local_image_combo_row.set_visible(false);
            imp.infra_remote_image_row.set_visible(true);
            imp.infra_remote_image_row.set_subtitle(&image);

            imp.infra_command_entry_row.set_text("");
        }));
        self.imp()
            .leaflet_overlay
            .show_details(&image_selection_page);
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
