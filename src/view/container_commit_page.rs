use std::cell::OnceCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use ashpd::WindowIdentifier;
use ashpd::desktop::account::UserInformationRequest;
use gettextrs::gettext;
use glib::Properties;
use glib::clone;
use gtk::CompositeTemplate;
use gtk::gio;
use gtk::glib;

use crate::model;
use crate::podman;
use crate::utils;
use crate::view;

const ACTION_FETCH_USERNAME: &str = "container-commit-page.fetch-username";
const ACTION_ADD_CHANGE: &str = "container-commit-page.add-change";
const ACTION_COMMIT: &str = "container-commit-page.commit";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainerCommitPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/container_commit_page.ui")]
    pub(crate) struct ContainerCommitPage {
        pub(super) changes: OnceCell<gio::ListStore>,
        #[property(get, set = Self::set_container, construct, nullable)]
        pub(super) container: glib::WeakRef<model::Container>,
        #[template_child]
        pub(super) navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub(super) commit_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) author_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) comment_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) repo_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) tag_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) format_combo_row: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub(super) format_list: TemplateChild<gtk::StringList>,
        #[template_child]
        pub(super) pause_switch_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) changes_list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerCommitPage {
        const NAME: &'static str = "PdsContainerCommitPage";
        type Type = super::ContainerCommitPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action_async(ACTION_FETCH_USERNAME, None, |widget, _, _| async move {
                widget.fetch_user_information().await;
            });
            klass.install_action(ACTION_ADD_CHANGE, None, |widget, _, _| {
                widget.add_change();
            });
            klass.install_action(ACTION_COMMIT, None, |widget, _, _| {
                widget.commit();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerCommitPage {
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

            self.changes_list_box
                .bind_model(Some(self.changes()), |item| {
                    view::ValueRow::new(item.downcast_ref().unwrap(), &gettext("Change")).upcast()
                });
            self.changes_list_box.append(
                &gtk::ListBoxRow::builder()
                    .action_name(ACTION_ADD_CHANGE)
                    .selectable(false)
                    .child(
                        &gtk::Image::builder()
                            .icon_name("list-add-symbolic")
                            .margin_top(12)
                            .margin_bottom(12)
                            .build(),
                    )
                    .build(),
            );
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ContainerCommitPage {
        fn root(&self) {
            self.parent_root();

            let widget = &*self.obj();

            glib::idle_add_local(clone!(
                #[weak]
                widget,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    widget.imp().author_entry_row.grab_focus();
                    glib::ControlFlow::Break
                }
            ));
            utils::root(widget).set_default_widget(Some(&*self.commit_button));
        }

        fn unroot(&self) {
            utils::root(&*self.obj()).set_default_widget(gtk::Widget::NONE);
            self.parent_unroot()
        }
    }

    impl ContainerCommitPage {
        pub(super) fn changes(&self) -> &gio::ListStore {
            self.changes
                .get_or_init(gio::ListStore::new::<model::Value>)
        }

        pub(super) fn set_container(&self, value: Option<&model::Container>) {
            let obj = &*self.obj();
            if obj.container().as_ref() == value {
                return;
            }

            if let Some(container) = value {
                container.connect_deleted(clone!(
                    #[weak]
                    obj,
                    move |_| {
                        obj.activate_action("win.close", None).unwrap();
                    }
                ));
            }

            self.container.set(value);
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainerCommitPage(ObjectSubclass<imp::ContainerCommitPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Container> for ContainerCommitPage {
    fn from(container: &model::Container) -> Self {
        glib::Object::builder()
            .property("container", container)
            .build()
    }
}

impl ContainerCommitPage {
    pub(crate) async fn fetch_user_information(&self) {
        let request = UserInformationRequest::default()
            .identifier(WindowIdentifier::from_native(&self.native().unwrap()).await);

        utils::do_async(
            async move {
                request
                    .send()
                    .await
                    .and_then(|user_info| user_info.response())
            },
            clone!(
                #[weak(rename_to = obj)]
                self,
                move |user_info| {
                    match user_info {
                        Ok(user_info) => obj.imp().author_entry_row.set_text(user_info.name()),
                        Err(e) => {
                            if let ashpd::Error::Portal(ashpd::PortalError::Cancelled(_)) = e {
                                utils::show_error_toast(
                                    &obj,
                                    &gettext("Error on fetching user name"),
                                    &e.to_string(),
                                );
                            }
                        }
                    }
                }
            ),
        );
    }

    pub(crate) fn add_change(&self) {
        let change = model::Value::default();

        change.connect_remove_request(clone!(
            #[weak(rename_to = obj)]
            self,
            move |change| {
                let changes = obj.imp().changes();
                if let Some(pos) = changes.find(change) {
                    changes.remove(pos);
                }
            }
        ));

        self.imp().changes().append(&change);
    }

    pub(crate) fn commit(&self) {
        if let Some(container) = self.container() {
            if let Some(api) = container.api() {
                if let Some(client) = container
                    .container_list()
                    .and_then(|container_list| container_list.client())
                {
                    let imp = self.imp();

                    let opts = podman::opts::ContainerCommitOpts::builder();

                    let opts = set_opts_builder_field(
                        opts,
                        imp.author_entry_row.text().trim(),
                        |opts, field| opts.author(field),
                    );
                    let opts = set_opts_builder_field(
                        opts,
                        imp.comment_entry_row.text().trim(),
                        |opts, field| opts.comment(field),
                    );

                    let repo = imp.repo_entry_row.text();
                    let repo = repo.trim();
                    let opts = set_opts_builder_field(opts, repo, |opts, field| opts.repo(field));

                    let tag = imp.tag_entry_row.text();
                    let tag = tag.trim();
                    let opts = set_opts_builder_field(opts, tag, |opts, field| opts.tag(field));

                    let opts = opts
                        .format(
                            imp.format_list
                                .get()
                                .string(imp.format_combo_row.selected())
                                .unwrap(),
                        )
                        .pause(imp.pause_switch_row.is_active());

                    let page = view::ActionPage::from(
                        &client.action_list().commit_container(
                            if repo.is_empty() {
                                None
                            } else {
                                Some(format!(
                                    "{}:{}",
                                    repo,
                                    if tag.is_empty() { "latest" } else { tag }
                                ))
                            }
                            .as_deref(),
                            &container.name(),
                            api,
                            opts.build(),
                        ),
                    );

                    imp.navigation_view.push(
                        &adw::NavigationPage::builder()
                            .can_pop(false)
                            .child(&page)
                            .build(),
                    );
                }
            }
        }
    }
}

fn set_opts_builder_field<F>(
    opts: podman::opts::ContainerCommitOptsBuilder,
    field: &str,
    op: F,
) -> podman::opts::ContainerCommitOptsBuilder
where
    F: FnOnce(
        podman::opts::ContainerCommitOptsBuilder,
        &str,
    ) -> podman::opts::ContainerCommitOptsBuilder,
{
    if field.is_empty() {
        opts
    } else {
        op(opts, field)
    }
}
