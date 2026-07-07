use std::cell::OnceCell;
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

const ACTION_PUSH: &str = "repo-tag-push-opts-dialog.push";

#[derive(Debug, Serialize, Deserialize)]
enum RegistryAuth {
    Basic { username: String, password: String },
    Token(String),
}

mod imp {
    use super::*;

    #[derive(Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::RepoTagPushOptsDialog)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/repo_tag_push_opts_dialog.ui")]
    pub(crate) struct RepoTagPushOptsDialog {
        #[property(get, set, construct_only, nullable)]
        pub(super) repo_tag: glib::WeakRef<model::RepoTag>,
        #[property(get, set, construct_only)]
        pub(super) opts: OnceCell<model::BoxedImagePushOpts>,

        #[template_child]
        pub(super) toast_overlay: TemplateChild<adw::ToastOverlay>,
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
    impl ObjectSubclass for RepoTagPushOptsDialog {
        const NAME: &'static str = "PdsRepoTagPushOptsDialog";
        type Type = super::RepoTagPushOptsDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_PUSH, None, |widget, _, _| {
                widget.close_and_push();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for RepoTagPushOptsDialog {
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

            let opts = obj.opts();

            self.tls_verify_switch_row.set_active(opts.tls_verify);
            match opts.credentials {
                Some(ref credentials) => {
                    self.login_switch.set_active(true);

                    match credentials {
                        engine::auth::Credentials::BasicAuth { username, password } => {
                            self.auth_toggle_group.set_active_name(Some("basic"));
                            self.username_entry_row.set_text(username);
                            self.password_entry_row.set_text(password);
                            self.token_entry_row.set_text("");
                        }
                        engine::auth::Credentials::IdentityToken(token) => {
                            self.auth_toggle_group.set_active_name(Some("token"));
                            self.username_entry_row.set_text("");
                            self.password_entry_row.set_text("");
                            self.token_entry_row.set_text(token);
                        }
                    }
                }
                None => {
                    self.login_switch.set_active(false);
                    self.username_entry_row.set_text("");
                    self.password_entry_row.set_text("");
                    self.token_entry_row.set_text("");
                }
            }
        }
    }

    impl WidgetImpl for RepoTagPushOptsDialog {
        fn map(&self) {
            self.parent_map();
            self.username_entry_row.grab_focus();
        }
    }

    impl AdwDialogImpl for RepoTagPushOptsDialog {}
}

glib::wrapper! {
    pub(crate) struct RepoTagPushOptsDialog(ObjectSubclass<imp::RepoTagPushOptsDialog>)
        @extends gtk::Widget, adw::Dialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl RepoTagPushOptsDialog {
    pub(crate) fn new(repo_tag: &model::RepoTag, opts: Option<model::BoxedImagePushOpts>) -> Self {
        glib::Object::builder()
            .property("repo-tag", repo_tag)
            .property("opts", opts.unwrap_or_default())
            .build()
    }

    fn close_and_push(&self) {
        self.close();

        let Some(repo_tag) = self.repo_tag() else {
            return;
        };

        let Some(action_list) = repo_tag
            .repo_tag_list()
            .and_then(|repo_tag_list| repo_tag_list.image())
            .and_then(|image| image.image_list())
            .and_then(|image_list| image_list.client())
            .map(|client| client.action_list())
        else {
            return;
        };

        view::ActionDialog::from(&action_list.push_image(&repo_tag, self.create_opts(&repo_tag)))
            .present(Some(self));
    }

    fn create_opts(&self, repo_tag: &model::RepoTag) -> engine::opts::ImagePushOpts {
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

        engine::opts::ImagePushOpts {
            credentials,
            repo: repo_tag.repo(),
            tag: repo_tag.tag(),
            tls_verify: imp.tls_verify_switch_row.is_active(),
        }
    }
}
