use std::collections::HashMap;

use adw::subclass::prelude::*;
use adw::traits::BinExt;
use gettextrs::gettext;
use glib::clone;
use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use serde::Deserialize;
use serde::Serialize;

use crate::model;
use crate::podman;
use crate::utils;
use crate::view;
use crate::RUNTIME;

const ACTION_PUSH: &str = "repo-tag-push-page.push";

#[derive(Debug, Serialize, Deserialize)]
enum RegistryAuth {
    Password { username: String, password: String },
    Token(String),
}

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::PushPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/repo-tag/push-page.ui")]
    pub(crate) struct PushPage {
        #[property(get, set, construct_only, nullable)]
        pub(super) repo_tag: glib::WeakRef<model::RepoTag>,
        #[template_child]
        pub(super) toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) window_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub(super) push_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) tls_verify_row_switch: TemplateChild<gtk::Switch>,
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
        pub(super) save_credentials_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) action_page_bin: TemplateChild<adw::Bin>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PushPage {
        const NAME: &'static str = "PdsRepoTagPushPage";
        type Type = super::PushPage;
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

    impl ObjectImpl for PushPage {
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

            self.password_toggle_button
                .connect_active_notify(clone!(@weak obj => move |button| {
                    let is_active = button.is_active();

                    let imp = obj. imp();
                    imp.username_entry_row.set_visible(is_active);
                    imp.password_entry_row.set_visible(is_active);
                    imp.token_entry_row.set_visible(!is_active);
                }));

            if let Some(repo_tag) = obj.repo_tag() {
                self.window_title.set_subtitle(&repo_tag.full());

                match crate::KEYRING.get() {
                    Some(keyring) => {
                        let host = repo_tag.host();
                        let namespace = repo_tag.namespace();

                        utils::do_async(
                            async move {
                                match keyring
                                    .search_items(attributes(&host, &namespace))
                                    .await
                                    .map_err(anyhow::Error::from)
                                {
                                    Ok(items) => {
                                        let item = items.get(0)?;
                                        Some(
                                            item.secret()
                                                .await
                                                .map_err(anyhow::Error::from)
                                                .and_then(|secret| {
                                                    serde_json::from_slice::<RegistryAuth>(
                                                        secret.as_slice(),
                                                    )
                                                    .map_err(anyhow::Error::from)
                                                }),
                                        )
                                    }
                                    Err(e) => Some(Err(e)),
                                }
                            },
                            clone!(@weak obj => move |maybe| {
                                let imp = obj.imp();

                                imp.login_group.set_sensitive(true);

                                if let Some(result) = maybe {
                                    match result {
                                        Ok(auth) => {
                                            imp.login_switch.set_active(true);
                                            imp.save_credentials_switch.set_active(true);

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
                                                imp.toast_overlay.upcast_ref(),
                                                &gettext("Error on accessing keyring"),
                                                &e.to_string()
                                            );
                                        }
                                    }
                                }
                            }),
                        );
                    }
                    None => self.login_group.set_sensitive(true),
                }
            }
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for PushPage {
        fn root(&self) {
            self.parent_root();
            utils::root(self.obj().upcast_ref()).set_default_widget(Some(&*self.push_button));
        }

        fn unroot(&self) {
            utils::root(self.obj().upcast_ref()).set_default_widget(gtk::Widget::NONE);
            self.parent_unroot()
        }
    }
}

glib::wrapper! {
    pub(crate) struct PushPage(ObjectSubclass<imp::PushPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::RepoTag> for PushPage {
    fn from(repo_tag: &model::RepoTag) -> Self {
        glib::Object::builder()
            .property("repo-tag", repo_tag)
            .build()
    }
}

impl PushPage {
    pub(crate) fn push(&self) {
        if let Some(repo_tag) = self.repo_tag() {
            if let Some(image) = repo_tag.repo_tag_list().and_then(|list| list.image()) {
                if let Some(client) = image.image_list().and_then(|list| list.client()) {
                    let imp = self.imp();

                    let destination = repo_tag.full();

                    let opts = podman::opts::ImagePushOpts::builder()
                        .tls_verify(imp.tls_verify_row_switch.is_active())
                        .destination(&destination)
                        .quiet(false);

                    let opts = if imp.login_switch.is_active() {
                        let host = repo_tag.host();
                        let namespace = repo_tag.namespace();

                        if imp.save_credentials_switch.is_active() {
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

                                    RUNTIME.spawn({
                                        async move {
                                            keyring
                                                .create_item(
                                                    &format!("{host}:{namespace}"),
                                                    attributes(&host, &namespace),
                                                    serde_json::to_vec(&secret).unwrap(),
                                                    true,
                                                )
                                                .await
                                                .unwrap();
                                        }
                                    });
                                }
                                None => {
                                    log::error!("Cannot save credentials, because secret service isn't available.");
                                    utils::show_error_toast(
                                        imp.toast_overlay.upcast_ref(),
                                        &gettext("Error saving credentials"),
                                        &gettext("Secret Service is not available"),
                                    );
                                }
                            }
                        } else if let Some(keyring) = crate::KEYRING.get() {
                            RUNTIME.spawn({
                                async move {
                                    keyring.delete(attributes(&host, &namespace)).await.unwrap();
                                }
                            });
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

                    imp.action_page_bin.set_child(Some(&page));
                    imp.stack.set_visible_child(&*imp.action_page_bin);
                }
            }
        }
    }
}

fn attributes<'a>(host: &'a str, namespace: &'a str) -> HashMap<&'a str, &'a str> {
    HashMap::from([("host", host), ("namespace", namespace)])
}
