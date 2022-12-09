use gettextrs::gettext;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::podman;
use crate::utils;

const ACTION_UNTAG: &str = "repo-tag-row.untag";

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/repo-tag/row.ui")]
    pub(crate) struct Row {
        pub(super) repo_tag: glib::WeakRef<model::RepoTag>,
        #[template_child]
        pub(super) label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Row {
        const NAME: &'static str = "PdsRepoTagRow";
        type Type = super::Row;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action(ACTION_UNTAG, None, move |widget, _, _| {
                widget.untag();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Row {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::builder::<model::RepoTag>("repo-tag")
                    .construct_only()
                    .build()]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "repo-tag" => self.repo_tag.set(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "repo-tag" => self.obj().repo_tag().to_value(),
                _ => unimplemented!(),
            }
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
            if let Some(repo_tag) = obj.repo_tag() {
                repo_tag.connect_notify_local(
                    Some("to-be-deleted"),
                    clone!(@weak obj, @weak style_manager => move |_, _| {
                        obj.set_label(style_manager.is_dark(), style_manager.is_high_contrast());
                    }),
                );
            }

            obj.set_label(style_manager.is_dark(), style_manager.is_high_contrast());
        }
    }

    impl WidgetImpl for Row {}
    impl ListBoxRowImpl for Row {}
}

glib::wrapper! {
    pub(crate) struct Row(ObjectSubclass<imp::Row>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::Actionable, gtk::ConstraintTarget;
}

impl From<&model::RepoTag> for Row {
    fn from(repo_tag: &model::RepoTag) -> Self {
        glib::Object::builder::<Self>()
            .property("repo-tag", repo_tag)
            .build()
    }
}

impl Row {
    pub(crate) fn repo_tag(&self) -> Option<model::RepoTag> {
        self.imp().repo_tag.upgrade()
    }

    fn set_label(&self, is_dark: bool, is_hc: bool) {
        if let Some(repo_tag) = self.repo_tag() {
            let repo = repo_tag.repo();
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

    fn untag(&self) {
        if let Some(repo_tag) = self.repo_tag() {
            if let Some(image) = repo_tag
                .repo_tag_list()
                .as_ref()
                .and_then(model::RepoTagList::image)
                .as_ref()
                .and_then(model::Image::api)
            {
                repo_tag.set_to_be_deleted(true);

                let repo = repo_tag.repo().to_owned();
                let tag = repo_tag.tag().to_owned();
                utils::do_async(
                    async move {
                        image
                            .untag(
                                &podman::opts::ImageTagOpts::builder()
                                    .repo(repo)
                                    .tag(tag)
                                    .build(),
                            )
                            .await
                    },
                    clone!(@weak self as obj => move |result| if let Err(e) = result {
                        if let Some(repo_tag) = obj.repo_tag() {
                            repo_tag.set_to_be_deleted(false);
                        }
                        log::warn!("Error on untagging image: {e}");
                        utils::show_error_toast(
                            &obj,
                            &gettext("Error on untagging image"),
                            &e.to_string()
                        );
                    }),
                );
            }
        }
    }
}
