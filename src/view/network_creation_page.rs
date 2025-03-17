use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
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
use crate::widget;

const ACTION_CREATE: &str = "network-creation-page.create";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::NetworkCreationPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/network_creation_page.ui")]
    pub(crate) struct NetworkCreationPage {
        #[property(get, set, construct_only, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[property(get, set, construct_only, nullable)]
        pub(super) infra_image: glib::WeakRef<model::Image>,
        #[property(get, set, construct_only)]
        pub(super) show_view_artifact: Cell<bool>,
        #[template_child]
        pub(super) navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub(super) create_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) name_entry_row: TemplateChild<widget::RandomNameEntryRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NetworkCreationPage {
        const NAME: &'static str = "PdsNetworkCreationPage";
        type Type = super::NetworkCreationPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action(ACTION_CREATE, None, |widget, _, _| {
                widget.finish();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for NetworkCreationPage {
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
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for NetworkCreationPage {
        fn root(&self) {
            self.parent_root();

            let widget = &*self.obj();

            glib::idle_add_local(clone!(
                #[weak]
                widget,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    widget.imp().name_entry_row.grab_focus();
                    glib::ControlFlow::Break
                }
            ));
            utils::root(widget).set_default_widget(Some(&*self.create_button));
        }

        fn unroot(&self) {
            utils::root(&*self.obj()).set_default_widget(gtk::Widget::NONE);
            self.parent_unroot()
        }
    }

    #[gtk::template_callbacks]
    impl NetworkCreationPage {
        #[template_callback]
        fn on_name_entry_row_changed(&self) {
            self.obj().on_name_changed();
        }
    }
}

glib::wrapper! {
    pub(crate) struct NetworkCreationPage(ObjectSubclass<imp::NetworkCreationPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Client> for NetworkCreationPage {
    fn from(client: &model::Client) -> Self {
        Self::new(client, true)
    }
}

impl NetworkCreationPage {
    pub(crate) fn new(client: &model::Client, show_view_artifact: bool) -> Self {
        glib::Object::builder()
            .property("client", client)
            .property("show-view-artifact", show_view_artifact)
            .build()
    }

    fn on_name_changed(&self) {
        self.action_set_enabled(ACTION_CREATE, self.imp().name_entry_row.text().len() > 0);
    }

    fn finish(&self) {
        self.create();
    }

    fn create(&self) {
        let imp = self.imp();

        let opts = self.opts();

        let page = view::ActionPage::new(
            &self
                .client()
                .unwrap()
                .action_list()
                .create_network(imp.name_entry_row.text().as_str(), opts.build()),
            self.show_view_artifact(),
        );

        imp.navigation_view.push(
            &adw::NavigationPage::builder()
                .can_pop(false)
                .child(&page)
                .build(),
        );
    }

    fn opts(&self) -> podman::opts::NetworkCreateOptsBuilder {
        let imp = self.imp();

        let mut opts = podman::opts::NetworkCreateOpts::builder()
            .name(imp.name_entry_row.text().as_str())
            // .hostname(imp.hostname_entry_row.text().as_str())
            // .labels(
            //     imp.labels()
            //         .iter::<model::KeyVal>()
            //         .map(Result::unwrap)
            //         .map(|entry| (entry.key(), entry.value())),
            // )
            // .portmappings(
            //     imp.port_mappings()
            //         .iter::<model::PortMapping>()
            //         .map(Result::unwrap)
            //         .map(|port_mapping| podman::models::PortMapping {
            //             container_port: Some(port_mapping.container_port() as u16),
            //             host_ip: None,
            //             host_port: Some(port_mapping.host_port() as u16),
            //             protocol: Some(port_mapping.protocol().to_string()),
            //             range: None,
            //         }),
            // );
            ;
        opts
    }
}

fn bind_model<F>(
    list_box: &gtk::ListBox,
    model: &gio::ListStore,
    widget_func: F,
    action_name: &str,
    label: &str,
) where
    F: Fn(&glib::Object) -> gtk::Widget + 'static,
{
    list_box.bind_model(Some(model), widget_func);
    list_box.append(
        &gtk::ListBoxRow::builder()
            .action_name(action_name)
            .selectable(false)
            .child(
                &gtk::Label::builder()
                    .label(label)
                    .margin_top(12)
                    .margin_bottom(12)
                    .build(),
            )
            .build(),
    );
}

fn add_key_val(model: &gio::ListStore) {
    let entry = model::KeyVal::default();

    entry.connect_remove_request(clone!(
        #[weak]
        model,
        move |entry| {
            if let Some(pos) = model.find(entry) {
                model.remove(pos);
            }
        }
    ));

    model.append(&entry);
}

fn add_value(model: &gio::ListStore) {
    let value = model::Value::default();

    value.connect_remove_request(clone!(
        #[weak]
        model,
        move |value| {
            if let Some(pos) = model.find(value) {
                model.remove(pos);
            }
        }
    ));

    model.append(&value);
}

fn add_port_mapping(model: &gio::ListStore) -> model::PortMapping {
    let port_mapping = model::PortMapping::default();

    port_mapping.connect_remove_request(clone!(
        #[weak]
        model,
        move |port_mapping| {
            if let Some(pos) = model.find(port_mapping) {
                model.remove(pos);
            }
        }
    ));

    model.append(&port_mapping);

    port_mapping
}

fn add_device(model: &gio::ListStore) {
    let device = model::Device::default();

    device.connect_remove_request(clone!(
        #[weak]
        model,
        move |device| {
            if let Some(pos) = model.find(device) {
                model.remove(pos);
            }
        }
    ));

    model.append(&device);
}
