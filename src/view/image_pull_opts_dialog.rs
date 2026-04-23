use std::cell::OnceCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::glib;

use crate::engine;
use crate::model;
use crate::view;

const ACTION_PULL: &str = "image-pull-opts-dialog.pull";

mod imp {
    use super::*;

    #[derive(Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImagePullOptsDialog)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/image_pull_opts_dialog.ui")]
    pub(crate) struct ImagePullOptsDialog {
        #[property(get, set, construct_only, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[property(get, set, construct_only)]
        pub(super) opts: OnceCell<model::BoxedImagePullOpts>,

        #[template_child]
        pub(super) image_suggestion_entry_row: TemplateChild<view::ImageSuggestionEntryRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagePullOptsDialog {
        const NAME: &'static str = "PdsImagePullOptsDialog";
        type Type = super::ImagePullOptsDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action(ACTION_PULL, None, |widget, _, _| {
                widget.close_and_pull();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImagePullOptsDialog {
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

            self.image_suggestion_entry_row
                .set_text(&obj.opts().reference);
        }
    }

    impl WidgetImpl for ImagePullOptsDialog {
        fn map(&self) {
            self.parent_map();
            self.image_suggestion_entry_row.grab_focus();
        }
    }

    impl AdwDialogImpl for ImagePullOptsDialog {}

    #[gtk::template_callbacks]
    impl ImagePullOptsDialog {
        #[template_callback]
        fn on_image_suggestion_entry_changed(&self) {
            self.obj().action_set_enabled(
                ACTION_PULL,
                !self.image_suggestion_entry_row.text().is_empty(),
            );
        }

        #[template_callback]
        fn on_image_suggestion_entry_activated(&self) {
            self.obj().close_and_pull();
        }

        #[template_callback]
        fn on_image_suggestion_entry_row_key_pressed(
            &self,
            key: gdk::Key,
            _: u32,
            _: gdk::ModifierType,
            _: &gtk::EventControllerKey,
        ) -> glib::Propagation {
            if key == gdk::Key::Return || key == gdk::Key::KP_Enter {
                self.obj().close_and_pull();
            } else if key == gdk::Key::Escape {
                self.obj().close();
            }

            glib::Propagation::Proceed
        }
    }
}

glib::wrapper! {
    pub(crate) struct ImagePullOptsDialog(ObjectSubclass<imp::ImagePullOptsDialog>)
        @extends gtk::Widget, adw::Dialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl ImagePullOptsDialog {
    pub(crate) fn new(client: &model::Client, opts: Option<model::BoxedImagePullOpts>) -> Self {
        glib::Object::builder()
            .property("client", client)
            .property("opts", opts.unwrap_or_default())
            .build()
    }

    fn close_and_pull(&self) {
        self.close();

        let Some(action_list) = self.client().map(|client| client.action_list2()) else {
            return;
        };

        let imp = self.imp();

        let opts = engine::opts::ImagePullOpts {
            reference: imp.image_suggestion_entry_row.text().into(),
            ..Default::default()
        };

        view::ActionDialog::from(&action_list.pull_image(opts)).present(Some(self));
    }
}
