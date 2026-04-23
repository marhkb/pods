use std::cell::OnceCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::engine;
use crate::model;
use crate::view;
use crate::widget;

const ACTION_PRUNE: &str = "images-prune-opts-dialog.prune";

mod imp {
    use super::*;

    #[derive(Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImagesPruneOptsDialog)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/images_prune_opts_dialog.ui")]
    pub(crate) struct ImagesPruneOptsDialog {
        #[property(get, set, construct_only, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[property(get, set, construct_only)]
        pub(super) opts: OnceCell<model::BoxedImagesPruneOpts>,

        #[template_child]
        pub(super) prune_all_switch_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) prune_external_switch_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) prune_until_row: TemplateChild<widget::DateTimeRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagesPruneOptsDialog {
        const NAME: &'static str = "PdsImagesPruneOptsDialog";
        type Type = super::ImagesPruneOptsDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_PRUNE, None, |widget, _, _| {
                widget.close_and_prune();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImagesPruneOptsDialog {
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

            let opts = self.obj().opts();

            self.prune_until_row
                .set_enable_expansion(opts.until.is_some());
            self.prune_until_row.set_timestamp(
                opts.until
                    .unwrap_or_else(|| glib::DateTime::now_local().unwrap().to_unix()),
            );
        }
    }

    impl WidgetImpl for ImagesPruneOptsDialog {}
    impl AdwDialogImpl for ImagesPruneOptsDialog {}
}

glib::wrapper! {
    pub(crate) struct ImagesPruneOptsDialog(ObjectSubclass<imp::ImagesPruneOptsDialog>)
        @extends gtk::Widget, adw::Dialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl From<&model::Client> for ImagesPruneOptsDialog {
    fn from(client: &model::Client) -> Self {
        glib::Object::builder().property("client", client).build()
    }
}

impl ImagesPruneOptsDialog {
    pub(crate) fn new(client: &model::Client, opts: Option<model::BoxedImagesPruneOpts>) -> Self {
        glib::Object::builder()
            .property("client", client)
            .property("opts", opts.unwrap_or_default())
            .build()
    }

    fn close_and_prune(&self) {
        self.close();

        let Some(action_list) = self.client().map(|client| client.action_list2()) else {
            return;
        };

        let imp = self.imp();

        let opts = engine::opts::ImagesPruneOpts {
            all: imp.prune_all_switch_row.is_active(),
            external: imp.prune_external_switch_row.is_active(),
            until: imp
                .prune_until_row
                .enables_expansion()
                .then(|| imp.prune_until_row.timestamp()),
        };

        view::ActionDialog::from(&action_list.prune_images(opts)).present(Some(self));
    }
}
