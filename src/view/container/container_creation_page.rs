use std::cell::RefCell;

use adw::subclass::prelude::*;
use adw::traits::ActionRowExt;
use adw::traits::BinExt;
use adw::traits::ComboRowExt;
use adw::traits::ExpanderRowExt;
use futures::TryFutureExt;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;
use serde::Serialize;

use crate::model;
use crate::model::AbstractContainerListExt;
use crate::podman;
use crate::utils;
use crate::utils::ToTypedListModel;
use crate::view;
use crate::window::Window;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/container-creation-page.ui")]
    pub(crate) struct ContainerCreationPage {
        pub(super) client: WeakRef<model::Client>,
        pub(super) image: WeakRef<model::Image>,
        pub(super) pod: WeakRef<model::Pod>,
        pub(super) port_mappings: RefCell<gio::ListStore>,
        pub(super) volumes: RefCell<gio::ListStore>,
        pub(super) env_vars: RefCell<gio::ListStore>,
        pub(super) command_row_handler:
            RefCell<Option<(glib::SignalHandlerId, WeakRef<model::Image>)>>,
        #[template_child]
        pub(super) leaflet: TemplateChild<adw::Leaflet>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
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
        pub(super) terminal_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) memory_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) mem_value: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub(super) mem_combo_box: TemplateChild<gtk::ComboBoxText>,
        #[template_child]
        pub(super) port_mapping_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) volume_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) env_var_list_box: TemplateChild<gtk::ListBox>,
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
        pub(super) image_pulling_page: TemplateChild<view::ImagePullingPage>,
        #[template_child]
        pub(super) container_details_page_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub(super) leaflet_overlay: TemplateChild<view::LeafletOverlay>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerCreationPage {
        const NAME: &'static str = "ContainerCreationPage";
        type Type = super::ContainerCreationPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            klass.install_action("navigation.go-first", None, move |widget, _, _| {
                widget.navigate_to_first();
            });
            klass.install_action("navigation.back", None, move |widget, _, _| {
                widget.navigate_back();
            });

            klass.install_action("image.search", None, move |widget, _, _| {
                widget.search_image();
            });
            klass.install_action("image.remove-remote", None, move |widget, _, _| {
                widget.remove_remote();
            });

            klass.install_action("container.add-port-mapping", None, |widget, _, _| {
                widget.add_port_mapping();
            });
            klass.install_action("container.add-volume", None, |widget, _, _| {
                widget.add_volume();
            });
            klass.install_action("container.add-env-var", None, |widget, _, _| {
                widget.add_env_var();
            });
            klass.install_action("container.create-and-run", None, |widget, _, _| {
                widget.finish(true);
            });
            klass.install_action("container.create", None, |widget, _, _| {
                widget.finish(false);
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerCreationPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::new(
                        "client",
                        "Client",
                        "The client of this ContainerCreationPage",
                        model::Client::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecObject::new(
                        "image",
                        "Image",
                        "The image of this ContainerCreationPage",
                        model::Image::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecObject::new(
                        "pod",
                        "Pod",
                        "The pod of this ContainerCreationPage",
                        model::Pod::static_type(),
                        glib::ParamFlags::READWRITE
                            | glib::ParamFlags::CONSTRUCT
                            | glib::ParamFlags::EXPLICIT_NOTIFY,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "client" => self.client.set(value.get().unwrap()),
                "image" => self.image.set(value.get().unwrap()),
                "pod" => obj.set_pod(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "client" => obj.client().to_value(),
                "image" => obj.image().to_value(),
                "pod" => obj.pod().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            self.name_entry_row
                .connect_text_notify(clone!(@weak obj => move |_| obj.on_name_changed()));

            let image_tag_expr = model::Image::this_expression("repo-tags")
                .chain_closure::<String>(closure!(
                    |_: glib::Object, repo_tags: utils::BoxedStringVec| {
                        utils::escape(&utils::format_option(repo_tags.iter().next()))
                    }
                ));
            let pod_name_expr = model::Pod::this_expression("name");

            if let Some(image) = obj.image() {
                self.local_image_combo_row.set_visible(false);

                image_tag_expr.bind(&*self.local_image_property_row, "value", Some(&image));

                match image.details().map(model::ImageDetails::config) {
                    Some(config) => {
                        self.command_entry_row.set_text(config.cmd().unwrap_or(""));
                        obj.set_exposed_ports(config);
                    }
                    None => {
                        image.connect_notify_local(
                            Some("details"),
                            clone!(@weak obj => move |image, _| {
                                let config = image.details().unwrap().config();
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

            self.port_mapping_list_box
                .bind_model(Some(&*self.port_mappings.borrow()), |item| {
                    view::PortMappingRow::from(item.downcast_ref::<model::PortMapping>().unwrap())
                        .upcast()
                });
            self.port_mapping_list_box.append(
                &gtk::ListBoxRow::builder()
                    .action_name("container.add-port-mapping")
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
                    .action_name("container.add-volume")
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
                    view::EnvVarRow::from(item.downcast_ref::<model::EnvVar>().unwrap()).upcast()
                });
            self.env_var_list_box.append(
                &gtk::ListBoxRow::builder()
                    .action_name("container.add-env-var")
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

        fn dispose(&self, _obj: &Self::Type) {
            self.leaflet.unparent();
        }
    }

    impl WidgetImpl for ContainerCreationPage {
        fn realize(&self, widget: &Self::Type) {
            self.parent_realize(widget);

            widget.action_set_enabled(
                "navigation.go-first",
                widget.previous_leaflet_overlay() != widget.root_leaflet_overlay(),
            );
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainerCreationPage(ObjectSubclass<imp::ContainerCreationPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Image> for ContainerCreationPage {
    fn from(image: &model::Image) -> Self {
        glib::Object::new(&[("image", &image)]).expect("Failed to create ContainerCreationPage")
    }
}

impl From<&model::Pod> for ContainerCreationPage {
    fn from(pod: &model::Pod) -> Self {
        glib::Object::new(&[("pod", &pod)]).expect("Failed to create ContainerCreationPage")
    }
}

impl From<Option<&model::Client>> for ContainerCreationPage {
    fn from(client: Option<&model::Client>) -> Self {
        glib::Object::new(&[("client", &client)]).expect("Failed to create ContainerCreationPage")
    }
}

impl ContainerCreationPage {
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
        self.action_set_enabled("container.create-and-run", enabled);
        self.action_set_enabled("container.create", enabled);
    }

    fn navigate_to_first(&self) {
        self.root_leaflet_overlay().hide_details();
    }

    fn navigate_back(&self) {
        self.previous_leaflet_overlay().hide_details();
    }

    fn previous_leaflet_overlay(&self) -> view::LeafletOverlay {
        utils::find_parent_leaflet_overlay(self)
    }

    fn root_leaflet_overlay(&self) -> view::LeafletOverlay {
        self.root()
            .unwrap()
            .downcast::<Window>()
            .unwrap()
            .leaflet_overlay()
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
            Some(image) => match image.details() {
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
                                image.details().unwrap().config().cmd().unwrap_or("")
                            );
                        }),
                    );
                    let image_weak = WeakRef::new();
                    image_weak.set(Some(image));
                    imp.command_row_handler.replace(Some((handler, image_weak)));

                    image.load_details()
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

    fn add_env_var(&self) {
        let env_var = model::EnvVar::default();
        self.connect_env_var(&env_var);

        self.imp().env_vars.borrow().append(&env_var);
    }

    fn connect_env_var(&self, env_var: &model::EnvVar) {
        env_var.connect_remove_request(clone!(@weak self as obj => move |env_var| {
            let imp = obj.imp();

            let env_vars = imp.env_vars.borrow();
            if let Some(pos) = env_vars.find(env_var) {
                env_vars.remove(pos);
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
                self.create(image.id(), run);
            }
        } else {
            imp.stack.set_visible_child_name("creation-settings");

            log::error!("Error while starting container: no image selected");
            utils::show_error_toast(self, "Failed to create container", "no image selected")
        }
    }

    fn pull_and_create(&self, reference: &str, run: bool) {
        let imp = self.imp();
        imp.stack.set_visible_child(&*imp.image_pulling_page);

        let opts = podman::opts::PullOpts::builder()
            .reference(reference)
            .quiet(false)
            .build();

        imp.image_pulling_page.pull(
            opts,
            clone!(@weak self as obj => move |result| match result {
                Ok(report) => obj.create(&report.id.unwrap(), run),
                Err(e) => obj.on_pull_error(&e.to_string())
            }),
        );
    }

    fn on_pull_error(&self, error: &str) {
        self.imp().stack.set_visible_child_name("creation-settings");
        log::error!("Error while pulling newest image: {}", error);
        utils::show_error_toast(self, "Error while pulling newest image", error);
    }

    fn create(&self, image_id: &str, run: bool) {
        let imp = self.imp();

        let create_opts = podman::opts::ContainerCreateOpts::builder()
            .name(imp.name_entry_row.text().as_str())
            .image(&image_id)
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
                    .to_typed_list_model::<model::EnvVar>()
                    .into_iter()
                    .map(|env_var| (env_var.key(), env_var.value())),
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
                            * 1000_i64.pow(imp.mem_combo_box.active().map(|i| i + 1).unwrap_or(0)),
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
            create_opts.command([cmd.as_str()])
        };

        let healthcheck_cmd = imp.health_check_command_entry_row.text();
        let opts = if healthcheck_cmd.is_empty() {
            create_opts
        } else {
            create_opts.health_config(podman::models::Schema2HealthConfig {
                interval: imp.health_check_interval_value.value() as i64 * 1_000_000_000,
                retries: Some(imp.health_check_retries_value.value() as i64),
                start_period: imp.health_check_start_period_value.value() as i64 * 1_000_000_000,
                test: Some(
                    healthcheck_cmd
                        .split(' ')
                        .map(str::to_string)
                        .collect::<Vec<_>>(),
                ),
                timeout: imp.health_check_timeout_value.value() as i64 * 1_000_000_000,
            })
        }
        .build();

        imp.stack.set_visible_child_name("creating");

        utils::do_async(
            {
                let podman = self.client().unwrap().podman().clone();
                async move { podman.containers().create(&opts).await }
            },
            clone!(@weak self as obj => move |result| {
                match result.map(|info| info.id) {
                    Ok(id) => {
                        let client = obj.client().unwrap();
                        match client.container_list().get_container(&id) {
                            Some(container) => obj.switch_to_container(&container),
                            None => {
                                client.container_list().connect_container_added(
                                    clone!(@weak obj, @strong id => move |_, container| {
                                        if container.id() == id.as_str() {
                                            obj.switch_to_container(container);
                                        }
                                    }),
                                );
                            }
                        }

                        if run {
                            utils::do_async(
                                {
                                    let podman = obj.client().unwrap().podman().clone();
                                    async move {
                                        podman
                                            .containers()
                                            .get(id.clone())
                                            .start(None)
                                            .map_ok(|_| id)
                                            .await
                                    }
                                },
                                clone!(@weak obj => move |result| if let Err(e) = result {
                                    log::error!("Error while starting container: {}", e);
                                    utils::show_error_toast(
                                        &obj,
                                        "Error while starting container",
                                        &e.to_string()
                                    );
                                }),
                            );
                        }
                    }
                    Err(e) => {
                        obj.imp().stack.set_visible_child_name("creation-settings");
                        log::error!("Error while creating container: {}", e);
                        utils::show_error_toast(
                            &obj,
                            "Error while creating container",
                            &e.to_string()
                        );
                    }
                }
            }),
        );
    }

    fn switch_to_container(&self, container: &model::Container) {
        let imp = self.imp();
        imp.container_details_page_bin
            .set_child(Some(&view::ContainerDetailsPage::from(container)));
        imp.stack
            .set_visible_child(&*imp.container_details_page_bin);
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
