use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::clone;
use glib::Properties;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;
use crate::widget;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainerRenamer)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/container_renamer.ui")]
    pub(crate) struct ContainerRenamer {
        #[property(get, set, construct_only, nullable)]
        pub(super) container: glib::WeakRef<model::Container>,
        #[property(get, set)]
        pub(super) new_name: RefCell<String>,
        #[template_child]
        pub(super) entry_row: TemplateChild<widget::RandomNameEntryRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerRenamer {
        const NAME: &'static str = "PdsContainerRenamer";
        type Type = super::ContainerRenamer;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerRenamer {
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

            if let Some(container) = obj.container() {
                self.entry_row.set_text(&container.name());
            }

            self.entry_row
                .bind_property("text", obj, "new-name")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ContainerRenamer {
        fn root(&self) {
            self.parent_root();

            let widget = &*self.obj();

            glib::idle_add_local(clone!(
                #[weak]
                widget,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    widget.imp().entry_row.grab_focus();
                    glib::ControlFlow::Break
                }
            ));
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainerRenamer(ObjectSubclass<imp::ContainerRenamer>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Container> for ContainerRenamer {
    fn from(container: &model::Container) -> Self {
        glib::Object::builder()
            .property("container", container)
            .build()
    }
}
