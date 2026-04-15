use std::collections::HashMap;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::Properties;
use glib::clone;
use gtk::CompositeTemplate;
use gtk::glib;
use serde::Deserialize;
use serde::Serialize;

use crate::engine;
use crate::model;
use crate::rt;
use crate::utils;
use crate::view;

const ACTION_PUSH: &str = "repo-tag-push-page.push";

#[derive(Debug, Serialize, Deserialize)]
enum RegistryAuth {
    Basic { username: String, password: String },
    Token(String),
}

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::RepoTagPushPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/repo_tag_push_page.ui")]
    pub(crate) struct RepoTagPushPage {
        #[property(get, set, construct_only, nullable)]
        pub(super) repo_tag: glib::WeakRef<model::RepoTag>,
        #[template_child]
        pub(super) toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub(super) navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub(super) window_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub(super) push_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) tls_verify_switch_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) login_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) login_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) auth_toggle_group: TemplateChild<adw::ToggleGroup>,
        #[template_child]
        pub(super) username_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) password_entry_row: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        pub(super) token_entry_row: TemplateChild<adw::PasswordEntryRow>,
        #[template_child]
        pub(super) save_credentials_switch_row: TemplateChild<adw::SwitchRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for RepoTagPushPage {
        const NAME: &'static str = "PdsRepoTagPushPage";
        type Type = super::RepoTagPushPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_PUSH, None, |widget, _, _| {
                widget.push();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RepoTagPushPage {
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

            self.auth_toggle_group.connect_active_name_notify(clone!(
                #[weak]
                obj,
                move |toggle_group| {
                    let is_basic_enabled = toggle_group.active_name().as_deref() == Some("basic");

                    let imp = obj.imp();
                    imp.username_entry_row.set_visible(is_basic_enabled);
                    imp.password_entry_row.set_visible(is_basic_enabled);
                    imp.token_entry_row.set_visible(!is_basic_enabled);
                }
            ));

            if let Some(repo_tag) = obj.repo_tag() {
                self.window_title.set_subtitle(&repo_tag.full());

                match crate::KEYRING.get() {
                    None => self.login_group.set_sensitive(true),
                    Some(keyring) => {
                        let items = HashMap::from([("repo-tag", repo_tag.full())]);

                        rt::Promise::new(async move {
                            match keyring
                                .search_items(&items)
                                .await
                                .map_err(anyhow::Error::from)
                            {
                                Ok(items) => {
                                    let item = items.first()?;
                                    Some(item.secret().await.map_err(anyhow::Error::from).and_then(
                                        |secret| {
                                            serde_json::from_slice::<RegistryAuth>(
                                                secret.as_bytes(),
                                            )
                                            .map_err(anyhow::Error::from)
                                        },
                                    ))
                                }
                                Err(e) => Some(Err(e)),
                            }
                        })
                        .defer(clone!(
                            #[weak]
                            obj,
                            move |maybe| {
                                let imp = obj.imp();

                                imp.login_group.set_sensitive(true);

                                if let Some(result) = maybe {
                                    match result {
                                        Ok(auth) => {
                                            imp.login_switch.set_active(true);
                                            imp.save_credentials_switch_row.set_active(true);

                                            match auth {
                                                RegistryAuth::Basic { username, password } => {
                                                    imp.auth_toggle_group
                                                        .set_active_name(Some("basic"));
                                                    imp.username_entry_row.set_text(&username);
                                                    imp.password_entry_row.set_text(&password);
                                                }
                                                RegistryAuth::Token(token) => {
                                                    imp.auth_toggle_group
                                                        .set_active_name(Some("token"));
                                                    imp.token_entry_row.set_text(&token);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            log::error!("Error on accessing keyring: {e}");
                                            utils::show_error_toast(
                                                &*imp.toast_overlay,
                                                &gettext("Error on accessing keyring"),
                                                &e.to_string(),
                                            );
                                        }
                                    }
                                }
                            }
                        ));
                    }
                }
            }
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for RepoTagPushPage {
        fn root(&self) {
            self.parent_root();
            utils::root(&*self.obj()).set_default_widget(Some(&*self.push_button));
        }

        fn unroot(&self) {
            utils::root(&*self.obj()).set_default_widget(gtk::Widget::NONE);
            self.parent_unroot()
        }
    }
}

glib::wrapper! {
    pub(crate) struct RepoTagPushPage(ObjectSubclass<imp::RepoTagPushPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::RepoTag> for RepoTagPushPage {
    fn from(repo_tag: &model::RepoTag) -> Self {
        glib::Object::builder()
            .property("repo-tag", repo_tag)
            .build()
    }
}

impl RepoTagPushPage {
    pub(crate) fn push(&self) {
        let Some(repo_tag) = self.repo_tag() else {
            return;
        };

        let Some(image) = repo_tag
            .repo_tag_list()
            .as_ref()
            .and_then(model::RepoTagList::image)
        else {
            return;
        };

        let Some(client) = image
            .image_list()
            .as_ref()
            .and_then(model::ImageList::client)
        else {
            return;
        };

        let imp = self.imp();

        let credentials = if imp.login_switch.is_active() {
            let repo_tag = repo_tag.full();

            if imp.save_credentials_switch_row.is_active() {
                match crate::KEYRING.get() {
                    Some(keyring) => {
                        let secret =
                            if imp.auth_toggle_group.active_name().as_deref() == Some("basic") {
                                RegistryAuth::Basic {
                                    username: imp.username_entry_row.text().into(),
                                    password: imp.password_entry_row.text().into(),
                                }
                            } else {
                                RegistryAuth::Token(imp.token_entry_row.text().into())
                            };

                        rt::Promise::new({
                            async move {
                                keyring
                                    .create_item(
                                        &repo_tag,
                                        &HashMap::from([("repo-tag", &repo_tag)]),
                                        serde_json::to_vec(&secret).unwrap(),
                                        true,
                                    )
                                    .await
                                    .unwrap();
                            }
                        })
                        .spawn();
                    }
                    None => {
                        log::error!(
                            "Cannot save credentials, because secret service isn't available."
                        );
                        utils::show_error_toast(
                            &*imp.toast_overlay,
                            &gettext("Error saving credentials"),
                            &gettext("Secret Service is not available"),
                        );
                    }
                }
            } else if let Some(keyring) = crate::KEYRING.get() {
                rt::Promise::new({
                    async move {
                        keyring
                            .delete(&HashMap::from([("repo-tag", &repo_tag)]))
                            .await
                            .unwrap();
                    }
                })
                .spawn();
            }

            Some(
                if imp.auth_toggle_group.active_name().as_deref() == Some("basic") {
                    engine::auth::Credentials::BasicAuth {
                        username: imp.username_entry_row.text().into(),
                        password: imp.password_entry_row.text().into(),
                    }
                } else {
                    engine::auth::Credentials::IdentityToken(imp.token_entry_row.text().into())
                },
            )
        } else {
            None
        };

        let page = view::ActionPage::from(&client.action_list().push_image(
            image.api().unwrap(),
            repo_tag.repo(),
            engine::opts::ImagePushOpts {
                tag: repo_tag.tag(),
                tls_verify: imp.tls_verify_switch_row.is_active(),
            },
            credentials,
        ));

        imp.navigation_view.push(
            &adw::NavigationPage::builder()
                .can_pop(false)
                .child(&page)
                .build(),
        );
    }
}
