use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::model;
use crate::view;

const ACTION_PRUNE: &str = "pods-prune-opts-dialog.prune";

mod imp {
    use super::*;

    #[derive(Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::PodsPruneOptsDialog)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/pods_prune_opts_dialog.ui")]
    pub(crate) struct PodsPruneOptsDialog {
        #[property(get, set, construct_only, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodsPruneOptsDialog {
        const NAME: &'static str = "PdsPodsPruneOptsDialog";
        type Type = super::PodsPruneOptsDialog;
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

    impl ObjectImpl for PodsPruneOptsDialog {
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

    impl WidgetImpl for PodsPruneOptsDialog {}
    impl AdwDialogImpl for PodsPruneOptsDialog {}
}

glib::wrapper! {
    pub(crate) struct PodsPruneOptsDialog(ObjectSubclass<imp::PodsPruneOptsDialog>)
        @extends gtk::Widget, adw::Dialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl From<&model::Client> for PodsPruneOptsDialog {
    fn from(value: &model::Client) -> Self {
        glib::Object::builder().property("client", value).build()
    }
}

impl PodsPruneOptsDialog {
    fn close_and_prune(&self) {
        self.close();

        let Some(action_list) = self.client().map(|client| client.action_list2()) else {
            return;
        };

        view::ActionDialog::from(&action_list.prune_pods()).present(Some(self));
    }
}
