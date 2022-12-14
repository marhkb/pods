use std::cell::RefCell;

use adw::subclass::prelude::*;
use adw::traits::MessageDialogExt;
use gettextrs::gettext;
use gtk::gdk;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::model;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/container/rename-dialog.ui")]
    pub(crate) struct RenameDialog {
        pub(super) container: glib::WeakRef<model::Container>,
        pub(super) response: RefCell<Option<String>>,
        pub(super) rename_finished: OnceCell<()>,
        #[template_child]
        pub(super) entry_row: TemplateChild<view::RandomNameEntryRow>,
        // #[template_child]
        // pub(super) error_label_row: TemplateChild<adw::PreferencesRow>,
        #[template_child]
        pub(super) error_label_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub(super) error_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RenameDialog {
        const NAME: &'static str = "PdsContainerRenameDialog";
        type Type = super::RenameDialog;
        type ParentType = adw::MessageDialog;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RenameDialog {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::Container>("container")
                        .construct_only()
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "container" => self.container.set(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "container" => self.obj().container().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            if let Some(container) = obj.container() {
                container.connect_deleted(clone!(@weak obj => move |_| {
                    obj.imp().rename_finished.set(()).unwrap();
                    obj.close();
                }));

                self.entry_row.set_text(&container.name());
                self.entry_row.grab_focus();
            }

            obj.connect_response(None, |obj, response| {
                obj.imp().response.replace(Some(response.to_owned()));
            });

            obj.connect_close_request(|obj| {
                let imp = obj.imp();

                if imp.rename_finished.get().is_some() {
                    return gtk::Inhibit(false);
                }

                match imp.response.take() {
                    Some(response) => {
                        if &response == "close" {
                            return gtk::Inhibit(false);
                        }

                        if let Some(container) = obj.container() {
                            let new_name = imp.entry_row.text().to_string();
                            container.rename(
                                new_name,
                                clone!(@weak obj => move |result| {
                                    let imp = obj.imp();
                                    match result {
                                        Ok(_) => {
                                            imp.rename_finished.set(()).unwrap();
                                            obj.close();
                                        },
                                        Err(e) => {
                                            imp.entry_row.add_css_class("error");
                                            imp.error_label_revealer.set_visible(true);
                                            imp.error_label_revealer.set_reveal_child(true);
                                            imp.error_label.set_text(&e.to_string());
                                        }
                                    }
                                }),
                            );
                        }

                        gtk::Inhibit(true)
                    }
                    None => {
                        glib::idle_add_local_once(clone!(@weak obj => move || {
                            obj.close();
                        }));
                        gtk::Inhibit(true)
                    }
                }
            });

            let key_events = gtk::EventControllerKey::new();
            obj.add_controller(&key_events);
            key_events.connect_key_pressed(
                clone!(@weak obj => @default-return gtk::Inhibit(false), move |_, key, _, _| {
                    gtk::Inhibit(
                        if key == gdk::Key::Escape {
                            obj.imp().response.replace(Some("close".to_string()));
                            obj.close();
                            true
                        } else {
                            false
                        }
                    )
                }),
            );

            self.entry_row
                .connect_changed(clone!(@weak obj => move |entry| {
                    let imp = obj.imp();
                    imp.entry_row.remove_css_class("error");
                    imp.error_label_revealer.set_reveal_child(false);
                    obj.set_response_enabled("rename", !entry.text().is_empty());
                }));

            self.error_label_revealer.connect_child_revealed_notify(
                clone!(@weak obj => move |revealer| {
                    if !revealer.reveals_child() {
                        revealer.set_visible(false);
                    }
                }),
            );

            obj.set_heading_use_markup(true);
            Self::Type::this_expression("container")
                .chain_property::<model::Container>("name")
                .chain_closure::<String>(closure!(|_: Self::Type, name: String| {
                    format!(
                        "{}\n<span weight=\"bold\">«{}»</span>",
                        gettext("Rename Container"),
                        name
                    )
                }))
                .bind(obj, "heading", Some(obj));
        }
    }

    impl WidgetImpl for RenameDialog {}
    impl WindowImpl for RenameDialog {}
    impl MessageDialogImpl for RenameDialog {}
}

glib::wrapper! {
    pub(crate) struct RenameDialog(ObjectSubclass<imp::RenameDialog>)
        @extends gtk::Widget, gtk::Window, adw::MessageDialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl From<Option<model::Container>> for RenameDialog {
    fn from(container: Option<model::Container>) -> Self {
        glib::Object::builder::<Self>()
            .property("container", &container)
            .build()
    }
}

impl RenameDialog {
    pub(crate) fn container(&self) -> Option<model::Container> {
        self.imp().container.upgrade()
    }
}
