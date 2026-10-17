use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::engine;
use crate::gettext::gettext;
use crate::model;
use crate::rt;
use crate::utils;
use crate::view;

const ACTION_UPDATE: &str = "repo-tag-row.update";
const ACTION_PUSH: &str = "repo-tag-row.push";
const ACTION_UNTAG: &str = "repo-tag-row.untag";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::RepoTagRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/repo_tag_row.ui")]
    pub(crate) struct RepoTagRow {
        #[property(get, set, construct_only, nullable)]
        pub(super) repo_tag: glib::WeakRef<model::RepoTag>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RepoTagRow {
        const NAME: &'static str = "PdsRepoTagRow";
        type Type = super::RepoTagRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action(ACTION_UPDATE, None, |widget, _, _| {
                widget.update();
            });
            klass.install_action(ACTION_PUSH, None, |widget, _, _| {
                widget.push();
            });
            klass.install_action_async(ACTION_UNTAG, None, async |widget, _, _| {
                widget.untag().await;
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RepoTagRow {
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

    #[gtk::template_callbacks]
    impl RepoTagRow {
        #[template_callback]
        fn repo_tag_markup(&self, full: String, to_be_deleted: bool) -> String {
            let full = glib::markup_escape_text(&full);
            if to_be_deleted {
                format!("<s>{full}</s>")
            } else {
                full.into()
            }
        }
    }

    impl WidgetImpl for RepoTagRow {}
    impl ListBoxRowImpl for RepoTagRow {}
}

glib::wrapper! {
    pub(crate) struct RepoTagRow(ObjectSubclass<imp::RepoTagRow>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::Actionable, gtk::ConstraintTarget;
}

impl From<&model::RepoTag> for RepoTagRow {
    fn from(repo_tag: &model::RepoTag) -> Self {
        glib::Object::builder()
            .property("repo-tag", repo_tag)
            .build()
    }
}

impl RepoTagRow {
    fn update(&self) {
        let Some(repo_tag) = self.repo_tag() else {
            return;
        };

        let Some(actions) = repo_tag
            .repo_tag_list()
            .and_then(|repo_tag_list| repo_tag_list.image())
            .and_then(|image| image.image_list())
            .and_then(|image_list| image_list.client())
            .map(|client| client.actions())
        else {
            return;
        };

        actions.pull_image(engine::opts::ImagePullOpts {
            reference: repo_tag.full(),
        });
    }

    fn push(&self) {
        if let Some(repo_tag) = self.repo_tag() {
            view::RepoTagPushOptsDialog::new(&repo_tag, None).present(Some(self));
        }
    }

    async fn untag(&self) {
        let repo_tag = self.repo_tag().unwrap();
        repo_tag.set_to_be_deleted(true);

        let result = rt::Promise::new({
            let image = repo_tag
                .repo_tag_list()
                .unwrap()
                .image()
                .unwrap()
                .api()
                .unwrap();
            let repo = repo_tag.repo();
            let tag = repo_tag.tag();
            async move { image.untag(&repo, &tag).await }
        })
        .exec()
        .await;

        if let Err(e) = result {
            repo_tag.set_to_be_deleted(false);

            log::warn!("Error on untagging image: {e}");
            utils::show_error_toast(self, &gettext("Could not untag image"), &e.to_string());
        }
    }
}
