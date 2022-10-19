use std::cell::RefCell;

use adw::traits::ActionRowExt;
use ashpd::desktop::file_chooser::OpenFileRequest;
use ashpd::WindowIdentifier;
use gettextrs::gettext;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::podman;
use crate::utils;
use crate::utils::ToTypedListModel;
use crate::view;

const ACTION_BUILD: &str = "image-build-page.build-image";
const ACTION_SELECT_CONTEXT_DIR: &str = "image-build-page.select-context-dir";
const ACTION_ADD_LABEL: &str = "image-build-page.add-label";
const GSETTINGS_KEY_LAST_USED_CONTAINER_FILE_PATH: &str = "last-used-container-file-path";

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/image/build-page.ui")]
    pub(crate) struct BuildPage {
        pub(super) settings: utils::PodsSettings,
        pub(super) client: glib::WeakRef<model::Client>,
        pub(super) labels: RefCell<gio::ListStore>,
        #[template_child]
        pub(super) tag_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) context_dir_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) container_file_path_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) labels_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) build_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) leaflet_overlay: TemplateChild<view::LeafletOverlay>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BuildPage {
        const NAME: &'static str = "PdsImageBuildPage";
        type Type = super::BuildPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action(ACTION_BUILD, None, move |widget, _, _| {
                widget.build();
            });

            klass.install_action(ACTION_SELECT_CONTEXT_DIR, None, move |widget, _, _| {
                widget.choose_context_dir();
            });

            klass.install_action(ACTION_ADD_LABEL, None, |widget, _, _| {
                widget.add_label();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BuildPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::builder::<model::Client>("client")
                    .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY)
                    .build()]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "client" => self.client.set(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "client" => self.instance().client().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.instance();

            self.container_file_path_entry_row.set_text(
                &self
                    .settings
                    .string(GSETTINGS_KEY_LAST_USED_CONTAINER_FILE_PATH),
            );

            obj.on_opts_changed();
            self.tag_entry_row
                .connect_text_notify(clone!(@weak obj => move |_| obj.on_opts_changed()));
            self.context_dir_row
                .connect_subtitle_notify(clone!(@weak obj => move |_| obj.on_opts_changed()));

            self.labels_list_box
                .bind_model(Some(&*self.labels.borrow()), |item| {
                    view::KeyValRow::from(item.downcast_ref::<model::KeyVal>().unwrap()).upcast()
                });
            self.labels_list_box.append(
                &gtk::ListBoxRow::builder()
                    .action_name(ACTION_ADD_LABEL)
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
            utils::ChildIter::from(&*self.instance()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for BuildPage {
        fn root(&self) {
            self.parent_root();

            let widget = &*self.instance();

            glib::idle_add_local(
                clone!(@weak widget => @default-return glib::Continue(false), move || {
                    widget.imp().tag_entry_row.grab_focus();
                    glib::Continue(false)
                }),
            );
            utils::root(widget).set_default_widget(Some(&*self.build_button));
        }

        fn unroot(&self) {
            utils::root(&*self.instance()).set_default_widget(gtk::Widget::NONE);
            self.parent_unroot()
        }
    }
}

glib::wrapper! {
    pub(crate) struct BuildPage(ObjectSubclass<imp::BuildPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<Option<&model::Client>> for BuildPage {
    fn from(client: Option<&model::Client>) -> Self {
        glib::Object::new::<Self>(&[("client", &client)])
    }
}

impl BuildPage {
    fn client(&self) -> Option<model::Client> {
        self.imp().client.upgrade()
    }

    fn on_opts_changed(&self) {
        let imp = self.imp();

        let enabled = imp.tag_entry_row.text().len() > 0
            && imp
                .context_dir_row
                .subtitle()
                .map(|s| !s.is_empty())
                .unwrap_or(false);

        self.action_set_enabled(ACTION_BUILD, enabled);
    }

    fn choose_context_dir(&self) {
        self.open_file_chooser_dialog(
            &gettext("Select Build Context Directory"),
            true,
            clone!(@weak self as obj => move |file| {
                obj.imp().context_dir_row.set_subtitle(file);
            }),
        );
    }

    fn open_file_chooser_dialog<F>(&self, title: &str, directory: bool, op: F)
    where
        F: FnOnce(&str) + 'static,
    {
        glib::MainContext::default().block_on(async move {
            let request = OpenFileRequest::default()
                .identifier(WindowIdentifier::from_native(&self.native().unwrap()).await)
                .title(title)
                .directory(directory)
                .modal(true);

            if let Ok(files) = request.build().await {
                let file = gio::File::for_uri(files.uris()[0].as_str());

                if let Some(path) = file.path() {
                    op(path.to_str().unwrap());
                }
            }
        });
    }

    fn add_label(&self) {
        let label = model::KeyVal::default();
        self.connect_label(&label);

        self.imp().labels.borrow().append(&label);
    }

    fn connect_label(&self, label: &model::KeyVal) {
        label.connect_remove_request(clone!(@weak self as obj => move |label| {
            let imp = obj.imp();

            let labels = imp.labels.borrow();
            if let Some(pos) = labels.find(label) {
                labels.remove(pos);
            }
        }));
    }

    fn build(&self) {
        let imp = self.imp();

        if imp.tag_entry_row.text().contains(char::is_uppercase) {
            utils::show_toast(
                self,
                &gettext("Image name should not contain uppercase characters."),
            );
            return;
        }

        if !imp.tag_entry_row.text().is_empty() {
            if let Some(context_dir_row) = imp.context_dir_row.subtitle() {
                let opts = podman::opts::ImageBuildOptsBuilder::new(context_dir_row)
                    .dockerfile(imp.container_file_path_entry_row.text())
                    .tag(imp.tag_entry_row.text())
                    .labels(
                        imp.labels
                            .borrow()
                            .to_owned()
                            .to_typed_list_model::<model::KeyVal>()
                            .into_iter()
                            .map(|label| (label.key(), label.value())),
                    )
                    .build();

                let page = view::ActionPage::from(
                    &self
                        .client()
                        .unwrap()
                        .action_list()
                        .build_image(imp.tag_entry_row.text().as_str(), opts),
                );

                imp.leaflet_overlay.show_details(&page);

                if let Err(e) = imp.settings.set_string(
                    GSETTINGS_KEY_LAST_USED_CONTAINER_FILE_PATH,
                    imp.container_file_path_entry_row.text().as_str(),
                ) {
                    log::warn!(
                        "Error on saving gsettings '{}': {}",
                        GSETTINGS_KEY_LAST_USED_CONTAINER_FILE_PATH,
                        e
                    );
                }
            }
        }
    }
}
