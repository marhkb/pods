use std::cell::Cell;
use std::cell::OnceCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use ashpd::WindowIdentifier;
use ashpd::desktop::file_chooser::OpenFileRequest;
use gettextrs::gettext;
use glib::Properties;
use glib::clone;
use gtk::CompositeTemplate;
use gtk::gio;
use gtk::glib;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_COPY: &str = "container-copy-to-opts-dialog.copy";
const ACTION_SELECT_HOST_FILE: &str = "container-copy-to-opts-dialog.select-host-file";
const ACTION_SELECT_HOST_DIRECTORY: &str = "container-copy-to-opts-dialog.select-host-directory";

mod imp {
    use super::*;

    #[derive(Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainerCopyToOptsDialog)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/container_copy_to_opts_dialog.ui")]
    pub(crate) struct ContainerCopyToOptsDialog {
        #[property(get, set, construct_only)]
        pub(super) container: glib::WeakRef<model::Container>,

        #[property(get, set)]
        pub(super) directory: Cell<bool>,
        #[property(get, set, construct_only)]
        pub(super) container_path: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub(super) host_path: OnceCell<String>,

        #[template_child]
        pub(super) host_path_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) container_path_row: TemplateChild<adw::EntryRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerCopyToOptsDialog {
        const NAME: &'static str = "PdsContainerCopyToOptsDialog";
        type Type = super::ContainerCopyToOptsDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action(ACTION_COPY, None, |widget, _, _| {
                widget.close_and_copy();
            });

            klass.install_action_async(ACTION_SELECT_HOST_FILE, None, |widget, _, _| async move {
                widget.select_file(false).await;
            });
            klass.install_action_async(
                ACTION_SELECT_HOST_DIRECTORY,
                None,
                |widget, _, _| async move {
                    widget.select_file(true).await;
                },
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerCopyToOptsDialog {
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

            self.host_path_row.set_subtitle(&obj.host_path());
            self.container_path_row.set_text(&obj.container_path());

            self.on_host_path_row_changed();
        }
    }

    impl WidgetImpl for ContainerCopyToOptsDialog {
        fn map(&self) {
            self.parent_map();
            self.container_path_row.grab_focus();
        }
    }

    impl AdwDialogImpl for ContainerCopyToOptsDialog {}

    #[gtk::template_callbacks]
    impl ContainerCopyToOptsDialog {
        #[template_callback]
        fn on_host_path_row_changed(&self) {
            let enabled: bool = self
                .host_path_row
                .subtitle()
                .map(|s| !s.is_empty())
                .unwrap_or(false);

            self.obj().action_set_enabled(ACTION_COPY, enabled);
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainerCopyToOptsDialog(ObjectSubclass<imp::ContainerCopyToOptsDialog>)
        @extends gtk::Widget, adw::Dialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl ContainerCopyToOptsDialog {
    pub(crate) fn new(
        container: &model::Container,
        directory: bool,
        host_path: &str,
        container_path: &str,
    ) -> Self {
        glib::Object::builder()
            .property("container", container)
            .property("directory", directory)
            .property("host-path", host_path)
            .property("container-path", container_path)
            .build()
    }

    async fn select_file(&self, directory: bool) {
        let request = OpenFileRequest::default()
            .identifier(WindowIdentifier::from_native(&self.native().unwrap()).await)
            .title(
                if directory {
                    gettext("Select Host Directory")
                } else {
                    gettext("Select Host File")
                }
                .as_str(),
            )
            .directory(directory)
            .modal(true);

        utils::show_open_file_dialog(
            request,
            self,
            clone!(
                #[weak(rename_to = obj)]
                self,
                move |files| {
                    let file = gio::File::for_uri(files.uris()[0].as_str());

                    if let Some(path) = file.path() {
                        let imp = obj.imp();

                        imp.host_path_row.set_subtitle(path.to_str().unwrap());
                        obj.set_directory(directory);
                    }
                }
            ),
        )
        .await;
    }

    fn close_and_copy(&self) {
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

        let imp = self.imp();

        let Some(host_path) = imp.host_path_row.subtitle() else {
            return;
        };

        let container_path = imp.container_path_row.text();
        let container_path = container_path.trim();
        let container_path = if container_path.is_empty() {
            "/"
        } else {
            container_path
        };

        view::ActionDialog::from(&action_list.copy_to_container(
            &container,
            self.directory(),
            host_path.as_str(),
            container_path,
        ))
        .present(Some(self));
    }
}
