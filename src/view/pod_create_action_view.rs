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
    #[properties(wrapper_type = super::PodCreateActionView)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/pod_create_action_view.ui")]
    pub(crate) struct PodCreateActionView {
        #[property(get, set, construct_only, nullable)]
        pub(super) action: glib::WeakRef<model::PodCreateAction>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodCreateActionView {
        const NAME: &'static str = "PdsPodCreateActionView";
        type Type = super::PodCreateActionView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PodCreateActionView {
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

    impl WidgetImpl for PodCreateActionView {}
}

glib::wrapper! {
    pub(crate) struct PodCreateActionView(ObjectSubclass<imp::PodCreateActionView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::PodCreateAction> for view::ActionDialog {
    fn from(value: &model::PodCreateAction) -> Self {
        Self::new(
            value.upcast_ref(),
            &gettext("Create Pod"),
            Some(&value.opts().name),
            &glib::Object::builder::<PodCreateActionView>()
                .property("action", value)
                .build(),
            400,
        )
    }
}
