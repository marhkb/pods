use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::gdk;
use gtk::glib;
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
    #[template(resource = "/com/github/marhkb/Pods/ui/view/container_terminal_page.ui")]
    pub(crate) struct ContainerTerminalPage {
        #[property(get, set, construct, nullable)]
        pub(super) container: glib::WeakRef<model::Container>,
        #[template_child]
        pub(super) zoom_control: TemplateChild<widget::ZoomControl>,
        #[template_child]
        pub(super) toolbar_view: TemplateChild<adw::ToolbarView>,
        #[template_child]
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,
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
            klass.bind_template_callbacks();

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

            self.menu_button
                .popover()
                .unwrap()
                .downcast::<gtk::PopoverMenu>()
                .unwrap()
                .add_child(&*self.zoom_control, "zoom-control");
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for ContainerTerminalPage {}

    #[gtk::template_callbacks]
    impl ContainerTerminalPage {
        #[template_callback]
        fn on_terminal_terminated(&self) {
            let obj = &*self.obj();
            let widget = obj.upcast_ref();
            match utils::try_navigation_view(widget) {
                Some(navigation_view) => {
                    navigation_view.pop();
                }
                None => utils::root(widget).close(),
            }
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
        if let Some(navigation_view) = utils::try_navigation_view(self.upcast_ref()) {
            self.action_set_enabled(ACTION_PIP_OUT, false);

            let animate_transitions = navigation_view.is_animate_transitions();
            navigation_view.set_animate_transitions(false);

            let page = navigation_view.visible_page().unwrap();
            navigation_view.pop();

            navigation_view.set_animate_transitions(animate_transitions);

            let toast_overlay = adw::ToastOverlay::new();
            toast_overlay.set_child(Some(&page));

            let window = adw::Window::builder()
                .content(&toast_overlay)
                .default_height(500)
                .default_width(700)
                .build();

            window.present();
        }
    }
}
