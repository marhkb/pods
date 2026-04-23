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
    #[properties(wrapper_type = super::RepoTagPushActionView)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/repo_tag_push_action_view.ui")]
    pub(crate) struct RepoTagPushActionView {
        #[property(get, set, construct_only, nullable)]
        pub(super) action: glib::WeakRef<model::ImagePushAction>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RepoTagPushActionView {
        const NAME: &'static str = "PdsRepoTagPushActionView";
        type Type = super::RepoTagPushActionView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RepoTagPushActionView {
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

    impl WidgetImpl for RepoTagPushActionView {}
}

glib::wrapper! {
    pub(crate) struct RepoTagPushActionView(ObjectSubclass<imp::RepoTagPushActionView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::ImagePushAction> for view::ActionDialog {
    fn from(value: &model::ImagePushAction) -> Self {
        let opts = value.opts();

        Self::new(
            value.upcast_ref(),
            &gettext("Push Image"),
            Some(&format!("{}:{}", opts.repo, opts.tag)),
            &glib::Object::builder::<RepoTagPushActionView>()
                .property("action", value)
                .build(),
            400,
        )
    }
}
