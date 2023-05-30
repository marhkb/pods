use adw::traits::BinExt;
use glib::clone;
use glib::Properties;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;
use crate::view;
use crate::widget;

const ACTION_PIP_OUT: &str = "container-terminal-page.pip-out";
const ACTION_ZOOM_OUT: &str = "container-terminal-page.zoom-out";
const ACTION_ZOOM_IN: &str = "container-terminal-page.zoom-in";
const ACTION_ZOOM_NORMAL: &str = "container-terminal-page.zoom-normal";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainerTerminalPage)]
    #[template(file = "container_terminal_page.ui")]
    pub(crate) struct ContainerTerminalPage {
        #[property(get, set, construct, nullable)]
        pub(super) container: glib::WeakRef<model::Container>,
        #[template_child]
        pub(super) zoom_control: TemplateChild<widget::ZoomControl>,
        #[template_child]
        pub(super) back_navigation_controls: TemplateChild<widget::BackNavigationControls>,
        #[template_child]
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) detach_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) terminal: TemplateChild<view::ContainerTerminal>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerTerminalPage {
        const NAME: &'static str = "PdsContainerTerminalPage";
        type Type = super::ContainerTerminalPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_PIP_OUT, None, |widget, _, _| {
                widget.pip_out();
            });

            klass.install_action(ACTION_ZOOM_OUT, None, |widget, _, _| {
                widget.imp().terminal.zoom_out();
            });
            klass.install_action(ACTION_ZOOM_IN, None, |widget, _, _| {
                widget.imp().terminal.zoom_in();
            });
            klass.install_action(ACTION_ZOOM_NORMAL, None, |widget, _, _| {
                widget.imp().terminal.zoom_normal();
            });

            klass.add_binding_action(
                gdk::Key::minus,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_ZOOM_OUT,
                None,
            );
            klass.add_binding_action(
                gdk::Key::KP_Subtract,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_ZOOM_OUT,
                None,
            );

            klass.add_binding_action(
                gdk::Key::plus,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_ZOOM_IN,
                None,
            );
            klass.add_binding_action(
                gdk::Key::KP_Add,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_ZOOM_IN,
                None,
            );
            klass.add_binding_action(
                gdk::Key::equal,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_ZOOM_IN,
                None,
            );

            klass.add_binding_action(
                gdk::Key::_0,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_ZOOM_NORMAL,
                None,
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerTerminalPage {
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

            self.menu_button
                .popover()
                .unwrap()
                .downcast::<gtk::PopoverMenu>()
                .unwrap()
                .add_child(&*self.zoom_control, "zoom-control");

            self.terminal
                .connect_terminated(clone!(@weak obj => move |_| {
                    if !obj.imp().back_navigation_controls.navigate_back() {
                        utils::root(obj.upcast_ref()).close();
                    }
                }));
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for ContainerTerminalPage {
        fn root(&self) {
            self.parent_root();

            let widget = &*self.obj();

            self.zoom_control
                .set_zoom_in_action_name(Some("container-terminal-page.zoom-in"));
            self.zoom_control
                .set_zoom_normal_action_name(Some("container-terminal-page.zoom-normal"));
            self.zoom_control
                .set_zoom_out_action_name(Some("container-terminal-page.zoom-out"));

            self.detach_button
                .set_action_name(Some("container-terminal-page.pip-out"));

            glib::idle_add_local(
                clone!(@weak widget => @default-return glib::Continue(false), move || {
                    widget.imp().terminal.grab_focus();
                    glib::Continue(false)
                }),
            );
        }

        fn unroot(&self) {
            self.parent_unroot();

            // We have to unset the action while when unrooting and set them again when rooting.
            // Otherwise, the widgets would be insensitive.
            self.zoom_control.set_zoom_in_action_name(None);
            self.zoom_control.set_zoom_normal_action_name(None);
            self.zoom_control.set_zoom_out_action_name(None);

            self.detach_button.set_action_name(None);
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainerTerminalPage(ObjectSubclass<imp::ContainerTerminalPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Container> for ContainerTerminalPage {
    fn from(image: &model::Container) -> Self {
        glib::Object::builder().property("container", image).build()
    }
}

impl ContainerTerminalPage {
    pub(crate) fn pip_out(&self) {
        if let Some(leaflet_overlay) = utils::parent_leaflet_overlay(self.upcast_ref()) {
            self.action_set_enabled(ACTION_PIP_OUT, false);

            leaflet_overlay.set_child(gtk::Widget::NONE);
            leaflet_overlay.hide_details();

            let toast_overlay = adw::ToastOverlay::new();
            toast_overlay.set_child(Some(self));

            let window = adw::Window::builder()
                .content(&toast_overlay)
                .default_height(500)
                .default_width(700)
                .build();

            window.present();
        }
    }
}
