use std::cell::RefCell;

use adw::subclass::prelude::*;
use adw::traits::MessageDialogExt;
use gettextrs::gettext;
use gtk::gdk;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::unsync::OnceCell;

use crate::model;
use crate::podman;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/repo-tag/add-dialog.ui")]
    pub(crate) struct AddDialog {
        pub(super) image: glib::WeakRef<model::Image>,
        pub(super) response: RefCell<Option<String>>,
        pub(super) rename_finished: OnceCell<()>,
        #[template_child]
        pub(super) entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) error_label_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub(super) error_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AddDialog {
        const NAME: &'static str = "PdsRepoTagAddDialog";
        type Type = super::AddDialog;
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
    impl AddDialog {
        #[template_callback]
        fn on_key_pressed(
            &self,
            key: gdk::Key,
            _: u32,
            _: gdk::ModifierType,
            _: &gtk::EventControllerKey,
        ) -> gtk::Inhibit {
            gtk::Inhibit(if key == gdk::Key::Escape {
                self.response.replace(Some("close".to_string()));
                self.obj().close();
                true
            } else {
                false
            })
        }
    }

    impl ObjectImpl for AddDialog {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

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

                        if let Some(image) =
                            imp.image.upgrade().as_ref().and_then(model::Image::api)
                        {
                            let repo_tag = imp.entry_row.text();
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
                                            Ok(_) => {
                                                obj.imp().rename_finished.set(()).unwrap();
                                                obj.close();
                                            },
                                            Err(e) => {
                                                obj.set_error(&e.to_string());
                                            }
                                        }),
                                    );
                                }
                                None => {
                                    obj.set_error(&gettext("Repo tag must contain a colon “:”"));
                                }
                            }
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
        }
    }

    impl WidgetImpl for AddDialog {}
    impl WindowImpl for AddDialog {}
    impl MessageDialogImpl for AddDialog {}
}

glib::wrapper! {
    pub(crate) struct AddDialog(ObjectSubclass<imp::AddDialog>)
        @extends gtk::Widget, gtk::Window, adw::MessageDialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl From<&model::Image> for AddDialog {
    fn from(image: &model::Image) -> Self {
        let obj: Self = glib::Object::builder().build();
        obj.imp().image.set(Some(image));
        obj
    }
}

impl AddDialog {
    fn set_error(&self, msg: &str) {
        let imp = self.imp();
        imp.entry_row.add_css_class("error");
        imp.error_label_revealer.set_visible(true);
        imp.error_label_revealer.set_reveal_child(true);
        imp.error_label.set_text(msg);
    }
}
