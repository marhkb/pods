use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainersCountBar)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/containers_count_bar.ui")]
    pub(crate) struct ContainersCountBar {
        #[property(get, set, construct, nullable)]
        pub(super) container_list: glib::WeakRef<model::AbstractContainerList>,
        #[template_child]
        pub(super) dead_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) dead_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) not_running_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) not_running_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) running_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) running_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainersCountBar {
        const NAME: &'static str = "PdsContainersCountBar";
        type Type = super::ContainersCountBar;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainersCountBar {
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

            let container_list_expr = Self::Type::this_expression("container-list");
            let dead_expr =
                container_list_expr.chain_property::<model::AbstractContainerList>("dead");
            let not_running_expr =
                container_list_expr.chain_property::<model::AbstractContainerList>("not-running");
            let running_expr =
                container_list_expr.chain_property::<model::AbstractContainerList>("running");

            dead_expr.bind(&*self.dead_box, "visible", Some(obj));
            dead_expr.bind(&*self.dead_label, "label", Some(obj));

            not_running_expr.bind(&*self.not_running_box, "visible", Some(obj));
            not_running_expr.bind(&*self.not_running_label, "label", Some(obj));

            running_expr.bind(&*self.running_box, "visible", Some(obj));
            running_expr.bind(&*self.running_label, "label", Some(obj));
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for ContainersCountBar {}
}

glib::wrapper! {
    pub(crate) struct ContainersCountBar(ObjectSubclass<imp::ContainersCountBar>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::AbstractContainerList> for ContainersCountBar {
    fn from(container_list: &model::AbstractContainerList) -> Self {
        glib::Object::builder()
            .property("container-list", container_list)
            .build()
    }
}
