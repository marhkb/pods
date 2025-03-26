use std::cell::OnceCell;
use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::Properties;
use glib::clone;
use glib::closure;
use gtk::CompositeTemplate;
use gtk::gio;
use gtk::glib;

use crate::model;
use crate::podman;
use crate::utils;
use crate::view;
use crate::widget;

const ACTION_ADD_CMD_ARG: &str = "container-creation-page.add-cmd-arg";
const ACTION_SELECT_POD: &str = "container-creation-page.select-pod";
const ACTION_CLEAR_POD: &str = "container-creation-page.clear-pod";
const ACTION_ADD_PORT_MAPPING: &str = "container-creation-page.add-port-mapping";
const ACTION_ADD_VOLUME: &str = "container-creation-page.add-volume";
const ACTION_ADD_ENV_VAR: &str = "container-creation-page.add-env-var";
const ACTION_ADD_LABEL: &str = "container-creation-page.add-label";
const ACTION_CREATE_AND_RUN: &str = "container-creation-page.create-and-run";
const ACTION_CREATE: &str = "container-creation-page.create";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainerCreationPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/container_creation_page.ui")]
    pub(crate) struct ContainerCreationPage {
        #[property(get, set, construct_only)]
        pub(super) model: OnceCell<model::ContainerCreation>,

        pub(super) command_row_handler:
            RefCell<Option<(glib::SignalHandlerId, glib::WeakRef<model::Image>)>>,
        #[template_child]
        pub(super) navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub(super) create_button: TemplateChild<adw::SplitButton>,
        #[template_child]
        pub(super) name_entry_row: TemplateChild<widget::RandomNameEntryRow>,
        #[template_child]
        pub(super) image_selection_combo_row: TemplateChild<view::ImageSelectionComboRow>,
        #[template_child]
        pub(super) pod_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) pull_latest_image_switch_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) command_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) command_arg_list_box: TemplateChild<gtk::ListBox>,
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
    impl ObjectSubclass for ContainerCreationPage {
        const NAME: &'static str = "PdsContainerCreationPage";
        type Type = super::ContainerCreationPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action(ACTION_ADD_CMD_ARG, None, |widget, _, _| {
                add_value(&widget.model().cmd_args());
            });
            klass.install_action(ACTION_SELECT_POD, None, |widget, _, _| {
                widget.select_pod();
            });
            klass.install_action(ACTION_CLEAR_POD, None, |widget, _, _| {
                widget.model().set_pod(Option::<model::Pod>::None);
            });
            klass.install_action(ACTION_ADD_PORT_MAPPING, None, |widget, _, _| {
                add_key_val(&widget.model().port_mappings());
            });
            klass.install_action(ACTION_ADD_VOLUME, None, |widget, _, _| {
                add_key_val(&widget.model().volumes());
            });
            klass.install_action(ACTION_ADD_ENV_VAR, None, |widget, _, _| {
                add_key_val(&widget.model().env_vars());
            });
            klass.install_action(ACTION_ADD_LABEL, None, |widget, _, _| {
                add_key_val(&widget.model().labels());
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

    impl ObjectImpl for ContainerCreationPage {
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

            self.image_selection_combo_row
                .set_client(obj.model().client().as_ref());

            let pod_expr = Self::Type::this_expression("model")
                .chain_property::<model::ContainerCreation>("pod");

            pod_expr
                .chain_closure::<String>(closure!(|_: Self::Type, pod: Option<&model::Pod>| pod
                    .map(model::Pod::name)
                    .unwrap_or_default()))
                .bind(&self.pod_row.get(), "subtitle", Some(obj));

            pod_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, pod: Option<&model::Pod>| {
                    pod.is_none()
                }))
                .bind(
                    &self.port_mapping_preferences_group.get(),
                    "visible",
                    Some(obj),
                );

            if let Some(image) = obj.model().image() {
                self.image_selection_combo_row.set_image(Some(image));
                obj.update_data();
            } else if let Some(volume) = obj.model().volume() {
                let mount = add_mount(
                    &obj.model().volumes(),
                    obj.model().client().as_ref().unwrap(),
                );
                mount.set_mount_type(model::MountType::Volume);
                mount.set_volume(Some(volume));
            }

            bind_model(
                &self.command_arg_list_box,
                &obj.model().cmd_args(),
                |item| {
                    view::ValueRow::new(item.downcast_ref().unwrap(), &gettext("Argument")).upcast()
                },
                ACTION_ADD_CMD_ARG,
                &gettext("Add Argument"),
            );

            bind_model(
                &self.port_mapping_list_box,
                &obj.model().port_mappings(),
                |item| {
                    view::PortMappingRow::from(item.downcast_ref::<model::PortMapping>().unwrap())
                        .upcast()
                },
                ACTION_ADD_PORT_MAPPING,
                &gettext("Add Port Mapping"),
            );

            bind_model(
                &self.volume_list_box,
                &obj.model().volumes(),
                |item| view::MountRow::from(item.downcast_ref::<model::Mount>().unwrap()).upcast(),
                ACTION_ADD_VOLUME,
                &gettext("Add Volume"),
            );

            bind_model(
                &self.env_var_list_box,
                &obj.model().env_vars(),
                |item| {
                    view::KeyValRow::from(item.downcast_ref::<model::KeyVal>().unwrap()).upcast()
                },
                ACTION_ADD_ENV_VAR,
                &gettext("Add Environment Variable"),
            );

            bind_model(
                &self.labels_list_box,
                &obj.model().labels(),
                |item| {
                    view::KeyValRow::from(item.downcast_ref::<model::KeyVal>().unwrap()).upcast()
                },
                ACTION_ADD_LABEL,
                &gettext("Add Label"),
            );
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ContainerCreationPage {
        fn root(&self) {
            self.parent_root();

            let widget = &*self.obj();

            glib::idle_add_local(clone!(
                #[weak]
                widget,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    widget.imp().name_entry_row.grab_focus();
                    glib::ControlFlow::Break
                }
            ));
            utils::root(widget).set_default_widget(Some(&*self.create_button));
        }

        fn unroot(&self) {
            utils::root(&*self.obj()).set_default_widget(gtk::Widget::NONE);
            self.parent_unroot()
        }
    }

    #[gtk::template_callbacks]
    impl ContainerCreationPage {
        #[template_callback]
        fn on_name_entry_row_notify_text(&self) {
            let enabled = self.name_entry_row.text().len() > 0;

            let obj = &*self.obj();
            obj.action_set_enabled(ACTION_CREATE_AND_RUN, enabled);
            obj.action_set_enabled(ACTION_CREATE, enabled);
        }

        #[template_callback]
        fn on_image_selection_combo_row_notify_subtitle(&self) {
            self.obj().update_data();
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainerCreationPage(ObjectSubclass<imp::ContainerCreationPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Client> for ContainerCreationPage {
    fn from(client: &model::Client) -> Self {
        Self::new(&model::ContainerCreation::from(client))
    }
}

impl From<&model::Image> for ContainerCreationPage {
    fn from(image: &model::Image) -> Self {
        Self::new(&model::ContainerCreation::from(image))
    }
}

impl From<&model::Pod> for ContainerCreationPage {
    fn from(pod: &model::Pod) -> Self {
        Self::new(&model::ContainerCreation::from(pod))
    }
}

impl From<&model::Volume> for ContainerCreationPage {
    fn from(volume: &model::Volume) -> Self {
        Self::new(&model::ContainerCreation::from(volume))
    }
}

impl ContainerCreationPage {
    fn new(model: &model::ContainerCreation) -> Self {
        let obj: Self = glib::Object::builder().property("model", model).build();

        let imp = obj.imp();

        imp.name_entry_row
            .bind_property("text", model, "name")
            .sync_create()
            .bidirectional()
            .build();

        obj
    }

    pub(crate) fn restore(model: &model::ContainerCreation) -> Self {
        let obj: Self = glib::Object::builder().property("model", model).build();

        let imp = obj.imp();

        model
            .bind_property("name", &imp.name_entry_row.get(), "text")
            .sync_create()
            .bidirectional()
            .build();

        obj
    }

    fn update_local_data(&self, config: &model::ImageConfig) {
        let imp = self.imp();

        imp.command_entry_row
            .set_text(&config.cmd().unwrap_or_default());

        self.model().port_mappings().remove_all();

        let exposed_ports = config.exposed_ports();
        for i in 0..exposed_ports.n_items() {
            let exposed = exposed_ports.string(i).unwrap();

            let port_mapping = add_port_mapping(&self.model().port_mappings());
            imp.port_mapping_list_box.set_visible(true);

            let mut split = exposed.split_terminator('/');
            if let Some(port) = split.next() {
                match port.parse::<i32>() {
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
        }
    }

    fn update_data(&self) {
        let imp = self.imp();

        match imp.image_selection_combo_row.image() {
            Some(image) => match image.data() {
                Some(data) => self.update_local_data(&data.config()),
                None => {
                    if let Some((handler, image)) = imp.command_row_handler.take() {
                        if let Some(image) = image.upgrade() {
                            image.disconnect(handler);
                        }
                    }
                    let handler = image.connect_data_notify(clone!(
                        #[weak(rename_to = obj)]
                        self,
                        move |image| {
                            obj.update_local_data(&image.data().unwrap().config());
                        }
                    ));
                    let image_weak = glib::WeakRef::new();
                    image_weak.set(Some(&image));
                    imp.command_row_handler.replace(Some((handler, image_weak)));

                    image.inspect(|_| {});
                }
            },
            None => {
                imp.command_entry_row.set_text("");
                self.model().port_mappings().remove_all();
            }
        }
    }

    pub(crate) fn select_pod(&self) {
        let client = if let Some(client) = self.model().client() {
            client
        } else {
            return;
        };

        // TODO: new Dialog????
        let pod_selection_page = view::PodSelectionPage::from(&client.pod_list());
        pod_selection_page.connect_pod_selected(clone!(
            #[weak(rename_to = obj)]
            self,
            move |_, pod| {
                obj.model().set_pod(Some(&pod));
            }
        ));
        self.imp().navigation_view.push(
            &adw::NavigationPage::builder()
                .child(&pod_selection_page)
                .build(),
        );
    }

    fn finish(&self, run: bool) {
        let imp = self.imp();

        match imp.image_selection_combo_row.mode() {
            view::ImageSelectionMode::Local => {
                let image = imp.image_selection_combo_row.subtitle().unwrap();
                if imp.pull_latest_image_switch_row.is_active() {
                    self.pull_and_create(image.as_str(), false, run);
                } else {
                    // let page = view::ActionPage::from(
                    //     // &self.client().unwrap().action_list().create_container(
                    //     //     imp.name_entry_row.text().as_str(),
                    //     //     self.create().image(image.as_str()).build(),
                    //     //     run,
                    //     // ),
                    // );

                    // imp.navigation_view.push(
                    //     &adw::NavigationPage::builder()
                    //         .can_pop(false)
                    //         .child(&page)
                    //         .build(),
                    // );
                }
            }
            view::ImageSelectionMode::Remote => {
                self.pull_and_create(
                    imp.image_selection_combo_row.subtitle().unwrap().as_str(),
                    true,
                    run,
                );
            }
            view::ImageSelectionMode::Unset => {
                log::error!("Error while starting container: no image selected");
                utils::show_error_toast(
                    self,
                    &gettext("Failed to create container"),
                    &gettext("no image selected"),
                )
            }
        }
    }

    fn pull_and_create(&self, reference: &str, remote: bool, run: bool) {
        let imp = self.imp();

        let pull_opts = podman::opts::PullOpts::builder()
            .reference(reference)
            .policy(if remote {
                podman::opts::PullPolicy::Always
            } else {
                podman::opts::PullPolicy::Newer
            })
            .build();

        // let page = view::ActionPage::from(
        //     &self
        //         .client()
        //         .unwrap()
        //         .action_list()
        //         .create_container_download_image(
        //             imp.name_entry_row.text().as_str(),
        //             pull_opts,
        //             self.create(),
        //             run,
        //         ),
        // );

        // imp.navigation_view
        //     .push(&adw::NavigationPage::builder().child(&page).build());
    }

    fn create(&self) -> podman::opts::ContainerCreateOptsBuilder {
        todo!()
        //     let imp = self.imp();

        //     let create_opts = podman::opts::ContainerCreateOpts::builder()
        //         .name(imp.name_entry_row.text().as_str())
        //         .pod(self.pod().as_ref().map(model::Pod::name))
        //         .terminal(imp.terminal_switch_row.is_active())
        //         .privileged(imp.privileged_switch_row.is_active())
        //         .portmappings(if self.pod().is_none() {
        //             Box::new(
        //                 imp.port_mappings()
        //                     .iter::<model::PortMapping>()
        //                     .map(Result::unwrap)
        //                     .map(|port_mapping| podman::models::PortMapping {
        //                         container_port: Some(port_mapping.container_port() as u16),
        //                         host_ip: None,
        //                         host_port: Some(port_mapping.host_port() as u16),
        //                         protocol: Some(port_mapping.protocol().to_string()),
        //                         range: None,
        //                     }),
        //             ) as Box<dyn Iterator<Item = _>>
        //         } else {
        //             Box::new(std::iter::empty()) as Box<dyn Iterator<Item = _>>
        //         })
        //         .mounts(
        //             imp.volumes()
        //                 .iter::<model::Mount>()
        //                 .map(Result::unwrap)
        //                 .filter(|mount| mount.mount_type() == model::MountType::Bind)
        //                 .map(|mount| podman::models::ContainerMount {
        //                     destination: Some(mount.container_path()),
        //                     source: Some(mount.host_path()),
        //                     _type: Some("bind".to_owned()),
        //                     options: mount_options(&mount),
        //                     uid_mappings: None,
        //                     gid_mappings: None,
        //                 }),
        //         )
        //         .volumes(
        //             imp.volumes()
        //                 .iter::<model::Mount>()
        //                 .map(Result::unwrap)
        //                 .filter(|mount| mount.mount_type() == model::MountType::Volume)
        //                 .map(|mount| podman::models::NamedVolume {
        //                     dest: Some(mount.container_path()),
        //                     is_anonymous: None,
        //                     name: mount.volume().map(|volume| volume.inner().name.clone()),
        //                     options: mount_options(&mount),
        //                 }),
        //         )
        //         .env(
        //             imp.env_vars()
        //                 .iter::<model::KeyVal>()
        //                 .map(Result::unwrap)
        //                 .map(|entry| (entry.key(), entry.value())),
        //         )
        //         .labels(
        //             imp.labels()
        //                 .iter::<model::KeyVal>()
        //                 .map(Result::unwrap)
        //                 .map(|entry| (entry.key(), entry.value())),
        //         );

        //     let create_opts = if imp.memory_switch.is_active() {
        //         create_opts.resource_limits(podman::models::LinuxResources {
        //             block_io: None,
        //             cpu: None,
        //             devices: None,
        //             hugepage_limits: None,
        //             memory: Some(podman::models::LinuxMemory {
        //                 disable_oom_killer: None,
        //                 kernel: None,
        //                 kernel_tcp: None,
        //                 limit: Some(
        //                     imp.mem_value.value() as i64
        //                         * 1000_i64.pow(imp.mem_drop_down.selected() + 1),
        //                 ),
        //                 reservation: None,
        //                 swap: None,
        //                 swappiness: None,
        //                 use_hierarchy: None,
        //             }),
        //             network: None,
        //             pids: None,
        //             rdma: None,
        //             unified: None,
        //         })
        //     } else {
        //         create_opts
        //     };

        //     let cmd = imp.command_entry_row.text();
        //     let create_opts = if cmd.is_empty() {
        //         create_opts
        //     } else {
        //         let args = imp
        //             .cmd_args()
        //             .iter::<model::Value>()
        //             .map(Result::unwrap)
        //             .map(|value| value.value());
        //         let mut cmd = vec![cmd.to_string()];
        //         cmd.extend(args);
        //         create_opts.command(&cmd)
        //     };

        //     let healthcheck_cmd = imp.health_check_command_entry_row.text();

        //     if healthcheck_cmd.is_empty() {
        //         create_opts
        //     } else {
        //         create_opts.health_config(podman::models::Schema2HealthConfig {
        //             interval: Some(imp.health_check_interval_value.value() as i64 * 1_000_000_000),
        //             retries: Some(imp.health_check_retries_value.value() as i64),
        //             start_period: Some(
        //                 imp.health_check_start_period_value.value() as i64 * 1_000_000_000,
        //             ),
        //             test: Some(
        //                 healthcheck_cmd
        //                     .split(' ')
        //                     .map(str::to_string)
        //                     .collect::<Vec<_>>(),
        //             ),
        //             timeout: Some(imp.health_check_timeout_value.value() as i64 * 1_000_000_000),
        //         })
        //     }
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

fn add_port_mapping(model: &gio::ListStore) -> model::PortMapping {
    let port_mapping = model::PortMapping::default();

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

fn add_mount(model: &gio::ListStore, client: &model::Client) -> model::Mount {
    let mount = model::Mount::from(client);

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

fn add_value(model: &gio::ListStore) {
    let value = model::Value::default();

    value.connect_remove_request(clone!(
        #[weak]
        model,
        move |value| {
            if let Some(pos) = model.find(value) {
                model.remove(pos);
            }
        }
    ));

    model.append(&value);
}

fn add_key_val(model: &gio::ListStore) {
    let entry = model::KeyVal::default();

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

fn mount_options(mount: &model::Mount) -> Option<Vec<String>> {
    Some({
        let mut options = vec![if mount.writable() { "rw" } else { "ro" }.to_owned()];

        let selinux = mount.selinux().to_string();
        if !selinux.is_empty() {
            options.push(selinux)
        }

        options
    })
}
