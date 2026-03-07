use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::RepoTagAddDialog)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/repo_tag_add_dialog.ui")]
    pub(crate) struct RepoTagAddDialog {
        #[property(get)]
        pub(super) repo: RefCell<String>,
        #[property(get)]
        pub(super) tag: RefCell<String>,
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
        type ParentType = adw::AlertDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RepoTagAddDialog {
        fn constructed(&self) {
            self.parent_constructed();
            self.on_entry_row_changed();
        }

        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }
    }
    impl WidgetImpl for RepoTagAddDialog {}
    impl AdwDialogImpl for RepoTagAddDialog {}
    impl AdwAlertDialogImpl for RepoTagAddDialog {}

    #[gtk::template_callbacks]
    impl RepoTagAddDialog {
        #[template_callback]
        fn on_entry_row_changed(&self) {
            let obj = &*self.obj();

            let repo_tag = self.entry_row.text();
            match split_repo_tag(repo_tag.as_str()) {
                Some((repo, tag)) => {
                    self.entry_row.remove_css_class("error");
                    self.error_label_revealer.set_reveal_child(false);

                    self.set_repo(repo);
                    self.set_tag(tag);

                    obj.set_response_enabled("add", true);
                }
                None => {
                    self.entry_row.add_css_class("error");
                    self.error_label_revealer.set_visible(true);
                    self.error_label_revealer.set_reveal_child(true);
                    self.error_label
                        .set_text(&gettext("Repo tag must contain a colon “:”"));

                    obj.set_response_enabled("add", false);
                }
            }
        }

        #[template_callback]
        fn on_error_label_revealer_notify_child_revealed(&self) {
            if !self.error_label_revealer.reveals_child() {
                self.error_label_revealer.set_visible(false);
            }
        }

        pub(super) fn set_repo(&self, value: &str) {
            let obj = &*self.obj();
            if obj.repo() == value {
                return;
            }
            self.repo.replace(value.to_owned());
            obj.notify_repo();
        }

        pub(super) fn set_tag(&self, value: &str) {
            let obj = &*self.obj();
            if obj.tag() == value {
                return;
            }
            self.tag.replace(value.to_owned());
            obj.notify_tag();
        }
    }
}

fn split_repo_tag(repo_tag: &str) -> Option<(&str, &str)> {
    repo_tag.rsplit_once(':').filter(|(repo, _)| {
        let split_at = repo.len();
        repo_tag
            .rfind('/')
            .map(|index| index < split_at)
            .unwrap_or(true)
    })
}

glib::wrapper! {
    pub(crate) struct RepoTagAddDialog(ObjectSubclass<imp::RepoTagAddDialog>)
        @extends gtk::Widget, adw::Dialog, adw::AlertDialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl Default for RepoTagAddDialog {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
