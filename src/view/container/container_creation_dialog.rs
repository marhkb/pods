use std::cell::RefCell;

use adw::subclass::prelude::*;
use adw::traits::ComboRowExt;
use futures::TryFutureExt;
use gettextrs::gettext;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;
use once_cell::sync::OnceCell;
use serde::Serialize;

use crate::api;
use crate::model;
use crate::utils;
use crate::utils::ToTypedListModel;
use crate::view;
use crate::PODMAN;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/container-creation-dialog.ui")]
    pub(crate) struct ContainerCreationDialog {
        pub(super) names: RefCell<names::Generator<'static>>,
        pub(super) image: WeakRef<model::Image>,
        pub(super) image_list: WeakRef<model::ImageList>,
        pub(super) port_mappings: RefCell<gio::ListStore>,
        pub(super) volumes: RefCell<gio::ListStore>,
        pub(super) env_vars: RefCell<gio::ListStore>,
        pub(super) created_container_id: OnceCell<String>,
        #[template_child]
        pub(super) toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub(super) name_entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub(super) image_property_row: TemplateChild<view::PropertyRow>,
        #[template_child]
        pub(super) image_combo_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub(super) pull_latest_image_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) command_entry: TemplateChild<gtk::Entry>,
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
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerCreationDialog {
        const NAME: &'static str = "ContainerCreationDialog";
        type Type = super::ContainerCreationDialog;
        type ParentType = gtk::Dialog;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
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
                widget.create(true);
            });
            klass.install_action("container.create", None, |widget, _, _| {
                widget.create(false);
            });
            klass.install_action("dialog.close", None, |widget, _, _| {
                widget.close();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerCreationDialog {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::new(
                        "image",
                        "Image",
                        "The image of this ContainerCreationDialog",
                        model::Image::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecObject::new(
                        "image-list",
                        "Image List",
                        "The image list of this ContainerCreationDialog",
                        model::ImageList::static_type(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "image" => self.image.set(value.get().unwrap()),
                "image-list" => self.image_list.set(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "image" => obj.image().to_value(),
                "image-list" => self.image_list.upgrade().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            self.name_entry
                .set_text(&self.names.borrow_mut().next().unwrap());

            self.name_entry
                .connect_text_notify(clone!(@weak obj => move |_| obj.on_name_changed()));

            let image_tag_expr = model::Image::this_expression("repo-tags")
                .chain_closure::<String>(closure!(
                    |_: glib::Object, repo_tags: utils::BoxedStringVec| {
                        utils::escape(&utils::format_option(repo_tags.iter().next()))
                    }
                ));

            if let Some(image) = obj.image() {
                self.image_combo_row.set_visible(false);

                image_tag_expr.bind(&*self.image_property_row, "value", Some(&image));

                if let Some(cmd) = image.config().cmd() {
                    self.command_entry.set_text(cmd);
                }

                image.config().exposed_ports().iter().for_each(|exposed| {
                    let port_mapping = model::PortMapping::default();
                    obj.connect_port_mapping(&port_mapping);
                    self.port_mappings.borrow().append(&port_mapping);
                    self.port_mapping_list_box.set_visible(true);

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
            } else if let Some(image_list) = obj.image_list() {
                self.image_property_row.set_visible(false);

                self.image_combo_row.set_model(Some(&image_list));
                self.image_combo_row.set_expression(Some(&image_tag_expr));
            }

            self.port_mapping_list_box
                .bind_model(Some(&*self.port_mappings.borrow()), |item| {
                    view::PortMappingRow::from(item.downcast_ref::<model::PortMapping>().unwrap())
                        .upcast()
                });

            self.volume_list_box
                .bind_model(Some(&*self.volumes.borrow()), |item| {
                    view::VolumeRow::from(item.downcast_ref::<model::Volume>().unwrap()).upcast()
                });

            self.env_var_list_box
                .bind_model(Some(&*self.env_vars.borrow()), |item| {
                    view::EnvVarRow::from(item.downcast_ref::<model::EnvVar>().unwrap()).upcast()
                });
        }
    }

    impl WidgetImpl for ContainerCreationDialog {}
    impl WindowImpl for ContainerCreationDialog {}
    impl DialogImpl for ContainerCreationDialog {}
}

glib::wrapper! {
    pub(crate) struct ContainerCreationDialog(ObjectSubclass<imp::ContainerCreationDialog>)
        @extends gtk::Widget, gtk::Window, gtk::Dialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl From<Option<&model::Image>> for ContainerCreationDialog {
    fn from(image: Option<&model::Image>) -> Self {
        glib::Object::new(&[("image", &image), ("use-header-bar", &1)])
            .expect("Failed to create ContainerCreationDialog")
    }
}

impl From<Option<&model::ImageList>> for ContainerCreationDialog {
    fn from(image_list: Option<&model::ImageList>) -> Self {
        glib::Object::new(&[("image-list", &image_list), ("use-header-bar", &1)])
            .expect("Failed to create ContainerCreationDialog")
    }
}

impl ContainerCreationDialog {
    fn image(&self) -> Option<model::Image> {
        self.imp().image.upgrade()
    }

    fn image_list(&self) -> Option<model::ImageList> {
        self.imp().image_list.upgrade()
    }

    fn on_name_changed(&self) {
        let enabled = self.imp().name_entry.text().len() > 0;
        self.action_set_enabled("container.create-and-run", enabled);
        self.action_set_enabled("container.create", enabled);
    }

    fn show_toast(&self, title: &str) {
        self.imp().toast_overlay.add_toast(
            &adw::Toast::builder()
                .title(title)
                .timeout(3)
                .priority(adw::ToastPriority::High)
                .build(),
        );
    }

    fn add_port_mapping(&self) {
        let port_mapping = model::PortMapping::default();
        self.connect_port_mapping(&port_mapping);

        let imp = self.imp();

        imp.port_mappings.borrow().insert(0, &port_mapping);
        imp.port_mapping_list_box.set_visible(true);
    }

    fn connect_port_mapping(&self, port_mapping: &model::PortMapping) {
        port_mapping.connect_remove_request(clone!(@weak self as obj => move |port_mapping| {
            let imp = obj.imp();

            let port_mappings = imp.port_mappings.borrow();
            if let Some(pos) = port_mappings.find(port_mapping) {
                port_mappings.remove(pos);

                if port_mappings.n_items() == 0 {
                    imp.port_mapping_list_box.set_visible(false);
                }
            }
        }));
    }

    fn add_volume(&self) {
        let volume = model::Volume::default();
        self.connect_volume(&volume);

        let imp = self.imp();

        imp.volumes.borrow().insert(0, &volume);
        imp.volume_list_box.set_visible(true);
    }

    fn connect_volume(&self, volume: &model::Volume) {
        volume.connect_remove_request(clone!(@weak self as obj => move |volume| {
            let imp = obj.imp();

            let volumes = imp.volumes.borrow();
            if let Some(pos) = volumes.find(volume) {
                volumes.remove(pos);

                if volumes.n_items() == 0 {
                    imp.volume_list_box.set_visible(false);
                }
            }
        }));
    }

    fn add_env_var(&self) {
        let env_var = model::EnvVar::default();
        self.connect_env_var(&env_var);

        let imp = self.imp();

        imp.env_vars.borrow().insert(0, &env_var);
        imp.env_var_list_box.set_visible(true);
    }

    fn connect_env_var(&self, env_var: &model::EnvVar) {
        env_var.connect_remove_request(clone!(@weak self as obj => move |env_var| {
            let imp = obj.imp();

            let env_vars = imp.env_vars.borrow();
            if let Some(pos) = env_vars.find(env_var) {
                env_vars.remove(pos);

                if env_vars.n_items() == 0 {
                    imp.env_var_list_box.set_visible(false);
                }
            }
        }));
    }

    fn create(&self, run: bool) {
        let imp = self.imp();

        match self.image().or_else(|| {
            imp.image_combo_row
                .selected_item()
                .and_then(|item| item.downcast::<model::Image>().ok())
        }) {
            Some(image) => {
                let opts = api::ContainerCreateOpts::builder()
                    .name(imp.name_entry.text().as_str())
                    .image(image.id())
                    .terminal(imp.terminal_switch.is_active())
                    .resource_limits(api::LinuxResources {
                        block_io: None,
                        cpu: None,
                        devices: None,
                        hugepage_limits: None,
                        memory: Some(api::LinuxMemory {
                            disable_oom_killer: None,
                            kernel: None,
                            kernel_tcp: None,
                            limit: if imp.memory_switch.is_active() {
                                Some(
                                    imp.mem_value.value() as i64
                                        * 1024_i64.pow(
                                            imp.mem_combo_box.active().map(|i| i + 1).unwrap_or(0),
                                        ),
                                )
                            } else {
                                None
                            },
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
                    .portmappings(
                        imp.port_mappings
                            .borrow()
                            .to_owned()
                            .to_typed_list_model::<model::PortMapping>()
                            .into_iter()
                            .map(|port_mapping| api::PortMapping {
                                container_port: Some(port_mapping.container_port() as i64),
                                host_ip: None,
                                host_port: Some(port_mapping.host_port() as i64),
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
                                    let mut options = vec![if volume.writable() {
                                        "rw"
                                    } else {
                                        "ro"
                                    }
                                    .to_owned()];

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

                let cmd = imp.command_entry.text();
                let opts = if cmd.is_empty() {
                    opts
                } else {
                    opts.command([cmd.as_str()])
                }
                .build();

                let pull_latest_tag = if imp.pull_latest_image_switch.is_active() {
                    image.repo_tags().first().cloned()
                } else {
                    None
                };

                utils::do_async(
                    async move {
                        match pull_latest_tag {
                            Some(repo_tag) => PODMAN
                                .images()
                                .pull(
                                    &api::PullOpts::builder()
                                        .reference(repo_tag)
                                        .quiet(true)
                                        .build(),
                                )
                                .and_then(
                                    |_| async move { PODMAN.containers().create(&opts).await },
                                )
                                .await,
                            None => PODMAN.containers().create(&opts).await,
                        }
                    },
                    clone!(@weak self as obj => move |result| {
                        match result {
                            Ok(info) => {
                                obj.imp().created_container_id.set(info.id).unwrap();
                                obj.response(gtk::ResponseType::Close)
                            },
                            Err(e) => {
                                log::error!("Failed to create container: {}", e);
                                obj.show_toast(&gettext!("Failed to create container: {}", e));
                            }
                        }
                    }),
                );
            }
            None => self.show_toast("Failed to create container: no image selected"),
        }
    }

    pub(crate) fn created_container_id(&self) -> Option<&str> {
        self.imp().created_container_id.get().map(String::as_str)
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
