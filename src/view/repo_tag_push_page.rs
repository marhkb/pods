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

use crate::model;
use crate::podman;
use crate::rt;
use crate::utils;
use crate::view;

const ACTION_PUSH: &str = "repo-tag-push-page.push";

#[derive(Debug, Serialize, Deserialize)]
enum RegistryAuth {
    Password { username: String, password: String },
    Token(String),
}

mod imp {
    use super::*;
    use crate::rt;

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
        pub(super) password_toggle_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) token_toggle_button: TemplateChild<gtk::ToggleButton>,
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

            self.password_toggle_button.connect_active_notify(clone!(
                #[weak]
                obj,
                move |button| {
                    let is_active = button.is_active();

                    let imp = obj.imp();
                    imp.username_entry_row.set_visible(is_active);
                    imp.password_entry_row.set_visible(is_active);
                    imp.token_entry_row.set_visible(!is_active);
                }
            ));

            if let Some(repo_tag) = obj.repo_tag() {
                self.window_title.set_subtitle(&repo_tag.full());

                match crate::KEYRING.get() {
                    None => self.login_group.set_sensitive(true),
                    Some(keyring) => {
                        let host = repo_tag.host();
                        let namespace = repo_tag.namespace();

                        rt::Promise::new(async move {
                            match keyring
                                .search_items(&attributes(&host, &namespace))
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
                                                RegistryAuth::Password { username, password } => {
                                                    imp.password_toggle_button.set_active(true);
                                                    imp.username_entry_row.set_text(&username);
                                                    imp.password_entry_row.set_text(&password);
                                                }
                                                RegistryAuth::Token(token) => {
                                                    imp.token_toggle_button.set_active(true);
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
        let repo_tag = if let Some(repo_tag) = self.repo_tag() {
            repo_tag
        } else {
            return;
        };

        let image = if let Some(image) = repo_tag.repo_tag_list().and_then(|list| list.image()) {
            image
        } else {
            return;
        };

        let client = if let Some(client) = image.image_list().and_then(|list| list.client()) {
            client
        } else {
            return;
        };

        let imp = self.imp();

        let destination = repo_tag.full();

        let opts = podman::opts::ImagePushOpts::builder()
            .tls_verify(imp.tls_verify_switch_row.is_active())
            .destination(&destination)
            .quiet(false);

        let opts = if imp.login_switch.is_active() {
            let host = repo_tag.host();
            let namespace = repo_tag.namespace();

            if imp.save_credentials_switch_row.is_active() {
                match crate::KEYRING.get() {
                    Some(keyring) => {
                        let secret = if imp.password_toggle_button.is_active() {
                            RegistryAuth::Password {
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
                                        &format!("{host}:{namespace}"),
                                        &attributes(&host, &namespace),
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
                            .delete(&attributes(&host, &namespace))
                            .await
                            .unwrap();
                    }
                })
                .spawn();
            }

            opts.auth(
                podman::opts::RegistryAuth::builder()
                    .username(imp.username_entry_row.text())
                    .password(imp.password_entry_row.text())
                    .build(),
            )
        } else {
            opts
        };

        let page = view::ActionPage::from(&client.action_list().push_image(
            &destination,
            image.api().unwrap(),
            opts.build(),
        ));

        imp.navigation_view.push(
            &adw::NavigationPage::builder()
                .can_pop(false)
                .child(&page)
                .build(),
        );
    }
}

fn attributes<'a>(host: &'a str, namespace: &'a str) -> HashMap<&'a str, &'a str> {
    HashMap::from([("host", host), ("namespace", namespace)])
}
