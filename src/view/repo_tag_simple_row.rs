use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::clone;
use glib::Properties;
use gtk::glib;
use gtk::CompositeTemplate;

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
        pub(super) label: TemplateChild<gtk::Label>,
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

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            let style_manager = adw::StyleManager::default();
            style_manager.connect_dark_notify(clone!(@weak obj => move |style_manager| {
                obj.set_label(style_manager);
            }));
            style_manager.connect_high_contrast_notify(clone!(@weak obj => move |style_manager| {
                obj.set_label(style_manager);
            }));

            obj.set_label(&style_manager);
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

impl RepoTagSimpleRow {
    fn set_label(&self, style_manager: &adw::StyleManager) {
        if let Some(repo_tag) = self.repo_tag() {
            let repo = repo_tag.repo();
            let repo = if style_manager.is_high_contrast() {
                repo
            } else {
                format!("<span alpha=\"55%\">{repo}</span>")
            };

            let tag = format!(
                "<span foreground=\"{}\"{}>{}</span>",
                if style_manager.is_dark() {
                    "#78aeed"
                } else {
                    "#1c71d8"
                },
                if style_manager.is_high_contrast() {
                    " weight=\"bold\""
                } else {
                    ""
                },
                repo_tag.tag(),
            );

            self.imp().label.set_markup(&if repo_tag.to_be_deleted() {
                format!("<s>{repo} {tag}</s>")
            } else {
                format!("{repo} {tag}")
            });
        }
    }
}
