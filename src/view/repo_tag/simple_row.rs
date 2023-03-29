use glib::clone;
use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::SimpleRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/repo-tag/simple-row.ui")]
    pub(crate) struct SimpleRow {
        #[property(get, set, construct_only, nullable)]
        pub(super) repo_tag: glib::WeakRef<model::RepoTag>,
        #[template_child]
        pub(super) label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SimpleRow {
        const NAME: &'static str = "PdsRepoTagSimpleRow";
        type Type = super::SimpleRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SimpleRow {
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
            style_manager.connect_dark_notify(clone!(@weak obj => move |manager| {
                obj.set_label(manager.is_dark(), manager.is_high_contrast());
            }));
            style_manager.connect_high_contrast_notify(clone!(@weak obj => move |manager| {
                obj.set_label(manager.is_dark(), manager.is_high_contrast());
            }));

            obj.set_label(style_manager.is_dark(), style_manager.is_high_contrast());
        }
    }

    impl WidgetImpl for SimpleRow {}
    impl ListBoxRowImpl for SimpleRow {}
}

glib::wrapper! {
    pub(crate) struct SimpleRow(ObjectSubclass<imp::SimpleRow>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::Actionable, gtk::ConstraintTarget;
}

impl From<&model::RepoTag> for SimpleRow {
    fn from(repo_tag: &model::RepoTag) -> Self {
        glib::Object::builder()
            .property("repo-tag", repo_tag)
            .build()
    }
}

impl SimpleRow {
    fn set_label(&self, is_dark: bool, is_hc: bool) {
        if let Some(repo_tag) = self.repo_tag() {
            let repo = repo_tag.repo();
            let repo = if is_hc {
                repo
            } else {
                format!("<span alpha=\"55%\">{repo}</span>")
            };

            let tag = format!(
                "<span foreground=\"{}\"{}>{}</span>",
                if is_dark { "#78aeed" } else { "#1c71d8" },
                if is_hc { " weight=\"bold\"" } else { "" },
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
