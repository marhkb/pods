use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::sync::OnceLock;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use glib::clone;
use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::glib;
use gtk::glib::subclass::Signal;

use crate::model;
use crate::utils;
use crate::widget;

const ACTION_SELECT: &str = "image-remote-selection-page.select";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImageRemoteSelectionPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/image_remote_selection_page.ui")]
    pub(crate) struct ImageRemoteSelectionPage {
        #[property(get, set, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[property(get, set)]
        pub(super) show_cancel_button: Cell<bool>,
        #[property(get, set, construct)]
        pub(super) action_button_name: RefCell<String>,
        #[property(get, set, construct)]
        pub(super) top_level: OnceCell<bool>,
        #[template_child]
        pub(super) size_group: TemplateChild<gtk::SizeGroup>,
        #[template_child]
        pub(super) cancel_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) image_suggestion_entry_row: TemplateChild<widget::SuggestionEntryRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageRemoteSelectionPage {
        const NAME: &'static str = "PdsImageRemoteSelectionPage";
        type Type = super::ImageRemoteSelectionPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action(ACTION_SELECT, None, |widget, _, _| {
                widget.select();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImageRemoteSelectionPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![
                    Signal::builder("image-selected")
                        .param_types([String::static_type()])
                        .build(),
                ]
            })
        }

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
            self.on_image_suggestion_entry_row_changed();
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ImageRemoteSelectionPage {
        fn root(&self) {
            self.parent_root();

            let widget = &*self.obj();

            glib::idle_add_local(clone!(
                #[weak]
                widget,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    widget.imp().image_suggestion_entry_row.grab_focus();
                    glib::ControlFlow::Break
                }
            ));
        }
    }

    impl PreferencesGroupImpl for ImageRemoteSelectionPage {}

    #[gtk::template_callbacks]
    impl ImageRemoteSelectionPage {
        #[template_callback]
        fn on_notify_show_cancel_button(&self) {
            if self.obj().show_cancel_button() {
                self.size_group.add_widget(&self.cancel_button.get())
            } else {
                self.size_group.remove_widget(&self.cancel_button.get())
            }
        }

        #[template_callback]
        fn on_image_suggestion_entry_row_activated(&self) {
            self.obj().select();
        }

        #[template_callback]
        fn on_image_suggestion_entry_row_changed(&self) {
            self.obj().action_set_enabled(
                ACTION_SELECT,
                !self.image_suggestion_entry_row.text().is_empty(),
            );
        }

        #[template_callback]
        fn on_image_suggestion_entry_row_key_pressed(
            &self,
            key: gdk::Key,
            _: u32,
            _: gdk::ModifierType,
            _: &gtk::EventControllerKey,
        ) -> glib::Propagation {
            if key == gdk::Key::Return || key == gdk::Key::KP_Enter {
                self.obj().select();
            } else if key == gdk::Key::Escape {
                let obj = &*self.obj();
                obj.activate_action(
                    if obj.top_level() {
                        "window.close"
                    } else {
                        "navigation.pop"
                    },
                    None,
                )
                .unwrap();
            }

            glib::Propagation::Proceed
        }
    }
}

glib::wrapper! {
    pub(crate) struct ImageRemoteSelectionPage(ObjectSubclass<imp::ImageRemoteSelectionPage>)
        @extends adw::PreferencesGroup, gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ImageRemoteSelectionPage {
    pub(crate) fn new(client: &model::Client, action_button_name: &str, top_level: bool) -> Self {
        glib::Object::builder()
            .property("client", client)
            .property("action-button-name", action_button_name)
            .property("top-level", top_level)
            .build()
    }

    pub(crate) fn select(&self) {
        let image = self.imp().image_suggestion_entry_row.text();

        if !image.is_empty() {
            self.emit_by_name::<()>("image-selected", &[&image]);
        }
    }

    pub(crate) fn connect_image_selected<F: Fn(&Self, &String) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("image-selected", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let image = values[1].get::<String>().unwrap();
            f(&obj, &image);

            None
        })
    }
}
