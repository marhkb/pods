use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::RepoTagSimpleRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/repo_tag_simple_row.ui")]
    pub(crate) struct RepoTagSimpleRow {
        #[property(get, set, construct_only, nullable)]
        pub(super) repo_tag: glib::WeakRef<model::RepoTag>,

        #[template_child]
        pub(super) repo_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) tag_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RepoTagSimpleRow {
        const NAME: &'static str = "PdsRepoTagSimpleRow";
        type Type = super::RepoTagSimpleRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RepoTagSimpleRow {
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

    impl WidgetImpl for RepoTagSimpleRow {}
    impl ListBoxRowImpl for RepoTagSimpleRow {}
}

glib::wrapper! {
    pub(crate) struct RepoTagSimpleRow(ObjectSubclass<imp::RepoTagSimpleRow>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::Actionable, gtk::ConstraintTarget;
}

impl From<&model::RepoTag> for RepoTagSimpleRow {
    fn from(repo_tag: &model::RepoTag) -> Self {
        glib::Object::builder()
            .property("repo-tag", repo_tag)
            .build()
    }
}
