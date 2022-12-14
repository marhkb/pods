use adw::subclass::prelude::*;
use adw::traits::ActionRowExt;
use adw::traits::BinExt;
use ashpd::desktop::file_chooser::FileFilter;
use ashpd::desktop::file_chooser::SaveFileRequest;
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

const ACTION_SELECT_HOST_PATH: &str = "container-files-get-page.select-host-path";
const ACTION_GET: &str = "container-files-get-page.get";

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/container/files-get-page.ui")]
    pub(crate) struct FilesGetPage {
        pub(super) container: WeakRef<model::Container>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) get_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) container_path_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) host_path_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) action_page_bin: TemplateChild<adw::Bin>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FilesGetPage {
        const NAME: &'static str = "PdsContainerFilesGetPage";
        type Type = super::FilesGetPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action_async(ACTION_SELECT_HOST_PATH, None, |widget, _, _| async move {
                widget.select_path().await;
            });
            klass.install_action_async(ACTION_GET, None, |widget, _, _| async move {
                widget.get().await;
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for FilesGetPage {
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

            obj.action_set_enabled(ACTION_GET, false);
            self.host_path_row
                .connect_subtitle_notify(clone!(@weak obj => move |row| {
                    obj.action_set_enabled(ACTION_GET, row.subtitle().is_some());
                }));
        }

        fn dispose(&self) {
            utils::ChildIter::from(&*self.obj()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for FilesGetPage {
        fn root(&self) {
            self.parent_root();

            let widget = &*self.obj();

            glib::idle_add_local(
                clone!(@weak widget => @default-return glib::Continue(false), move || {
                    widget.imp().container_path_row.grab_focus();
                    glib::Continue(false)
                }),
            );
            utils::root(widget).set_default_widget(Some(&*self.get_button));
        }

        fn unroot(&self) {
            utils::root(&*self.obj()).set_default_widget(gtk::Widget::NONE);
            self.parent_unroot()
        }
    }
}

glib::wrapper! {
    pub(crate) struct FilesGetPage(ObjectSubclass<imp::FilesGetPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Container> for FilesGetPage {
    fn from(container: &model::Container) -> Self {
        glib::Object::builder::<Self>()
            .property("container", &container)
            .build()
    }
}

impl FilesGetPage {
    fn container(&self) -> Option<model::Container> {
        self.imp().container.upgrade()
    }

    fn set_container(&self, value: Option<&model::Container>) {
        if self.container().as_ref() == value {
            return;
        }

        if let Some(container) = value {
            container.connect_deleted(clone!(@weak self as obj => move |_| {
                obj.activate_action("action.cancel", None).unwrap();
            }));
        }

        self.imp().container.set(value);
        self.notify("container");
    }

    async fn select_path(&self) {
        let request = SaveFileRequest::default()
            .identifier(WindowIdentifier::from_native(&self.native().unwrap()).await)
            .title(&gettext("Select Host Destination Path"))
            .filter(FileFilter::new("Tar Archive").mimetype("application/x-tar"))
            .modal(true);

        utils::show_save_file_dialog(request, self, |obj, files| {
            let file = gio::File::for_uri(files.uris()[0].as_str());

            if let Some(path) = file.path() {
                obj.imp().host_path_row.set_subtitle(path.to_str().unwrap());
            }
        })
        .await;
    }

    async fn get(&self) {
        if let Some(container) = self.container() {
            let imp = self.imp();

            let host_path = imp
                .host_path_row
                .subtitle()
                .unwrap_or_else(|| glib::GString::from("/"));
            let container_path = imp.container_path_row.text();

            let page = view::ActionPage::from(
                &container
                    .container_list()
                    .unwrap()
                    .client()
                    .unwrap()
                    .action_list()
                    .copy_files_from_container(
                        &container,
                        if container_path.is_empty() {
                            String::from("/")
                        } else {
                            String::from(container_path)
                        },
                        host_path,
                    ),
            );

            imp.action_page_bin.set_child(Some(&page));
            imp.stack.set_visible_child(&*imp.action_page_bin);
        }
    }
}
