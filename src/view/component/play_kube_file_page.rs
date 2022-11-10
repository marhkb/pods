use std::cell::RefCell;

use adw::subclass::prelude::*;
use adw::traits::ActionRowExt;
use ashpd::desktop::file_chooser::FileFilter;
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
use crate::podman;
use crate::utils;
use crate::view;

const ACTION_SELECT_HOST_PATH: &str = "play-kube-file-page.select-file";
const ACTION_ADD_STATIC_IP: &str = "play-kube-file-page.add-static-ip";
const ACTION_ADD_STATIC_MAC: &str = "play-kube-file-page.add-static-mac";
const ACTION_PLAY_KUBE: &str = "play-kube-file-page.play-kube";

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/component/play-kube-file-page.ui")]
    pub(crate) struct PlayKubeFilePage {
        pub(super) client: WeakRef<model::Client>,
        pub(super) static_ips: gio::ListStore,
        pub(super) static_macs: gio::ListStore,
        #[template_child]
        pub(super) kube_file_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) log_driver_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) start_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) tls_verify_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) static_ips_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) static_macs_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) play_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) leaflet_overlay: TemplateChild<view::LeafletOverlay>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PlayKubeFilePage {
        const NAME: &'static str = "PdsPlayKubeFilePage";
        type Type = super::PlayKubeFilePage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action_async(ACTION_SELECT_HOST_PATH, None, |widget, _, _| async move {
                widget.select_path().await;
            });
            klass.install_action(ACTION_ADD_STATIC_IP, None, |widget, _, _| {
                widget.add_static_ip();
            });
            klass.install_action(ACTION_ADD_STATIC_MAC, None, |widget, _, _| {
                widget.add_static_mac();
            });
            klass.install_action(ACTION_PLAY_KUBE, None, |widget, _, _| async move {
                widget.play_kube();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PlayKubeFilePage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::builder::<model::Client>("client")
                    .flags(
                        glib::ParamFlags::READWRITE
                            | glib::ParamFlags::CONSTRUCT
                            | glib::ParamFlags::EXPLICIT_NOTIFY,
                    )
                    .build()]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "client" => self.obj().set_client(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "client" => self.obj().client().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            bind_model(
                &self.static_ips_list_box,
                &self.static_ips,
                gettext("IP"),
                ACTION_ADD_STATIC_IP,
            );

            bind_model(
                &self.static_macs_list_box,
                &self.static_macs,
                gettext("MAC"),
                ACTION_ADD_STATIC_MAC,
            );

            obj.action_set_enabled(ACTION_PLAY_KUBE, false);
            self.kube_file_row
                .connect_subtitle_notify(clone!(@weak obj => move |row| {
                    obj.action_set_enabled(ACTION_PLAY_KUBE, row.subtitle().is_some());
                }));
        }

        fn dispose(&self) {
            utils::ChildIter::from(&*self.obj()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for PlayKubeFilePage {
        fn root(&self) {
            self.parent_root();

            utils::root(&*self.obj()).set_default_widget(Some(&*self.play_button));
        }

        fn unroot(&self) {
            utils::root(&*self.obj()).set_default_widget(gtk::Widget::NONE);
            self.parent_unroot()
        }
    }
}

glib::wrapper! {
    pub(crate) struct PlayKubeFilePage(ObjectSubclass<imp::PlayKubeFilePage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Client> for PlayKubeFilePage {
    fn from(client: &model::Client) -> Self {
        glib::Object::builder::<Self>()
            .property("client", &client)
            .build()
    }
}

impl PlayKubeFilePage {
    fn client(&self) -> Option<model::Client> {
        self.imp().client.upgrade()
    }

    fn set_client(&self, value: Option<&model::Client>) {
        if self.client().as_ref() == value {
            return;
        }
        self.imp().client.set(value);
        self.notify("client");
    }

    async fn select_path(&self) {
        let request = OpenFileRequest::default()
            .identifier(WindowIdentifier::from_native(&self.native().unwrap()).await)
            .title(&gettext("Select Kube File"))
            .filter(FileFilter::new(&gettext("YAML Files")).mimetype("application/x-yaml"))
            .filter(FileFilter::new(&gettext("All Files")).glob("*"))
            .modal(true);

        utils::show_open_file_dialog(request, self, |obj, files| {
            let file = gio::File::for_uri(files.uris()[0].as_str());

            if let Some(path) = file.path() {
                obj.imp().kube_file_row.set_subtitle(path.to_str().unwrap());
            }
        })
        .await;
    }

    fn add_static_ip(&self) {
        add_value(&self.imp().static_ips);
    }

    fn add_static_mac(&self) {
        add_value(&self.imp().static_macs);
    }

    fn play_kube(&self) {
        if let Some(client) = self.client() {
            let imp = self.imp();

            let opts = podman::opts::PlayKubernetesYamlOpts::builder()
                .start(imp.start_switch.is_active())
                .tls_verify(imp.tls_verify_switch.is_active())
                .static_ips(
                    imp.static_ips
                        .iter::<glib::Object>()
                        .unwrap()
                        .map(|item| item.unwrap().downcast::<model::Value>().unwrap())
                        .map(|value| value.value()),
                )
                .static_macs(
                    imp.static_macs
                        .iter::<glib::Object>()
                        .unwrap()
                        .map(|item| item.unwrap().downcast::<model::Value>().unwrap())
                        .map(|value| value.value()),
                );

            let log_driver = imp.log_driver_entry_row.text();
            let log_driver = log_driver.trim();
            let opts = if !log_driver.is_empty() {
                opts.log_driver(log_driver)
            } else {
                opts
            };

            imp.leaflet_overlay.show_details(&view::ActionPage::from(
                &client.action_list().play_kubernetes_yaml(
                    imp.kube_file_row.subtitle().unwrap(),
                    (**client.podman()).clone(),
                    opts.build(),
                ),
            ))
        }
    }
}

fn bind_model(list_box: &gtk::ListBox, model: &gio::ListStore, title: String, action_name: &str) {
    list_box.bind_model(Some(model), {
        // let title = title.clone();
        move |item| {
            view::ValueRow::new(item.downcast_ref::<model::Value>().unwrap(), &title).upcast()
        }
    });
    list_box.append(
        &gtk::ListBoxRow::builder()
            .action_name(action_name)
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

fn add_value(model: &gio::ListStore) {
    let value = model::Value::default();

    value.connect_remove_request(clone!(@weak model => move |value| {
        if let Some(pos) = model.find(value) {
            model.remove(pos);
        }
    }));

    model.append(&value);
}
