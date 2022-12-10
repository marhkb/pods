use std::cell::Cell;

use adw::subclass::prelude::*;
use adw::traits::ActionRowExt;
use ashpd::desktop::file_chooser::OpenFileRequest;
use ashpd::WindowIdentifier;
use gettextrs::gettext;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_SELECT_HOST_FILE: &str = "container-files-put-page.select-host-file";

const ACTION_SELECT_HOST_DIRECTORY: &str = "container-files-put-page.select-host-directory";
const ACTION_PUT: &str = "container-files-put-page.put";

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/container/files-put-page.ui")]
    pub(crate) struct FilesPutPage {
        pub(super) container: WeakRef<model::Container>,
        pub(super) directory: Cell<bool>,
        #[template_child]
        pub(super) host_path_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) container_path_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) put_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) leaflet_overlay: TemplateChild<view::LeafletOverlay>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FilesPutPage {
        const NAME: &'static str = "PdsContainerFilesPutPage";
        type Type = super::FilesPutPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

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
            klass.install_action_async(ACTION_PUT, None, |widget, _, _| async move {
                widget.put().await;
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for FilesPutPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::Container>("container")
                        .construct()
                        .explicit_notify()
                        .build(),
                ]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "container" => self.obj().set_container(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "container" => self.obj().container().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.action_set_enabled(ACTION_PUT, false);
            self.host_path_row
                .connect_subtitle_notify(clone!(@weak obj => move |row| {
                    obj.action_set_enabled(ACTION_PUT, row.subtitle().is_some());
                }));
        }

        fn dispose(&self) {
            utils::ChildIter::from(&*self.obj()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for FilesPutPage {
        fn root(&self) {
            self.parent_root();

            let widget = &*self.obj();

            glib::idle_add_local(
                clone!(@weak widget => @default-return glib::Continue(false), move || {
                    widget.imp().container_path_row.grab_focus();
                    glib::Continue(false)
                }),
            );
            utils::root(widget).set_default_widget(Some(&*self.put_button));
        }

        fn unroot(&self) {
            utils::root(&*self.obj()).set_default_widget(gtk::Widget::NONE);
            self.parent_unroot()
        }
    }
}

glib::wrapper! {
    pub(crate) struct FilesPutPage(ObjectSubclass<imp::FilesPutPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Container> for FilesPutPage {
    fn from(container: &model::Container) -> Self {
        glib::Object::builder::<Self>()
            .property("container", &container)
            .build()
    }
}

impl FilesPutPage {
    fn container(&self) -> Option<model::Container> {
        self.imp().container.upgrade()
    }

    fn set_container(&self, value: Option<&model::Container>) {
        if self.container().as_ref() == value {
            return;
        }
        self.imp().container.set(value);
        self.notify("container");
    }

    async fn select_file(&self, directory: bool) {
        let request = OpenFileRequest::default()
            .identifier(WindowIdentifier::from_native(&self.native().unwrap()).await)
            .title(&if directory {
                gettext("Select Host Directory")
            } else {
                gettext("Select Host File")
            })
            .directory(directory)
            .modal(true);

        utils::show_open_file_dialog(request, self, |obj, files| {
            let file = gio::File::for_uri(files.uris()[0].as_str());

            if let Some(path) = file.path() {
                let imp = obj.imp();

                imp.host_path_row.set_subtitle(path.to_str().unwrap());
                imp.directory.set(directory);
            }
        })
        .await;
    }

    async fn put(&self) {
        if let Some(container) = self.container() {
            let imp = self.imp();

            let host_path = imp
                .host_path_row
                .subtitle()
                .unwrap_or_else(|| glib::GString::from("/"));
            let container_path = imp.container_path_row.text();

            imp.leaflet_overlay.show_details(&view::ActionPage::from(
                &container
                    .container_list()
                    .unwrap()
                    .client()
                    .unwrap()
                    .action_list()
                    .copy_files_into_container(
                        host_path,
                        if container_path.is_empty() {
                            String::from("/")
                        } else {
                            String::from(container_path)
                        },
                        imp.directory.get(),
                        &container,
                    ),
            ))
        }
    }
}
