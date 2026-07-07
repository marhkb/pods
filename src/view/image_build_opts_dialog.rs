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

use crate::engine;
use crate::model;
use crate::utils;
use crate::view;

const ACTION_BUILD: &str = "image-build-opts-dialog.build";
const ACTION_SELECT_CONTEXT_DIR: &str = "image-build-opts-dialog.select-context-dir";
const ACTION_ADD_LABEL: &str = "image-build-opts-dialog.add-label";

mod imp {
    use super::*;

    #[derive(Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImageBuildOptsDialog)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/image_build_opts_dialog.ui")]
    pub(crate) struct ImageBuildOptsDialog {
        pub(super) labels: OnceCell<gio::ListStore>,

        #[property(get, set, construct_only, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[property(get, set, construct_only)]
        pub(super) opts: OnceCell<model::BoxedImageBuildOpts>,

        #[template_child]
        pub(super) build_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) image_name_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) context_dir_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) container_file_path_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) labels_list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageBuildOptsDialog {
        const NAME: &'static str = "PdsImageBuildOptsDialog";
        type Type = super::ImageBuildOptsDialog;
        type ParentType = adw::Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action(ACTION_BUILD, None, |widget, _, _| {
                widget.close_and_build();
            });

            klass.install_action_async(
                ACTION_SELECT_CONTEXT_DIR,
                None,
                move |widget, _, _| async move {
                    widget.choose_context_dir().await;
                },
            );

            klass.install_action(ACTION_ADD_LABEL, None, |widget, _, _| {
                widget.add_label();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImageBuildOptsDialog {
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

            self.labels_list_box
                .bind_model(Some(self.labels()), |item| {
                    view::KeyValRow::from(item.downcast_ref::<model::KeyVal>().unwrap()).upcast()
                });
            self.labels_list_box.append(
                &gtk::ListBoxRow::builder()
                    .action_name(ACTION_ADD_LABEL)
                    .selectable(false)
                    .child(
                        &gtk::Label::builder()
                            .label(gettext("Add Label"))
                            .margin_top(12)
                            .margin_bottom(12)
                            .build(),
                    )
                    .build(),
            );

            let opts = obj.opts();

            self.image_name_entry_row.set_text(&opts.tag);
            self.context_dir_row.set_subtitle(&opts.path);
            opts.labels.iter().for_each(|(key, value)| {
                let label = obj.add_label();
                label.set_key(key.to_owned());
                label.set_value(value.to_owned());
            });

            self.on_opts_changed();
        }
    }

    impl WidgetImpl for ImageBuildOptsDialog {
        fn map(&self) {
            self.parent_map();
            self.image_name_entry_row.grab_focus();
        }
    }

    impl AdwDialogImpl for ImageBuildOptsDialog {}

    #[gtk::template_callbacks]
    impl ImageBuildOptsDialog {
        #[template_callback]
        fn on_opts_changed(&self) {
            let enabled: bool = !self.image_name_entry_row.text().trim().is_empty()
                && self
                    .context_dir_row
                    .subtitle()
                    .map(|s| !s.is_empty())
                    .unwrap_or(false);

            self.obj().action_set_enabled(ACTION_BUILD, enabled);
        }

        pub(super) fn labels(&self) -> &gio::ListStore {
            self.labels
                .get_or_init(gio::ListStore::new::<model::KeyVal>)
        }
    }
}

glib::wrapper! {
    pub(crate) struct ImageBuildOptsDialog(ObjectSubclass<imp::ImageBuildOptsDialog>)
        @extends gtk::Widget, adw::Dialog,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::ShortcutManager;
}

impl ImageBuildOptsDialog {
    pub(crate) fn new(client: &model::Client, opts: Option<model::BoxedImageBuildOpts>) -> Self {
        glib::Object::builder()
            .property("client", client)
            .property("opts", opts.unwrap_or_default())
            .build()
    }

    pub(crate) fn add_label(&self) -> model::KeyVal {
        let label = model::KeyVal::default();

        label.connect_remove_request(clone!(
            #[weak(rename_to = obj)]
            self,
            move |label| {
                let labels = obj.imp().labels();
                if let Some(pos) = labels.find(label) {
                    labels.remove(pos);
                }
            }
        ));

        self.imp().labels().append(&label);

        label
    }

    async fn choose_context_dir(&self) {
        let request = OpenFileRequest::default()
            .identifier(WindowIdentifier::from_native(&self.native().unwrap()).await)
            .title(gettext("Select Build Context Directory").as_str())
            .directory(true)
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
                        obj.imp()
                            .context_dir_row
                            .set_subtitle(path.to_str().unwrap());
                    }
                }
            ),
        )
        .await;
    }

    fn close_and_build(&self) {
        self.close();

        let Some(action_list) = self.client().map(|client| client.action_list()) else {
            return;
        };

        let imp = self.imp();

        let Some(path) = imp.context_dir_row.subtitle() else {
            return;
        };

        let opts = engine::opts::ImageBuildOpts {
            dockerfile: imp.container_file_path_entry_row.text().into(),
            labels: imp
                .labels()
                .iter::<model::KeyVal>()
                .map(Result::unwrap)
                .map(|entry| (entry.key(), entry.value()))
                .collect(),
            path: path.into(),
            tag: imp.image_name_entry_row.text().into(),
        };

        view::ActionDialog::from(&action_list.build_image(opts)).present(Some(self));
    }
}
