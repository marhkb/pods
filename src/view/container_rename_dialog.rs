use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::gdk;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::widget;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainerRenameDialog)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/container_rename_dialog.ui")]
    pub(crate) struct ContainerRenameDialog {
        pub(super) close_request_handler_id: RefCell<Option<glib::SignalHandlerId>>,
        #[property(get, set, construct_only, nullable)]
        pub(super) container: glib::WeakRef<model::Container>,
        #[template_child]
        pub(super) entry_row: TemplateChild<widget::RandomNameEntryRow>,
        #[template_child]
        pub(super) error_label_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub(super) error_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerRenameDialog {
        const NAME: &'static str = "PdsContainerRenameDialog";
        type Type = super::ContainerRenameDialog;
        type ParentType = adw::MessageDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl ContainerRenameDialog {
        #[template_callback]
        fn on_key_pressed(
            &self,
            key: gdk::Key,
            _: u32,
            _: gdk::ModifierType,
            _: &gtk::EventControllerKey,
        ) -> glib::Propagation {
            if key == gdk::Key::Escape {
                self.obj().force_close();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        }

        #[template_callback]
        fn on_response(&self, response: &str) {
            let obj = &*self.obj();

            if response == "close" {
                obj.force_close();
                return;
            }

            if let Some(container) = obj.container() {
                let new_name = self.entry_row.text().to_string();
                container.rename(
                    new_name,
                    clone!(@weak obj => move |result| match result {
                        Ok(_) => obj.force_close(),
                        Err(e) => {
                            let imp = obj.imp();

                            imp.entry_row.add_css_class("error");
                            imp.error_label_revealer.set_visible(true);
                            imp.error_label_revealer.set_reveal_child(true);
                            imp.error_label.set_text(&e.to_string());

                            imp.entry_row.grab_focus();
                        }
                    }),
                );
            }
        }

        #[template_callback]
        fn on_entry_row_changed(&self) {
            self.entry_row.remove_css_class("error");
            self.error_label_revealer.set_reveal_child(false);
            self.obj()
                .set_response_enabled("rename", !self.entry_row.text().is_empty());
        }

        #[template_callback]
        fn on_error_label_revealer_notify_child_revealed(&self) {
            if !self.error_label_revealer.reveals_child() {
                self.error_label_revealer.set_visible(false);
            }
        }
    }

    impl ObjectImpl for ContainerRenameDialog {
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

            if let Some(container) = obj.container() {
                container.connect_deleted(clone!(@weak obj => move |_| {
                    obj.force_close();
                }));
                self.entry_row.set_text(&container.name());
            }

            let handler_id = obj.connect_close_request(|_| glib::Propagation::Stop);
            self.close_request_handler_id.replace(Some(handler_id));

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

            self.entry_row.grab_focus();
        }
    }

    impl WidgetImpl for ContainerRenameDialog {}
    impl WindowImpl for ContainerRenameDialog {}
    impl MessageDialogImpl for ContainerRenameDialog {}
}

glib::wrapper! {
    pub(crate) struct ContainerRenameDialog(ObjectSubclass<imp::ContainerRenameDialog>)
        @extends gtk::Widget, gtk::Window, adw::MessageDialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl From<&model::Container> for ContainerRenameDialog {
    fn from(container: &model::Container) -> Self {
        glib::Object::builder()
            .property("container", container)
            .build()
    }
}

impl ContainerRenameDialog {
    pub(crate) fn force_close(&self) {
        if let Some(handler_id) = self.imp().close_request_handler_id.replace(None) {
            self.disconnect(handler_id);
            self.close();
        }
    }
}
