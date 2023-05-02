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

const ACTION_ZOOM_OUT: &str = "container-tty-page.zoom-out";
const ACTION_ZOOM_IN: &str = "container-tty-page.zoom-in";
const ACTION_ZOOM_NORMAL: &str = "container-tty-page.zoom-normal";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::TtyPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/container/tty-page.ui")]
    pub(crate) struct TtyPage {
        #[property(get, set, construct, nullable)]
        pub(super) container: glib::WeakRef<model::Container>,
        #[template_child]
        pub(super) zoom_control: TemplateChild<view::ZoomControl>,
        #[template_child]
        pub(super) back_navigation_controls: TemplateChild<view::BackNavigationControls>,
        #[template_child]
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) tty: TemplateChild<view::ContainerTty>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TtyPage {
        const NAME: &'static str = "PdsContainerTtyPage";
        type Type = super::TtyPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_ZOOM_OUT, None, |widget, _, _| {
                widget.imp().tty.zoom_out();
            });
            klass.install_action(ACTION_ZOOM_IN, None, |widget, _, _| {
                widget.imp().tty.zoom_in();
            });
            klass.install_action(ACTION_ZOOM_NORMAL, None, |widget, _, _| {
                widget.imp().tty.zoom_normal();
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

    impl ObjectImpl for TtyPage {
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

            self.tty.connect_terminated(clone!(@weak obj => move |_| {
                obj.imp().back_navigation_controls.navigate_back();
            }));
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for TtyPage {
        fn root(&self) {
            self.parent_root();

            let widget = &*self.obj();

            glib::idle_add_local(
                clone!(@weak widget => @default-return glib::Continue(false), move || {
                    widget.imp().tty.grab_focus();
                    glib::Continue(false)
                }),
            );
        }
    }
}

glib::wrapper! {
    pub(crate) struct TtyPage(ObjectSubclass<imp::TtyPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Container> for TtyPage {
    fn from(image: &model::Container) -> Self {
        glib::Object::builder().property("container", image).build()
    }
}
