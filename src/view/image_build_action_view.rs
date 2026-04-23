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
    #[properties(wrapper_type = super::ImageBuildActionView)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/image_build_action_view.ui")]
    pub(crate) struct ImageBuildActionView {
        #[property(get, set, construct_only, nullable)]
        pub(super) action: glib::WeakRef<model::ImageBuildAction>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageBuildActionView {
        const NAME: &'static str = "PdsImageBuildActionView";
        type Type = super::ImageBuildActionView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImageBuildActionView {
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

    impl WidgetImpl for ImageBuildActionView {}
}

glib::wrapper! {
    pub(crate) struct ImageBuildActionView(ObjectSubclass<imp::ImageBuildActionView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::ImageBuildAction> for view::ActionDialog {
    fn from(value: &model::ImageBuildAction) -> Self {
        Self::new(
            value.upcast_ref(),
            &gettext("Build Image"),
            None,
            &glib::Object::builder::<ImageBuildActionView>()
                .property("action", value)
                .build(),
            400,
        )
    }
}
