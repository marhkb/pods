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

use crate::engine;
use crate::model;
use crate::rt;
use crate::utils;
use crate::view;

const ACTION_FETCH_USERNAME: &str = "container-commit-opts-dialog.fetch-username";
const ACTION_ADD_CHANGE: &str = "container-commit-opts-dialog.add-change";
const ACTION_COMMIT: &str = "container-commit-opts-dialog.commit";

mod imp {
    use super::*;

    #[derive(Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainerCommitOptsDialog)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/container_commit_opts_dialog.ui")]
    pub(crate) struct ContainerCommitOptsDialog {
        pub(super) changes: OnceCell<gio::ListStore>,

        #[property(get, set = Self::set_container, construct, nullable)]
        pub(super) container: glib::WeakRef<model::Container>,
        #[property(get, set, construct_only)]
        pub(super) opts: OnceCell<model::BoxedContainerCommitOpts>,

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
        pub(super) pause_switch_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) changes_list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerCommitOptsDialog {
        const NAME: &'static str = "PdsContainerCommitOptsDialog";
        type Type = super::ContainerCommitOptsDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action_async(ACTION_FETCH_USERNAME, None, |widget, _, _| async move {
                widget.fetch_user_information().await;
            });
            klass.install_action(ACTION_ADD_CHANGE, None, |widget, _, _| {
                widget.add_change(None);
            });
            klass.install_action(ACTION_COMMIT, None, |widget, _, _| {
                widget.close_and_commit();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerCommitOptsDialog {
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

            let opts = obj.opts();

            self.author_entry_row
                .set_text(opts.author.as_deref().unwrap_or_default());
            self.comment_entry_row
                .set_text(opts.comment.as_deref().unwrap_or_default());
            self.repo_entry_row
                .set_text(opts.repo.as_deref().unwrap_or_default());
            self.tag_entry_row
                .set_text(opts.tag.as_deref().unwrap_or_default());

            match obj
                .container()
                .and_then(|container| container.container_list())
                .and_then(|container_list| container_list.client())
                .and_then(|client| client.engine().capabilities().image_formats())
            {
                Some(format_list) => {
                    self.format_combo_row.set_visible(true);
                    self.format_combo_row.set_model(Some(&format_list));

                    if let Some(ref format) = opts.format {
                        self.format_combo_row.set_selected(format_list.find(format));
                    }
                }
                None => {
                    self.format_combo_row.set_visible(false);
                    self.format_combo_row.set_model(gio::ListModel::NONE);
                }
            }

            self.pause_switch_row.set_active(opts.pause);

            opts.changes.iter().for_each(|change| {
                obj.add_change(Some(change.as_str().into()));
            });
        }
    }

    impl WidgetImpl for ContainerCommitOptsDialog {
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

    impl AdwDialogImpl for ContainerCommitOptsDialog {}

    impl ContainerCommitOptsDialog {
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
    pub(crate) struct ContainerCommitOptsDialog(ObjectSubclass<imp::ContainerCommitOptsDialog>)
        @extends gtk::Widget, adw::Dialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl ContainerCommitOptsDialog {
    pub(crate) fn new(
        container: &model::Container,
        opts: Option<model::BoxedContainerCommitOpts>,
    ) -> Self {
        glib::Object::builder()
            .property("container", container)
            .property("opts", opts.clone().unwrap_or_default())
            .build()
    }

    pub(crate) fn close_and_commit(&self) {
        self.close();

        let Some(container) = self.container() else {
            return;
        };

        let Some(action_list) = container
            .container_list()
            .and_then(|container_list| container_list.client())
            .map(|client| client.action_list())
        else {
            return;
        };

        view::ActionDialog::from(&action_list.commit_container(&container, self.create_opts()))
            .present(Some(self));
    }

    fn create_opts(&self) -> engine::opts::ContainerCommitOpts {
        let imp = self.imp();

        engine::opts::ContainerCommitOpts {
            author: extract_option(&imp.author_entry_row),
            changes: imp
                .changes()
                .iter::<model::Value>()
                .map(Result::unwrap)
                .map(|change| change.value())
                .collect(),
            comment: extract_option(&imp.comment_entry_row),
            format: imp.format_combo_row.selected_item().map(|item| {
                item.downcast_ref::<gtk::StringObject>()
                    .unwrap()
                    .string()
                    .to_string()
            }),
            pause: imp.pause_switch_row.is_active(),
            repo: extract_option(&imp.repo_entry_row),
            tag: extract_option(&imp.tag_entry_row),
        }
    }

    pub(crate) async fn fetch_user_information(&self) {
        let request = UserInformationRequest::default()
            .identifier(WindowIdentifier::from_native(&self.native().unwrap()).await);

        let user_info = rt::Promise::new(async move {
            request
                .send()
                .await
                .and_then(|user_info| user_info.response())
        })
        .exec()
        .await;

        match user_info {
            Ok(user_info) => self.imp().author_entry_row.set_text(user_info.name()),
            Err(e) => {
                if let ashpd::Error::Portal(ashpd::PortalError::Cancelled(_)) = e {
                    utils::show_error_toast(
                        self,
                        &gettext("Error on fetching user name"),
                        &e.to_string(),
                    );
                }
            }
        }
    }

    pub(crate) fn add_change(&self, change: Option<model::Value>) -> model::Value {
        let change = change.unwrap_or_default();

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

        change
    }
}

fn extract_option(row: &adw::EntryRow) -> Option<String>
where
{
    let text = row.text();
    let text = text.trim();
    (!text.is_empty()).then(|| text.to_owned())
}
