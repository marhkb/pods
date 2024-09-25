use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gettextrs::ngettext;
use glib::clone;
use glib::Properties;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;
use crate::widget;

const ACTION_REFRESH: &str = "info-panel.refresh";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::InfoPanel)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/info_panel.ui")]
    pub(crate) struct InfoPanel {
        pub(super) store_graph_options_rows: RefCell<Vec<widget::PropertyRow>>,
        pub(super) store_graph_status_rows: RefCell<Vec<widget::PropertyRow>>,
        #[property(get, set, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) version_api_version_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) version_built_time_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) version_git_commit_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) version_go_version_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) version_os_arch_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) version_version_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) store_config_file_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) store_container_store_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) store_container_store_paused_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) store_container_store_running_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) store_container_store_stopped_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) store_graph_driver_name_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) store_graph_options_row: TemplateChild<adw::ExpanderRow>,
        #[template_child]
        pub(super) store_graph_options_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) store_graph_root_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) store_graph_status_row: TemplateChild<adw::ExpanderRow>,
        #[template_child]
        pub(super) store_graph_status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) store_image_store_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) store_run_root_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) store_volume_path_row: TemplateChild<widget::PropertyRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for InfoPanel {
        const NAME: &'static str = "PdsInfoPanel";
        type Type = super::InfoPanel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action(ACTION_REFRESH, None, |widget, _, _| {
                widget.refresh();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for InfoPanel {
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
            self.obj().action_set_enabled(ACTION_REFRESH, false);
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for InfoPanel {}

    #[gtk::template_callbacks]
    impl InfoPanel {
        #[template_callback]
        fn on_notify_client(&self) {
            self.obj().refresh();
        }
    }
}

glib::wrapper! {
    pub(crate) struct InfoPanel(ObjectSubclass<imp::InfoPanel>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl InfoPanel {
    pub(crate) fn refresh(&self) {
        if let Some(client) = self.client() {
            let imp = self.imp();

            imp.stack.set_visible_child_name("spinner");
            self.action_set_enabled(ACTION_REFRESH, false);

            utils::do_async(
                {
                    let podman = client.podman();
                    async move { podman.info().await }
                },
                clone!(@weak self as obj => move |result| {
                    let imp = obj.imp();

                    obj.action_set_enabled(ACTION_REFRESH, true);

                    match result {
                        Ok(info) => {
                            imp.stack.set_visible_child_name("content");

                            // Version
                            let version = info.version.as_ref();
                            imp.version_api_version_row.set_value(&utils::format_option(
                                version.and_then(|version| version.api_version.as_ref()),
                            ));
                            imp.version_built_time_row
                                .set_value(&utils::format_option(version.and_then(|v| v.built.and_then(|t|
                                    glib::DateTime::from_unix_local(t).ok().map(|d| {
                                        d.format(
                                            // Translators: This is a date time format (https://valadoc.org/glib-2.0/GLib.DateTime.format.html)
                                            &gettext("%x %X"),
                                        )
                                        .unwrap()
                                    })
                                ))));
                            imp.version_git_commit_row
                                .set_value(&utils::format_option(version.and_then(|v| v.git_commit.as_ref().and_then(|s| {
                                    if s.is_empty() {
                                        None
                                    } else {
                                        Some(s)
                                    }
                                }))));
                            imp.version_go_version_row.set_value(&utils::format_option(
                                version.and_then(|v| v.go_version.as_ref())
                            ));
                            imp.version_os_arch_row.set_value(&utils::format_option(
                                version.and_then(|v| v.os_arch.as_ref())
                            ));
                            imp.version_version_row.set_value(&utils::format_option(
                                version.and_then(|v| v.version.as_ref())
                            ));

                            // Store
                            let store = info.store.as_ref();
                            let container_store = store.and_then(|s| s.container_store.as_ref());
                            imp.store_config_file_row.set_value(&utils::format_option(
                                store.and_then(|v| v.config_file.as_ref()),
                            ));
                            imp.store_container_store_label
                                .set_label(&utils::format_option(
                                    container_store.and_then(|c| c.number.map(|n| {
                                        // Translators: "{}" is a placeholder for a cardinal numbers.
                                        ngettext!("{} Container", "{} Containers", n as u32, n)
                                    })
                                )));
                            imp.store_container_store_paused_row
                                .set_value(&utils::format_option(container_store.and_then(|s| s
                                    .paused
                                    .as_ref()
                                    .map(i64::to_string)
                                )));
                            imp.store_container_store_running_row
                                .set_value(&utils::format_option(container_store.and_then(|s| s
                                    .running
                                    .as_ref()
                                    .map(i64::to_string)
                                )));
                            imp.store_container_store_stopped_row
                                .set_value(&utils::format_option(container_store.and_then(|s| s
                                    .stopped
                                    .as_ref()
                                    .map(i64::to_string)
                                )));
                            imp.store_graph_driver_name_row
                                .set_value(&utils::format_option(
                                    store.and_then(|s| s.graph_driver_name.as_ref())
                                ));

                            let graph_options = store.and_then(|s| s.graph_options.as_ref());
                            imp.store_graph_options_label
                                .set_label(&utils::format_option(
                                    graph_options.as_ref().map(|o| {
                                        let len = o.as_object().unwrap().len();
                                        // Translators: "{}" is a placeholder for a cardinal number.
                                        ngettext!("{} Option", "{} Options", len as u32, len)
                                    })
                                ));

                            let mut store_graph_options_rows =
                                imp.store_graph_options_rows.borrow_mut();
                            while let Some(row) = store_graph_options_rows.pop() {
                                imp.store_graph_options_row.remove(&row);
                            }

                            if let Some(graph_options) = store.and_then(|s| s.graph_options.as_ref()) {
                                graph_options.as_object().unwrap().iter().for_each(|(k, v)| {
                                    let row = widget::PropertyRow::default();
                                    row.set_key(k);
                                    row.set_value(&v.to_string());

                                    imp.store_graph_options_row.add_row(&row);
                                    store_graph_options_rows.push(row);
                                });
                            }

                            imp.store_graph_root_row.set_value(&utils::format_option(
                                store.and_then(|s| s.graph_root.as_ref())
                            ));
                            imp.store_graph_status_label
                                .set_label(&utils::format_option(
                                    store.and_then(|s| s.graph_status.as_ref().map(|s| {
                                        // Translators: "{}" is placeholders for a cardinal number.
                                        ngettext!("{} State", "{} States", s.len() as u32, s.len())
                                    }))
                                ));

                            let mut store_graph_status_rows = imp.store_graph_status_rows.borrow_mut();
                            while let Some(row) = store_graph_status_rows.pop() {
                                imp.store_graph_status_row.remove(&row);
                            }

                            if let Some(graph_status) = store.and_then(|s| s.graph_status.as_ref())
                            {
                                graph_status.iter().for_each(|(k, v)| {
                                    let row = widget::PropertyRow::default();
                                    row.set_key(k);
                                    row.set_value(v);

                                    imp.store_graph_status_row.add_row(&row);
                                    store_graph_status_rows.push(row);
                                });
                            }
                            imp.store_image_store_row
                                .set_value(&utils::format_option(
                                    store.and_then(|s| s.image_store.as_ref()).and_then(|s| s.number.map(|n| {
                                        // Translators: "{}" is placeholders for a cardinal number.
                                        ngettext!("{} Image", "{} Images", n as u32, n)
                                    })
                                )));
                            imp.store_run_root_row.set_value(&utils::format_option(
                                store.and_then(|s| s.run_root.as_ref())
                            ));
                            imp.store_volume_path_row.set_value(&utils::format_option(
                                store.and_then(|s| s.volume_path.as_ref())
                            ));
                        }
                        Err(e) => {
                            imp.stack.set_visible_child_name("error");

                            log::error!("Failed to retrieve host info: {e}");
                            utils::show_error_toast(obj.upcast_ref(), &gettext("Error on retrieving info"), &e.to_string());
                        }
                    }
                }),
            );
        }
    }
}
