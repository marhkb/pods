use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use gtk::gdk;
use gtk::glib;
use gtk::glib::clone;
use gtk::CompositeTemplate;

use crate::model;
use crate::podman;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/repo_tag_add_dialog.ui")]
    pub(crate) struct RepoTagAddDialog {
        pub(super) close_request_handler_id: RefCell<Option<glib::SignalHandlerId>>,
        pub(super) image: glib::WeakRef<model::Image>,
        #[template_child]
        pub(super) entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) error_label_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub(super) error_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RepoTagAddDialog {
        const NAME: &'static str = "PdsRepoTagAddDialog";
        type Type = super::RepoTagAddDialog;
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
    impl RepoTagAddDialog {
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

            if let Some(image) = self.image.upgrade().as_ref().and_then(model::Image::api) {
                let repo_tag = self.entry_row.text();
                match repo_tag.split_once(':') {
                    Some((repo, tag)) => {
                        let repo = repo.trim().to_owned();
                        let tag = tag.trim().to_owned();
                        utils::do_async(
                            async move {
                                image
                                    .tag(
                                        &podman::opts::ImageTagOpts::builder()
                                            .repo(repo)
                                            .tag(tag)
                                            .build(),
                                    )
                                    .await
                            },
                            clone!(@weak obj => move |result| match result {
                                Ok(_) => obj.force_close(),
                                Err(e) => obj.set_error(&e.to_string()),
                            }),
                        );
                    }
                    None => {
                        obj.set_error(&gettext("Repo tag must contain a colon “:”"));
                    }
                }
            }
        }

        #[template_callback]
        fn on_entry_row_changed(&self) {
            self.entry_row.remove_css_class("error");
            self.error_label_revealer.set_reveal_child(false);
            self.obj()
                .set_response_enabled("add", !self.entry_row.text().is_empty());
        }

        #[template_callback]
        fn on_error_label_revealer_notify_child_revealed(&self) {
            if !self.error_label_revealer.reveals_child() {
                self.error_label_revealer.set_visible(false);
            }
        }
    }

    impl ObjectImpl for RepoTagAddDialog {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            let handler_id = obj.connect_close_request(|_| glib::Propagation::Stop);
            self.close_request_handler_id.replace(Some(handler_id));

            self.entry_row.grab_focus();
        }
    }

    impl WidgetImpl for RepoTagAddDialog {}
    impl WindowImpl for RepoTagAddDialog {}
    impl MessageDialogImpl for RepoTagAddDialog {}
}

glib::wrapper! {
    pub(crate) struct RepoTagAddDialog(ObjectSubclass<imp::RepoTagAddDialog>)
        @extends gtk::Widget, gtk::Window, adw::MessageDialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl From<&model::Image> for RepoTagAddDialog {
    fn from(image: &model::Image) -> Self {
        let obj: Self = glib::Object::builder().build();
        obj.imp().image.set(Some(image));
        obj
    }
}

impl RepoTagAddDialog {
    pub(crate) fn force_close(&self) {
        if let Some(handler_id) = self.imp().close_request_handler_id.replace(None) {
            self.disconnect(handler_id);
            self.close();
        }
    }

    fn set_error(&self, msg: &str) {
        let imp = self.imp();
        imp.entry_row.add_css_class("error");
        imp.error_label_revealer.set_visible(true);
        imp.error_label_revealer.set_reveal_child(true);
        imp.error_label.set_text(msg);
    }
}
