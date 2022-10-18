use std::cell::RefCell;

use adw::subclass::prelude::*;
use adw::traits::ActionRowExt;
use adw::traits::ComboRowExt;
use adw::traits::ExpanderRowExt;
use gettextrs::gettext;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;
use serde::Serialize;

use crate::model;
use crate::podman;
use crate::utils;
use crate::utils::ToTypedListModel;
use crate::view;

const ACTION_SEARCH_IMAGE: &str = "container-creation-page.search-image";
const ACTION_REMOVE_REMOTE_IMAGE: &str = "container-creation-page.remove-remote-image";
const ACTION_ADD_CMD_ARG: &str = "container-creation-page.add-cmd-arg";
const ACTION_ADD_PORT_MAPPING: &str = "container-creation-page.add-port-mapping";
const ACTION_ADD_VOLUME: &str = "container-creation-page.add-volume";
const ACTION_ADD_ENV_VAR: &str = "container-creation-page.add-env-var";
const ACTION_ADD_LABEL: &str = "container-creation-page.add-label";
const ACTION_CREATE_AND_RUN: &str = "container-creation-page.create-and-run";
const ACTION_CREATE: &str = "container-creation-page.create";

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/container/creation-page.ui")]
    pub(crate) struct CreationPage {
        pub(super) client: glib::WeakRef<model::Client>,
        pub(super) image: glib::WeakRef<model::Image>,
        pub(super) pod: glib::WeakRef<model::Pod>,
        pub(super) port_mappings: RefCell<gio::ListStore>,
        pub(super) volumes: RefCell<gio::ListStore>,
        pub(super) env_vars: RefCell<gio::ListStore>,
        pub(super) cmd_args: RefCell<gio::ListStore>,
        pub(super) labels: RefCell<gio::ListStore>,
        pub(super) command_row_handler:
            RefCell<Option<(glib::SignalHandlerId, glib::WeakRef<model::Image>)>>,
        #[template_child]
        pub(super) name_entry_row: TemplateChild<view::RandomNameEntryRow>,
        #[template_child]
        pub(super) local_image_property_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) local_image_combo_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub(super) remote_image_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) pod_property_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) pod_expander_row: TemplateChild<adw::ExpanderRow>,
        #[template_child]
        pub(super) pod_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) pod_combo_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub(super) pull_latest_image_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) pull_latest_image_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) command_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) command_arg_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) terminal_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) memory_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) mem_value: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub(super) mem_drop_down: TemplateChild<gtk::DropDown>,
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
        #[template_child]
        pub(super) crate_and_run_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) leaflet_overlay: TemplateChild<view::LeafletOverlay>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CreationPage {
        const NAME: &'static str = "PdsContainerCreationPage";
        type Type = super::CreationPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action(ACTION_SEARCH_IMAGE, None, move |widget, _, _| {
                widget.search_image();
            });
            klass.install_action(ACTION_REMOVE_REMOTE_IMAGE, None, move |widget, _, _| {
                widget.remove_remote();
            });
            klass.install_action(ACTION_ADD_CMD_ARG, None, |widget, _, _| {
                widget.add_cmd_arg();
            });

            klass.install_action(ACTION_ADD_PORT_MAPPING, None, |widget, _, _| {
                widget.add_port_mapping();
            });
            klass.install_action(ACTION_ADD_VOLUME, None, |widget, _, _| {
                widget.add_volume();
            });
            klass.install_action(ACTION_ADD_ENV_VAR, None, |widget, _, _| {
                widget.add_env_var();
            });

            klass.install_action(ACTION_ADD_LABEL, None, |widget, _, _| {
                widget.add_label();
            });
            klass.install_action(ACTION_CREATE_AND_RUN, None, |widget, _, _| {
                widget.finish(true);
            });
            klass.install_action(ACTION_CREATE, None, |widget, _, _| {
                widget.finish(false);
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
                    glib::ParamSpecObject::builder::<model::Image>("image")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
                        .build(),
                    glib::ParamSpecObject::builder::<model::Pod>("pod")
                        .flags(
                            glib::ParamFlags::READWRITE
                                | glib::ParamFlags::CONSTRUCT
                                | glib::ParamFlags::EXPLICIT_NOTIFY,
                        )
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "client" => self.client.set(value.get().unwrap()),
                "image" => self.image.set(value.get().unwrap()),
                "pod" => self.instance().set_pod(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            let obj = &*self.instance();
            match pspec.name() {
                "client" => obj.client().to_value(),
                "image" => obj.image().to_value(),
                "pod" => obj.pod().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.instance();

            self.name_entry_row
                .connect_text_notify(clone!(@weak obj => move |_| obj.on_name_changed()));

            let image_tag_expr = model::Image::this_expression("repo-tags")
                .chain_closure::<String>(closure!(
                    |_: model::Image, repo_tags: utils::BoxedStringVec| {
                        utils::escape(&utils::format_option(repo_tags.iter().next()))
                    }
                ));
            let pod_name_expr = model::Pod::this_expression("name");

            if let Some(image) = obj.image() {
                self.local_image_combo_row.set_visible(false);

                image_tag_expr.bind(&*self.local_image_property_row, "value", Some(&image));

                match image.data().map(model::ImageData::config) {
                    Some(config) => {
                        self.command_entry_row.set_text(config.cmd().unwrap_or(""));
                        obj.set_exposed_ports(config);
                    }
                    None => {
                        image.connect_notify_local(
                            Some("details"),
                            clone!(@weak obj => move |image, _| {
                                let config = image.data().unwrap().config();
                                obj.imp().command_entry_row.set_text(config.cmd().unwrap_or(""));
                                obj.set_exposed_ports(config);
                            }),
                        );
                    }
                }
            } else {
                self.local_image_property_row.set_visible(false);

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

                self.local_image_combo_row.set_model(Some(&filter_model));
                self.local_image_combo_row
                    .set_expression(Some(&image_tag_expr));
                self.local_image_combo_row.connect_selected_item_notify(
                    clone!(@weak obj => move |_| obj.update_command_row()),
                );
                obj.update_command_row();
            }

            if let Some(pod) = obj.pod() {
                self.pod_expander_row.set_visible(false);
                pod_name_expr.bind(&*self.pod_property_row, "value", Some(&pod));
            } else {
                self.pod_property_row.set_visible(false);

                obj.connect_notify_local(Some("pod"), |obj, _| {
                    obj.imp().pod_expander_row.set_subtitle(
                        &obj.pod().as_ref().map(model::Pod::name).unwrap_or_default(),
                    );
                });

                self.pod_switch
                    .connect_active_notify(clone!(@weak obj => move |_| {
                        obj.imp().pod_combo_row.notify("selected-item");
                    }));

                self.pod_combo_row.connect_selected_item_notify(
                    clone!(@weak obj => move |combo_row| {
                        obj.set_pod(
                            if obj.imp().pod_switch.is_active() {
                                combo_row.selected_item().and_then(|o| o.downcast().ok())
                            } else {
                                None
                            }
                            .as_ref(),
                        );
                    }),
                );

                self.pod_combo_row.set_expression(Some(&pod_name_expr));
                self.pod_combo_row
                    .set_model(Some(obj.client().unwrap().pod_list()));
            }
            self.command_arg_list_box
                .bind_model(Some(&*self.cmd_args.borrow()), |item| {
                    view::ValueRow::new(item.downcast_ref().unwrap(), gettext("Argument")).upcast()
                });
            self.command_arg_list_box.append(
                &gtk::ListBoxRow::builder()
                    .action_name(ACTION_ADD_CMD_ARG)
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

            self.port_mapping_list_box
                .bind_model(Some(&*self.port_mappings.borrow()), |item| {
                    view::PortMappingRow::from(item.downcast_ref::<model::PortMapping>().unwrap())
                        .upcast()
                });
            self.port_mapping_list_box.append(
                &gtk::ListBoxRow::builder()
                    .action_name(ACTION_ADD_PORT_MAPPING)
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

            self.volume_list_box
                .bind_model(Some(&*self.volumes.borrow()), |item| {
                    view::VolumeRow::from(item.downcast_ref::<model::Volume>().unwrap()).upcast()
                });
            self.volume_list_box.append(
                &gtk::ListBoxRow::builder()
                    .action_name(ACTION_ADD_VOLUME)
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

            self.env_var_list_box
                .bind_model(Some(&*self.env_vars.borrow()), |item| {
                    view::KeyValRow::from(item.downcast_ref::<model::KeyVal>().unwrap()).upcast()
                });
            self.env_var_list_box.append(
                &gtk::ListBoxRow::builder()
                    .action_name(ACTION_ADD_ENV_VAR)
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
            utils::root(widget).set_default_widget(Some(&*self.crate_and_run_button));
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

impl From<&model::Image> for CreationPage {
    fn from(image: &model::Image) -> Self {
        glib::Object::new::<Self>(&[("image", &image)])
    }
}

impl From<&model::Pod> for CreationPage {
    fn from(pod: &model::Pod) -> Self {
        glib::Object::new::<Self>(&[("pod", &pod)])
    }
}

impl From<Option<&model::Client>> for CreationPage {
    fn from(client: Option<&model::Client>) -> Self {
        glib::Object::new::<Self>(&[("client", &client)])
    }
}

impl CreationPage {
    fn client(&self) -> Option<model::Client> {
        self.imp()
            .client
            .upgrade()
            .or_else(|| {
                self.image()
                    .as_ref()
                    .and_then(model::Image::image_list)
                    .as_ref()
                    .and_then(model::ImageList::client)
            })
            .or_else(|| {
                self.pod()
                    .as_ref()
                    .and_then(model::Pod::pod_list)
                    .as_ref()
                    .and_then(model::PodList::client)
            })
    }

    fn image(&self) -> Option<model::Image> {
        self.imp().image.upgrade()
    }

    fn pod(&self) -> Option<model::Pod> {
        self.imp().pod.upgrade()
    }

    fn set_pod(&self, value: Option<&model::Pod>) {
        if self.pod().as_ref() == value {
            return;
        }
        self.imp().pod.set(value);
        self.notify("pod");
    }

    fn on_name_changed(&self) {
        let enabled = self.imp().name_entry_row.text().len() > 0;
        self.action_set_enabled(ACTION_CREATE_AND_RUN, enabled);
        self.action_set_enabled(ACTION_CREATE, enabled);
    }

    fn set_exposed_ports(&self, config: &model::ImageConfig) {
        let imp = self.imp();

        config.exposed_ports().iter().for_each(|exposed| {
            let port_mapping = model::PortMapping::default();
            self.connect_port_mapping(&port_mapping);
            imp.port_mappings.borrow().append(&port_mapping);
            imp.port_mapping_list_box.set_visible(true);

            let mut split = exposed.split_terminator('/');
            if let Some(port) = split.next() {
                match port.parse() {
                    Ok(port) => {
                        port_mapping.set_host_port(port);
                        port_mapping.set_container_port(port);
                    }
                    Err(e) => log::warn!("Error on parsing port: {e}"),
                }

                if let Some(protocol) = split.next() {
                    match protocol {
                        "tcp" => port_mapping.set_protocol(model::PortMappingProtocol::Tcp),
                        "udp" => port_mapping.set_protocol(model::PortMappingProtocol::Udp),
                        _ => log::warn!("Unknown protocol: {protocol}"),
                    }
                }
            }
        });
    }

    fn remove_remote(&self) {
        let imp = self.imp();
        imp.remote_image_row.set_subtitle("");
        imp.remote_image_row.set_visible(false);
        imp.local_image_combo_row.set_visible(true);
        imp.pull_latest_image_row.set_visible(true);

        self.update_command_row();
    }

    fn update_command_row(&self) {
        let imp = self.imp();

        match imp
            .local_image_combo_row
            .selected_item()
            .as_ref()
            .map(|item| item.downcast_ref::<model::Image>().unwrap())
        {
            Some(image) => match image.data() {
                Some(details) => imp
                    .command_entry_row
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
                            obj.imp().command_entry_row.set_text(
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
            None => imp.command_entry_row.set_text(""),
        }
    }

    fn search_image(&self) {
        let image_selection_page = view::ImageSelectionPage::from(self.client().as_ref());
        image_selection_page.connect_image_selected(clone!(@weak self as obj => move |_, image| {
            let imp = obj.imp();

            imp.local_image_combo_row.set_visible(false);
            imp.remote_image_row.set_visible(true);
            imp.remote_image_row.set_subtitle(&image);
            imp.pull_latest_image_row.set_visible(false);

            imp.command_entry_row.set_text("");
        }));
        self.imp()
            .leaflet_overlay
            .show_details(&image_selection_page);
    }

    fn add_port_mapping(&self) {
        let port_mapping = model::PortMapping::default();
        self.connect_port_mapping(&port_mapping);

        self.imp().port_mappings.borrow().append(&port_mapping);
    }

    fn connect_port_mapping(&self, port_mapping: &model::PortMapping) {
        port_mapping.connect_remove_request(clone!(@weak self as obj => move |port_mapping| {
            let imp = obj.imp();

            let port_mappings = imp.port_mappings.borrow();
            if let Some(pos) = port_mappings.find(port_mapping) {
                port_mappings.remove(pos);
            }
        }));
    }

    fn add_volume(&self) {
        let volume = model::Volume::default();
        self.connect_volume(&volume);

        self.imp().volumes.borrow().append(&volume);
    }

    fn connect_volume(&self, volume: &model::Volume) {
        volume.connect_remove_request(clone!(@weak self as obj => move |volume| {
            let imp = obj.imp();

            let volumes = imp.volumes.borrow();
            if let Some(pos) = volumes.find(volume) {
                volumes.remove(pos);
            }
        }));
    }

    fn add_cmd_arg(&self) {
        let arg = model::Value::default();
        self.connect_cmd_arg(&arg);

        self.imp().cmd_args.borrow().append(&arg);
    }

    fn connect_cmd_arg(&self, cmd_arg: &model::Value) {
        cmd_arg.connect_remove_request(clone!(@weak self as obj => move |cmd_arg| {
            let imp = obj.imp();

            let cmd_args = imp.cmd_args.borrow();
            if let Some(pos) = cmd_args.find(cmd_arg) {
                cmd_args.remove(pos);
            }
        }));
    }

    fn add_env_var(&self) {
        let env_var = model::KeyVal::default();
        self.connect_env_var(&env_var);

        self.imp().env_vars.borrow().append(&env_var);
    }

    fn connect_env_var(&self, env_var: &model::KeyVal) {
        env_var.connect_remove_request(clone!(@weak self as obj => move |env_var| {
            let imp = obj.imp();

            let env_vars = imp.env_vars.borrow();
            if let Some(pos) = env_vars.find(env_var) {
                env_vars.remove(pos);
            }
        }));
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

    fn finish(&self, run: bool) {
        let imp = self.imp();

        if imp.remote_image_row.is_visible() {
            self.pull_and_create(imp.remote_image_row.subtitle().unwrap().as_str(), run);
        } else if let Some(image) = self.image().or_else(|| {
            imp.local_image_combo_row
                .selected_item()
                .map(|item| item.downcast().unwrap())
        }) {
            if imp.pull_latest_image_switch.is_active() {
                self.pull_and_create(image.repo_tags().first().unwrap(), run);
            } else {
                let page = view::ActionPage::from(
                    &self.client().unwrap().action_list().create_container(
                        imp.name_entry_row.text().as_str(),
                        image
                            .repo_tags()
                            .first()
                            .map(String::as_str)
                            .unwrap_or_else(|| image.id()),
                        self.create().image(image.id()).build(),
                        run,
                    ),
                );

                imp.leaflet_overlay.show_details(&page);
            }
        } else {
            log::error!("Error while starting container: no image selected");
            utils::show_error_toast(
                self,
                &gettext("Failed to create container"),
                &gettext("no image selected"),
            )
        }
    }

    fn pull_and_create(&self, reference: &str, run: bool) {
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
                .create_container_download_image(
                    imp.name_entry_row.text().as_str(),
                    reference,
                    pull_opts,
                    self.create(),
                    run,
                ),
        );

        imp.leaflet_overlay.show_details(&page);
    }

    fn create(&self) -> podman::opts::ContainerCreateOptsBuilder {
        let imp = self.imp();

        let create_opts = podman::opts::ContainerCreateOpts::builder()
            .name(imp.name_entry_row.text().as_str())
            .pod(self.pod().as_ref().map(model::Pod::name))
            .terminal(imp.terminal_switch.is_active())
            .portmappings(
                imp.port_mappings
                    .borrow()
                    .to_owned()
                    .to_typed_list_model::<model::PortMapping>()
                    .into_iter()
                    .map(|port_mapping| podman::models::PortMapping {
                        container_port: Some(port_mapping.container_port() as u16),
                        host_ip: None,
                        host_port: Some(port_mapping.host_port() as u16),
                        protocol: Some(port_mapping.protocol().to_string()),
                        range: None,
                    }),
            )
            .mounts(
                imp.volumes
                    .borrow()
                    .to_owned()
                    .to_typed_list_model::<model::Volume>()
                    .into_iter()
                    .map(|volume| Mount {
                        destination: Some(volume.container_path()),
                        source: Some(volume.host_path()),
                        _type: Some("bind".to_owned()),
                        options: Some({
                            let mut options =
                                vec![if volume.writable() { "rw" } else { "ro" }.to_owned()];

                            let selinux = volume.selinux().to_string();
                            if !selinux.is_empty() {
                                options.push(selinux)
                            }

                            options
                        }),
                    }),
            )
            .env(
                imp.env_vars
                    .borrow()
                    .to_owned()
                    .to_typed_list_model::<model::KeyVal>()
                    .into_iter()
                    .map(|env_var| (env_var.key(), env_var.value())),
            )
            .labels(
                imp.labels
                    .borrow()
                    .to_owned()
                    .to_typed_list_model::<model::KeyVal>()
                    .into_iter()
                    .map(|label| (label.key(), label.value())),
            );

        let create_opts = if imp.memory_switch.is_active() {
            create_opts.resource_limits(podman::models::LinuxResources {
                block_io: None,
                cpu: None,
                devices: None,
                hugepage_limits: None,
                memory: Some(podman::models::LinuxMemory {
                    disable_oom_killer: None,
                    kernel: None,
                    kernel_tcp: None,
                    limit: Some(
                        imp.mem_value.value() as i64
                            * 1000_i64.pow(imp.mem_drop_down.selected() + 1),
                    ),
                    reservation: None,
                    swap: None,
                    swappiness: None,
                    use_hierarchy: None,
                }),
                network: None,
                pids: None,
                rdma: None,
                unified: None,
            })
        } else {
            create_opts
        };

        let cmd = imp.command_entry_row.text();
        let create_opts = if cmd.is_empty() {
            create_opts
        } else {
            let args = imp
                .cmd_args
                .borrow()
                .to_owned()
                .to_typed_list_model::<model::Value>()
                .into_iter()
                .map(|arg| arg.value());
            let mut cmd = vec![cmd.to_string()];
            cmd.extend(args);
            create_opts.command(&cmd)
        };

        let healthcheck_cmd = imp.health_check_command_entry_row.text();

        if healthcheck_cmd.is_empty() {
            create_opts
        } else {
            create_opts.health_config(podman::models::Schema2HealthConfig {
                interval: Some(imp.health_check_interval_value.value() as i64 * 1_000_000_000),
                retries: Some(imp.health_check_retries_value.value() as i64),
                start_period: Some(
                    imp.health_check_start_period_value.value() as i64 * 1_000_000_000,
                ),
                test: Some(
                    healthcheck_cmd
                        .split(' ')
                        .map(str::to_string)
                        .collect::<Vec<_>>(),
                ),
                timeout: Some(imp.health_check_timeout_value.value() as i64 * 1_000_000_000),
            })
        }
    }
}

/// It seems that `mount` in
/// https://docs.podman.io/en/latest/_static/api.html?version=v3.4#operation/ContainerCreateLibpod
/// describes the wrong datatype. Hence this is used instead
#[derive(Clone, Debug, Serialize)]
pub struct Mount {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub destination: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(rename = "Type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _type: Option<String>,
}
