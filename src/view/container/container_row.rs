use std::error;

use adw::subclass::prelude::{ActionRowImpl, PreferencesRowImpl};
use cascade::cascade;
use gettextrs::gettext;
use gtk::glib::{clone, closure, WeakRef};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib, CompositeTemplate};
use once_cell::sync::Lazy;

use crate::window::Window;
use crate::{model, utils, view};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/container-row.ui")]
    pub(crate) struct ContainerRow {
        pub(super) container: WeakRef<model::Container>,
        #[template_child]
        pub(super) stats_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) cpu_bar: TemplateChild<view::CircularProgressBar>,
        #[template_child]
        pub(super) mem_bar: TemplateChild<view::CircularProgressBar>,
        #[template_child]
        pub(super) status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) menu_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerRow {
        const NAME: &'static str = "ContainerRow";
        type Type = super::ContainerRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action("container.show-details", None, move |widget, _, _| {
                widget.show_details();
            });

            klass.install_action("container.start", None, move |widget, _, _| {
                widget.start();
            });
            klass.install_action("container.stop", None, move |widget, _, _| {
                widget.stop();
            });
            klass.install_action("container.force-stop", None, move |widget, _, _| {
                widget.force_stop();
            });
            klass.install_action("container.restart", None, move |widget, _, _| {
                widget.restart();
            });
            klass.install_action("container.force-restart", None, move |widget, _, _| {
                widget.force_restart();
            });
            klass.install_action("container.pause", None, move |widget, _, _| {
                widget.pause();
            });
            klass.install_action("container.resume", None, move |widget, _, _| {
                widget.resume();
            });

            klass.install_action("container.rename", None, move |widget, _, _| {
                widget.rename();
            });

            klass.install_action("container.commit", None, move |widget, _, _| {
                widget.commit();
            });

            klass.install_action("container.delete", None, move |widget, _, _| {
                widget.delete();
            });
            klass.install_action("container.force-delete", None, move |widget, _, _| {
                widget.force_delete();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerRow {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "container",
                    "container",
                    "The Container of this ContainerRow",
                    model::Container::static_type(),
                    glib::ParamFlags::READWRITE,
                )]
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
                "container" => {
                    self.container.set(value.get().unwrap());
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "container" => obj.container().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let container_expr = Self::Type::this_expression("container");
            let stats_expr = container_expr.chain_property::<model::Container>("stats");
            let status_expr = container_expr.chain_property::<model::Container>("status");

            container_expr
                .chain_property::<model::Container>("name")
                .chain_closure::<String>(closure!(|_: glib::Object, name: Option<String>| {
                    utils::escape(&utils::format_option(name))
                }))
                .bind(obj, "title", Some(obj));

            container_expr
                .chain_property::<model::Container>("image-name")
                .chain_closure::<String>(closure!(|_: glib::Object, name: Option<String>| {
                    utils::escape(&utils::format_option(name))
                }))
                .bind(obj, "subtitle", Some(obj));

            status_expr
                .chain_closure::<bool>(closure!(
                    |_: glib::Object, status: model::ContainerStatus| matches!(
                        status,
                        model::ContainerStatus::Running
                    )
                ))
                .bind(&*self.stats_box, "visible", Some(obj));

            stats_expr
                .chain_closure::<f64>(closure!(
                    |_: glib::Object, stats: Option<model::BoxedContainerStats>| {
                        stats
                            .and_then(|stats| stats.CPU.map(|perc| perc as f64 * 0.01))
                            .unwrap_or_default()
                    }
                ))
                .bind(&*self.cpu_bar, "percentage", Some(obj));

            stats_expr
                .chain_closure::<f64>(closure!(
                    |_: glib::Object, stats: Option<model::BoxedContainerStats>| {
                        stats
                            .and_then(|stats| stats.mem_perc.map(|perc| perc as f64 * 0.01))
                            .unwrap_or_default()
                    }
                ))
                .bind(&*self.mem_bar, "percentage", Some(obj));

            status_expr
                .chain_closure::<String>(closure!(
                    |_: glib::Object, status: model::ContainerStatus| status.to_string()
                ))
                .bind(&*self.status_label, "label", Some(obj));

            let css_classes = self.status_label.css_classes();
            status_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: glib::Object, status: model::ContainerStatus| {
                        css_classes
                            .iter()
                            .cloned()
                            .chain(Some(glib::GString::from(view::container_status_css_class(
                                status,
                            ))))
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(&*self.status_label, "css-classes", Some(obj));

            container_expr
                .chain_property::<model::Container>("action-ongoing")
                .chain_closure::<String>(closure!(|_: glib::Object, action_ongoing: bool| {
                    if action_ongoing {
                        "ongoing"
                    } else {
                        "menu"
                    }
                }))
                .bind(&*self.menu_stack, "visible-child-name", Some(obj));

            status_expr
                .chain_closure::<Option<gio::MenuModel>>(closure!(
                    |_: Self::Type, status: model::ContainerStatus| {
                        use model::ContainerStatus::*;

                        Some(match status {
                            Running => running_menu(),
                            Paused => paused_menu(),
                            Configured | Created | Exited | Dead | Stopped => stopped_menu(),
                            _ => return None,
                        })
                    }
                ))
                .bind(&*self.menu_button, "menu-model", Some(obj));
        }
    }

    impl WidgetImpl for ContainerRow {}
    impl ListBoxRowImpl for ContainerRow {}
    impl PreferencesRowImpl for ContainerRow {}
    impl ActionRowImpl for ContainerRow {}
}

glib::wrapper! {
    pub(crate) struct ContainerRow(ObjectSubclass<imp::ContainerRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow;
}

impl From<&model::Container> for ContainerRow {
    fn from(container: &model::Container) -> Self {
        glib::Object::new(&[("container", container)]).expect("Failed to create ContainerRow")
    }
}

impl ContainerRow {
    pub(crate) fn container(&self) -> Option<model::Container> {
        self.imp().container.upgrade()
    }
}

impl ContainerRow {
    fn show_toast(&self, title: &str, e: impl error::Error) {
        self.root()
            .unwrap()
            .downcast::<Window>()
            .unwrap()
            .show_toast(
                &adw::Toast::builder()
                    .title(&format!("{}: {}", title, e))
                    .timeout(3)
                    .priority(adw::ToastPriority::High)
                    .build(),
            );
    }

    pub(crate) fn start(&self) {
        self.container().unwrap().start(
            clone!(@weak self as obj => move |result| if let Err(e) = result {
                obj.show_toast(&gettext("Error on starting container"), e);
            }),
        );
    }

    pub(crate) fn stop(&self) {
        self.container().unwrap().stop(
            clone!(@weak self as obj => move |result| if let Err(e) = result {
                obj.show_toast(&gettext("Error on stopping container"), e);
            }),
        );
    }

    pub(crate) fn force_stop(&self) {
        self.container().unwrap().force_stop(
            clone!(@weak self as obj => move |result| if let Err(e) = result {
                obj.show_toast(&gettext("Error on force stopping container"), e);
            }),
        );
    }

    pub(crate) fn restart(&self) {
        self.container().unwrap().restart(
            clone!(@weak self as obj => move |result| if let Err(e) = result {
                obj.show_toast(&gettext("Error on restarting container"), e);
            }),
        );
    }

    pub(crate) fn force_restart(&self) {
        self.container().unwrap().force_restart(
            clone!(@weak self as obj => move |result| if let Err(e) = result {
                obj.show_toast(&gettext("Error on force restarting container"), e);
            }),
        );
    }

    pub(crate) fn pause(&self) {
        self.container().unwrap().pause(
            clone!(@weak self as obj => move |result| if let Err(e) = result {
                obj.show_toast(&gettext("Error on pausing container"), e);
            }),
        );
    }

    pub(crate) fn resume(&self) {
        self.container().unwrap().resume(
            clone!(@weak self as obj => move |result| if let Err(e) = result {
                obj.show_toast(&gettext("Error on resuming container"), e);
            }),
        );
    }

    pub(crate) fn rename(&self) {
        let dialog = view::ContainerRenameDialog::from(self.container());
        dialog.set_transient_for(Some(
            &self.root().unwrap().downcast::<gtk::Window>().unwrap(),
        ));
        dialog.run_async(clone!(@weak self as obj => move |dialog, response| {
            obj.on_rename_dialog_response(dialog.upcast_ref(), response, |obj, dialog| {
                dialog.connect_response(clone!(@weak obj => move |dialog, response| {
                    obj.on_rename_dialog_response(dialog, response, |_, _| {});
                }));
            });
        }));
    }

    fn on_rename_dialog_response<F>(&self, dialog: &gtk::Dialog, response: gtk::ResponseType, op: F)
    where
        F: Fn(&Self, &gtk::Dialog),
    {
        match response {
            gtk::ResponseType::Cancel => dialog.close(),
            gtk::ResponseType::Apply => {
                dialog.close();
                glib::timeout_add_seconds_local_once(
                    1,
                    clone!(@weak self as obj => move || {
                        let panel = obj
                            .ancestor(view::ContainersPanel::static_type())
                            .unwrap()
                            .downcast::<view::ContainersPanel>()
                            .unwrap();

                        panel.update_search_filter();
                        panel.update_sorter();
                    }),
                );
            }
            _ => op(self, dialog),
        }
    }

    pub(crate) fn commit(&self) {
        self.container().unwrap().commit(
            clone!(@weak self as obj => move |result| if let Err(e) = result {
                obj.show_toast(&gettext("Error on committing container"), e);
            }),
        );
    }

    pub(crate) fn delete(&self) {
        self.container().unwrap().delete(
            clone!(@weak self as obj => move |result| if let Err(e) = result {
                obj.show_toast(&gettext("Error on deleting container"), e);
            }),
        );
    }

    pub(crate) fn force_delete(&self) {
        self.container().unwrap().force_delete(
            clone!(@weak self as obj => move |result| if let Err(e) = result {
                obj.show_toast(&gettext("Error on force deleting container"), e);
            }),
        );
    }

    fn show_details(&self) {
        utils::find_leaflet_overlay(self)
            .show_details(&view::ContainerPage::from(&self.container().unwrap()));
    }
}

fn base_menu() -> gio::Menu {
    cascade! {
        gio::Menu::new();
        ..append_section(None, &cascade!{
            gio::Menu::new();
            ..append(Some(&gettext("Re_name…")), Some("container.rename"));
        });
        ..append_section(None, &cascade!{
            gio::Menu::new();
            ..append(Some(&gettext("_Commit")), Some("container.commit"));
        });
        ..append_section(None, &cascade!{
            gio::Menu::new();
            ..append(Some(&gettext("_Delete")), Some("container.delete"));
        });
    }
}

fn stopped_menu() -> gio::Menu {
    cascade! {
        base_menu();
        ..prepend_section(None, &cascade! {
            gio::Menu::new();
            ..append(Some(&gettext("_Start")), Some("container.start"));
        });
    }
}

fn not_stopped_menu() -> gio::Menu {
    cascade! {
        gio::Menu::new();
        ..append(Some(&gettext("S_top")), Some("container.stop"));
        ..append(Some(&gettext("_Force Stop")), Some("container.force-stop"));
        ..append(Some(&gettext("R_estart")), Some("container.restart"));
        ..append(Some(&gettext("F_orce Restart")), Some("container.force-restart"));
    }
}

fn running_menu() -> gio::Menu {
    cascade! {
        base_menu();
        ..prepend_section(None, &cascade! {
            not_stopped_menu();
            ..append(Some(&gettext("_Pause")), Some("container.pause"));
        });
    }
}

fn paused_menu() -> gio::Menu {
    cascade! {
        base_menu();
        ..prepend_section(None, &cascade! {
            not_stopped_menu();
            ..append(Some(&gettext("_Resume")), Some("container.resume"));
        });
    }
}
