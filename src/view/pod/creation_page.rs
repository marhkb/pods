use std::cell::RefCell;

use adw::subclass::prelude::*;
use adw::traits::BinExt;
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
use crate::utils::ToTypedListModel;
use crate::view;

const ACTION_CREATE: &str = "pod-creation-page.create";

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/pod/creation-page.ui")]
    pub(crate) struct CreationPage {
        pub(super) client: WeakRef<model::Client>,
        pub(super) labels: RefCell<gio::ListStore>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) preferences_page: TemplateChild<adw::PreferencesPage>,
        #[template_child]
        pub(super) name_entry_row: TemplateChild<view::RandomNameEntryRow>,
        #[template_child]
        pub(super) hostname_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) labels_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) create_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) pod_details_page_bin: TemplateChild<adw::Bin>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CreationPage {
        const NAME: &'static str = "PdsPodCreationPage";
        type Type = super::CreationPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action(ACTION_CREATE, None, |widget, _, _| {
                widget.create();
            });

            klass.install_action("pod.add-label", None, |widget, _, _| {
                widget.add_label();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CreationPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "client",
                    "Client",
                    "The client of this pod creation page",
                    model::Client::static_type(),
                    glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "client" => self.client.set(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "client" => obj.client().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            self.name_entry_row
                .connect_text_notify(clone!(@weak obj => move |_| obj.on_name_changed()));

            self.labels_list_box
                .bind_model(Some(&*self.labels.borrow()), |item| {
                    view::KeyValRow::from(item.downcast_ref::<model::KeyVal>().unwrap()).upcast()
                });
            self.labels_list_box.append(
                &gtk::ListBoxRow::builder()
                    .action_name("pod.add-label")
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

        fn dispose(&self, obj: &Self::Type) {
            utils::ChildIter::from(obj).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for CreationPage {
        fn root(&self, widget: &Self::Type) {
            self.parent_root(widget);

            glib::idle_add_local(
                clone!(@weak widget => @default-return glib::Continue(false), move || {
                    widget.imp().name_entry_row.grab_focus();
                    glib::Continue(false)
                }),
            );
            utils::root(widget).set_default_widget(Some(&*self.create_button));
        }

        fn unroot(&self, widget: &Self::Type) {
            utils::root(widget).set_default_widget(gtk::Widget::NONE);
            self.parent_unroot(widget)
        }
    }
}

glib::wrapper! {
    pub(crate) struct CreationPage(ObjectSubclass<imp::CreationPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<Option<&model::Client>> for CreationPage {
    fn from(client: Option<&model::Client>) -> Self {
        glib::Object::new(&[("client", &client)]).expect("Failed to create PdsPodCreationPage")
    }
}

impl CreationPage {
    fn client(&self) -> Option<model::Client> {
        self.imp().client.upgrade()
    }

    fn on_name_changed(&self) {
        self.action_set_enabled(ACTION_CREATE, self.imp().name_entry_row.text().len() > 0);
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

    fn create(&self) {
        self.action_set_enabled(ACTION_CREATE, false);

        let imp = self.imp();
        imp.preferences_page.set_sensitive(false);

        let opts = podman::opts::PodCreateOpts::builder()
            .name(imp.name_entry_row.text().as_str())
            .hostname(imp.hostname_entry_row.text().as_str())
            .labels(
                imp.labels
                    .borrow()
                    .to_owned()
                    .to_typed_list_model::<model::KeyVal>()
                    .into_iter()
                    .map(|label| (label.key(), label.value())),
            )
            .build();

        utils::do_async(
            {
                let podman = self.client().unwrap().podman().clone();
                async move { podman.pods().create(&opts).await }
            },
            clone!(@weak self as obj => move |result| {
                match result.map(|pod| pod.id().to_string()) {
                    Ok(id) => {
                        let client = obj.client().unwrap();
                        match client.pod_list().get_pod(&id) {
                            Some(pod) => obj.switch_to_pod(&pod),
                            None => {
                                client.pod_list().connect_pod_added(
                                    clone!(@weak obj, @strong id => move |_, pod| {
                                        if pod.id() == id.as_str() {
                                            obj.switch_to_pod(pod);
                                        }
                                    }),
                                );
                            }
                        }
                    }
                    Err(e) => {
                        log::error!("Error while creating pod: {}", e);
                        utils::show_error_toast(
                            &obj,
                            "Error while creating pod",
                            &e.to_string()
                        );

                        obj.action_set_enabled(ACTION_CREATE, true);
                        obj.imp().preferences_page.set_sensitive(true);
                    }
                }
            }),
        );
    }

    fn switch_to_pod(&self, pod: &model::Pod) {
        let imp = self.imp();
        imp.pod_details_page_bin
            .set_child(Some(&view::PodDetailsPage::from(pod)));
        imp.stack.set_visible_child(&*imp.pod_details_page_bin);
    }
}
