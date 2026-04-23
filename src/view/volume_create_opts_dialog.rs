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

const ACTION_CREATE_VOLUME: &str = "volume-create-opts-dialog.create-volume";

mod imp {
    use super::*;

    #[derive(Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::VolumeCreateOptsDialog)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/volume_create_opts_dialog.ui")]
    pub(crate) struct VolumeCreateOptsDialog {
        #[property(get, set, construct_only)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[property(get, set, construct_only)]
        pub(super) opts: OnceCell<model::BoxedVolumeCreateOpts>,

        #[template_child]
        pub(super) name_entry_row: TemplateChild<widget::RandomNameEntryRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VolumeCreateOptsDialog {
        const NAME: &'static str = "PdsVolumeCreateOptsDialog";
        type Type = super::VolumeCreateOptsDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_CREATE_VOLUME, None, |widget, _, _| {
                widget.close_and_create();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for VolumeCreateOptsDialog {
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

            if let Some(name) = self.obj().opts().name.as_deref() {
                self.name_entry_row.set_text(name);
            }
        }
    }

    impl WidgetImpl for VolumeCreateOptsDialog {
        fn map(&self) {
            self.parent_map();
            self.name_entry_row.grab_focus();
        }
    }

    impl AdwDialogImpl for VolumeCreateOptsDialog {}
}

glib::wrapper! {
    pub(crate) struct VolumeCreateOptsDialog(ObjectSubclass<imp::VolumeCreateOptsDialog>)
        @extends gtk::Widget, adw::Dialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl From<&model::Client> for VolumeCreateOptsDialog {
    fn from(value: &model::Client) -> Self {
        Self::new(value, None)
    }
}

impl VolumeCreateOptsDialog {
    pub(crate) fn new(client: &model::Client, opts: Option<model::BoxedVolumeCreateOpts>) -> Self {
        glib::Object::builder()
            .property("client", client)
            .property("opts", opts.unwrap_or_default())
            .build()
    }

    pub(crate) fn close_and_create(&self) {
        self.close();

        let Some(action_list) = self.client().map(|client| client.action_list2()) else {
            return;
        };

        let opts = engine::opts::VolumeCreateOpts {
            name: Some(self.imp().name_entry_row.text().into())
                .filter(|name: &String| !name.is_empty()),
        };

        view::ActionDialog::from(&action_list.create_volume(opts)).present(Some(self));
    }
}
