use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::clone;
use glib::Properties;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ConnectionChooserPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/connection_chooser_page.ui")]
    pub(crate) struct ConnectionChooserPage {
        #[property(get, set, nullable)]
        pub(crate) connection_manager: glib::WeakRef<model::ConnectionManager>,
        #[template_child]
        pub(super) connection_list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ConnectionChooserPage {
        const NAME: &'static str = "PdsConnectionChooserPage";
        type Type = super::ConnectionChooserPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
            klass.set_css_name("connectionchooserpage");
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ConnectionChooserPage {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ConnectionChooserPage {}

    #[gtk::template_callbacks]
    impl ConnectionChooserPage {
        #[template_callback]
        fn on_notify_connection_manager(&self) {
            let obj = self.obj();
            self.connection_list_box
                .bind_model(obj.connection_manager().as_ref(), |item| {
                    gtk::ListBoxRow::builder()
                        .selectable(false)
                        .child(&view::ConnectionRow::from(item.downcast_ref().unwrap()))
                        .build()
                        .upcast()
                });
        }

        #[template_callback]
        fn on_connection_list_box_activated(&self, row: &gtk::ListBoxRow) {
            let obj = &*self.obj();

            if let Some(connection_manager) = obj.connection_manager() {
                let position = (0..connection_manager.n_items())
                    .find(|position| {
                        self.connection_list_box
                            .row_at_index(*position as i32)
                            .as_ref()
                            == Some(row)
                    })
                    .unwrap();

                let connection = connection_manager
                    .item(position)
                    .and_downcast::<model::Connection>()
                    .unwrap();

                if connection.is_active() {
                    return;
                }

                connection_manager.set_client_from(
                    &connection.uuid(),
                    clone!(
                        #[weak]
                        obj,
                        move |result| if let Err(e) = result {
                            utils::show_error_toast(
                                &obj,
                                &gettext("Error on establishing connection"),
                                &e.to_string(),
                            );
                        }
                    ),
                );
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct ConnectionChooserPage(ObjectSubclass<imp::ConnectionChooserPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
