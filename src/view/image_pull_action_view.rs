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
    #[properties(wrapper_type = super::ImagePullActionView)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/image_pull_action_view.ui")]
    pub(crate) struct ImagePullActionView {
        #[property(get, set, construct_only, nullable)]
        pub(super) action: glib::WeakRef<model::ImagePullAction>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagePullActionView {
        const NAME: &'static str = "PdsImagePullActionView";
        type Type = super::ImagePullActionView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImagePullActionView {
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

    impl WidgetImpl for ImagePullActionView {}
}

glib::wrapper! {
    pub(crate) struct ImagePullActionView(ObjectSubclass<imp::ImagePullActionView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::ImagePullAction> for view::ActionDialog {
    fn from(value: &model::ImagePullAction) -> Self {
        Self::new(
            value.upcast_ref(),
            &gettext("Pull Image"),
            Some(&value.opts().reference),
            &glib::Object::builder::<ImagePullActionView>()
                .property("action", value)
                .build(),
            400,
        )
    }
}
