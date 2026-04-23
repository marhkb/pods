use std::cell::OnceCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use ashpd::WindowIdentifier;
use ashpd::desktop::file_chooser::FileFilter;
use ashpd::desktop::file_chooser::SaveFileRequest;
use gettextrs::gettext;
use glib::Properties;
use glib::clone;
use gtk::CompositeTemplate;
use gtk::gio;
use gtk::glib;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_COPY: &str = "container-copy-from-opts-dialog.copy";
const ACTION_SELECT_HOST_PATH: &str = "container-copy-from-opts-dialog.select-host-path";

mod imp {
    use std::i32;

    use super::*;

    #[derive(Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainerCopyFromOptsDialog)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/container_copy_from_opts_dialog.ui")]
    pub(crate) struct ContainerCopyFromOptsDialog {
        #[property(get, set, construct_only)]
        pub(super) container: glib::WeakRef<model::Container>,
        #[property(get, set, construct_only)]
        pub(super) container_path: OnceCell<String>,
        #[property(get, set, construct_only)]
        pub(super) host_path: OnceCell<String>,

        #[template_child]
        pub(super) container_path_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) host_path_row: TemplateChild<adw::ActionRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerCopyFromOptsDialog {
        const NAME: &'static str = "PdsContainerCopyFromOptsDialog";
        type Type = super::ContainerCopyFromOptsDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action(ACTION_COPY, None, |widget, _, _| {
                widget.close_and_copy();
            });
            klass.install_action_async(ACTION_SELECT_HOST_PATH, None, async |widget, _, _| {
                widget.select_path().await;
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerCopyFromOptsDialog {
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

            self.container_path_row.set_text(&obj.container_path());
            self.host_path_row.set_subtitle(&obj.host_path());

            self.on_opts_changed();
        }
    }

    impl WidgetImpl for ContainerCopyFromOptsDialog {
        fn map(&self) {
            self.parent_map();
            self.container_path_row.grab_focus();
        }
    }

    impl AdwDialogImpl for ContainerCopyFromOptsDialog {}

    #[gtk::template_callbacks]
    impl ContainerCopyFromOptsDialog {
        #[template_callback]
        fn on_opts_changed(&self) {
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
    pub(crate) struct ContainerCopyFromOptsDialog(ObjectSubclass<imp::ContainerCopyFromOptsDialog>)
        @extends gtk::Widget, adw::Dialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl ContainerCopyFromOptsDialog {
    pub(crate) fn new(container: &model::Container, container_path: &str, host_path: &str) -> Self {
        glib::Object::builder()
            .property("container", container)
            .property("container-path", container_path)
            .property("host-path", host_path)
            .build()
    }

    async fn select_path(&self) {
        let container_name = self
            .container()
            .map(|container| container.name().to_string())
            .unwrap_or_else(|| String::from("container"));

        let suggested_archive_name = glib::DateTime::now_local()
            .and_then(|now| now.format_iso8601())
            .map(|date| format!("{container_name}-{date}.tar"))
            .unwrap_or_else(|_| format!("{container_name}.tar"));

        let request = SaveFileRequest::default()
            .identifier(WindowIdentifier::from_native(&self.native().unwrap()).await)
            .title(gettext("Host Path").as_str())
            .current_name(suggested_archive_name.as_str())
            .filter(FileFilter::new("Tar Archive").mimetype("application/x-tar"))
            .modal(true);

        utils::show_save_file_dialog(
            request,
            self,
            clone!(
                #[weak(rename_to = obj)]
                self,
                move |files| {
                    let file = gio::File::for_uri(files.uris()[0].as_str());

                    if let Some(path) = file.path() {
                        obj.imp().host_path_row.set_subtitle(path.to_str().unwrap());
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
            .map(|client| client.action_list2())
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

        view::ActionDialog::from(&action_list.copy_from_container(
            &container,
            container_path,
            host_path.as_str(),
        ))
        .present(Some(self));
    }
}
