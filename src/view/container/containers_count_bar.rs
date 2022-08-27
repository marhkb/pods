use gtk::glib;
use gtk::glib::closure;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/containers-count-bar.ui")]
    pub(crate) struct ContainersCountBar {
        pub(super) container_list: WeakRef<model::AbstractContainerList>,
        #[template_child]
        pub(super) dead_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) dead_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) not_running_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) not_running_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) paused_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) paused_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) running_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub(super) running_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainersCountBar {
        const NAME: &'static str = "ContainersCountBar";
        type Type = super::ContainersCountBar;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainersCountBar {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "container-list",
                    "Container List",
                    "The container list",
                    model::AbstractContainerList::static_type(),
                    glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT,
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
                "container-list" => {
                    self.container_list.set(value.get().unwrap());
                }
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "container-list" => obj.container_list().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let container_list_expr = Self::Type::this_expression("container-list");
            let dead_expr =
                container_list_expr.chain_property::<model::AbstractContainerList>("dead");
            let not_running_expr = gtk::ClosureExpression::new::<u32, _, _>(
                &[
                    container_list_expr.chain_property::<model::AbstractContainerList>("created"),
                    container_list_expr.chain_property::<model::AbstractContainerList>("exited"),
                    container_list_expr.chain_property::<model::AbstractContainerList>("removing"),
                ],
                closure!(
                    |_: Self::Type, created: u32, exited: u32, removing: u32| created
                        + exited
                        + removing
                ),
            );
            let paused_expr =
                container_list_expr.chain_property::<model::AbstractContainerList>("paused");
            let running_expr =
                container_list_expr.chain_property::<model::AbstractContainerList>("running");

            dead_expr.bind(&*self.dead_box, "visible", Some(obj));
            dead_expr.bind(&*self.dead_label, "label", Some(obj));

            not_running_expr.bind(&*self.not_running_box, "visible", Some(obj));
            not_running_expr.bind(&*self.not_running_label, "label", Some(obj));

            paused_expr.bind(&*self.paused_box, "visible", Some(obj));
            paused_expr.bind(&*self.paused_label, "label", Some(obj));

            running_expr.bind(&*self.running_box, "visible", Some(obj));
            running_expr.bind(&*self.running_label, "label", Some(obj));
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.dead_box.unparent();
            self.not_running_box.unparent();
            self.paused_box.unparent();
            self.running_box.unparent();
        }
    }

    impl WidgetImpl for ContainersCountBar {}
}

glib::wrapper! {
    pub(crate) struct ContainersCountBar(ObjectSubclass<imp::ContainersCountBar>)
        @extends gtk::Widget;
}

impl From<&model::AbstractContainerList> for ContainersCountBar {
    fn from(image: &model::AbstractContainerList) -> Self {
        glib::Object::new(&[("container-list", image)])
            .expect("Failed to create ContainersCountBar")
    }
}

impl ContainersCountBar {
    pub(crate) fn container_list(&self) -> Option<model::AbstractContainerList> {
        self.imp().container_list.upgrade()
    }
}
