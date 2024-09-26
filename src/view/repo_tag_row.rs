use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::clone;
use glib::Properties;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::podman;
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
        #[template_child]
        pub(super) label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RepoTagRow {
        const NAME: &'static str = "PdsRepoTagRow";
        type Type = super::RepoTagRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_UPDATE, None, |widget, _, _| {
                widget.update();
            });
            klass.install_action(ACTION_PUSH, None, |widget, _, _| {
                widget.push();
            });
            klass.install_action(ACTION_UNTAG, None, |widget, _, _| {
                widget.untag();
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

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            let style_manager = adw::StyleManager::default();
            style_manager.connect_high_contrast_notify(clone!(@weak obj => move |style_manager| {
                obj.set_label(style_manager);
            }));
            style_manager.connect_accent_color_notify(clone!(@weak obj => move |style_manager| {
                obj.set_label(style_manager);
            }));

            if let Some(repo_tag) = obj.repo_tag() {
                repo_tag.connect_notify_local(
                    Some("to-be-deleted"),
                    clone!(@weak obj, @weak style_manager => move |_, _| {
                        obj.set_label(&style_manager);
                    }),
                );
            }

            obj.set_label(&style_manager);
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
    fn set_label(&self, style_manager: &adw::StyleManager) {
        if let Some(repo_tag) = self.repo_tag() {
            let repo = repo_tag.repo();

            let accent_color = style_manager.accent_color_rgba();
            let tag = format!(
                "<span foreground=\"#{:02x}{:02x}{:02x}\"{}>{}</span>",
                (accent_color.red() * 255.0) as i32,
                (accent_color.green() * 255.0) as i32,
                (accent_color.blue() * 255.0) as i32,
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

    fn update(&self) {
        if let Some(repo_tag) = self.repo_tag() {
            if let Some(action_list) = repo_tag
                .repo_tag_list()
                .as_ref()
                .and_then(model::RepoTagList::image)
                .as_ref()
                .and_then(model::Image::image_list)
                .as_ref()
                .and_then(model::ImageList::client)
                .as_ref()
                .map(model::Client::action_list)
            {
                let reference = repo_tag.full();

                action_list.download_image(
                    &reference,
                    podman::opts::PullOpts::builder()
                        .reference(&reference)
                        .policy(podman::opts::PullPolicy::Newer)
                        .build(),
                );
            }
        }
    }

    fn push(&self) {
        if let Some(repo_tag) = self.repo_tag() {
            utils::show_dialog(
                self.upcast_ref(),
                view::RepoTagPushPage::from(&repo_tag).upcast_ref(),
            );
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

                let repo = repo_tag.repo();
                let tag = repo_tag.tag();
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
                            obj.upcast_ref(),
                            &gettext("Error on untagging image"),
                            &e.to_string()
                        );
                    }),
                );
            }
        }
    }
}
