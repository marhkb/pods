use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::model;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::PodsPruneActionView)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/pods_prune_action_view.ui")]
    pub(crate) struct PodsPruneActionView {
        #[property(get, set, construct_only, nullable)]
        pub(super) action: glib::WeakRef<model::PodsPruneAction>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodsPruneActionView {
        const NAME: &'static str = "PdsPodsPruneActionView";
        type Type = super::PodsPruneActionView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PodsPruneActionView {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for PodsPruneActionView {}
}

glib::wrapper! {
    pub(crate) struct PodsPruneActionView(ObjectSubclass<imp::PodsPruneActionView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::PodsPruneAction> for view::ActionDialog {
    fn from(value: &model::PodsPruneAction) -> Self {
        Self::new(
            value.upcast_ref(),
            &gettext("Prune Pods"),
            None,
            &glib::Object::builder::<PodsPruneActionView>()
                .property("action", value)
                .build(),
            400,
        )
    }
}
