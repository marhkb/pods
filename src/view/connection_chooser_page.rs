use std::cell::OnceCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::clone;
use glib::Properties;
use gtk::gdk;
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
        pub(super) filter: OnceCell<gtk::Filter>,
        #[property(get, set, nullable)]
        pub(crate) connection_manager: glib::WeakRef<model::ConnectionManager>,
        #[template_child]
        pub(super) filter_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) title_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) filter_entry: TemplateChild<gtk::SearchEntry>,
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
        fn on_filter_button_toggled(&self) {
            if self.filter_button.is_active() {
                self.filter_entry.grab_focus();
                self.title_stack.set_visible_child(&self.filter_entry.get());
            } else {
                self.filter_entry.set_text("");
                self.title_stack.set_visible_child_name("title");
            }
        }

        #[template_callback]
        fn on_filter_started(&self) {
            self.filter_button.set_active(true)
        }

        #[template_callback]
        fn on_filter_changed(&self) {
            self.filter().changed(gtk::FilterChange::Different);
        }

        #[template_callback]
        fn on_filter_key_pressed(
            &self,
            key: gdk::Key,
            _: u32,
            _: gdk::ModifierType,
            _: &gtk::EventControllerKey,
        ) -> glib::Propagation {
            if key == gdk::Key::Escape {
                self.obj().enable_filter_mode(false);
            }
            // else if key == gdk::Key::KP_Enter {
            //     self.obj().activate_action(ACTION_SELECT, None).unwrap();
            // }

            glib::Propagation::Proceed
        }

        #[template_callback]
        fn on_notify_connection_manager(&self) {
            let obj = self.obj();

            let model =
                gtk::FilterListModel::new(obj.connection_manager(), Some(self.filter().to_owned()));

            // self.connection_list_box.set_filter_func(filter_func);
            self.connection_list_box.bind_model(Some(&model), |item| {
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

        pub(super) fn filter(&self) -> &gtk::Filter {
            let obj = &*self.obj();

            self.filter.get_or_init(|| {
                gtk::CustomFilter::new(clone!(
                    #[weak]
                    obj,
                    #[upgrade_or]
                    false,
                    move |item| {
                        let term = obj.imp().filter_entry.text().to_lowercase();
                        let connection = item.downcast_ref::<model::Connection>().unwrap();

                        connection.name().to_lowercase().contains(&term)
                            || connection.url().to_lowercase().contains(&term)
                    }
                ))
                .upcast()
            })
        }
    }
}

glib::wrapper! {
    pub(crate) struct ConnectionChooserPage(ObjectSubclass<imp::ConnectionChooserPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ConnectionChooserPage {
    pub(crate) fn toggle_filter_mode(&self) {
        self.enable_filter_mode(!self.imp().filter_button.is_active());
    }

    pub(crate) fn enable_filter_mode(&self, enable: bool) {
        let imp = self.imp();

        imp.filter_button.set_active(enable);
        if !enable {
            imp.filter().changed(gtk::FilterChange::LessStrict);
        }
    }
}
