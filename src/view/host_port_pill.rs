use std::cell::RefCell;

use adw::subclass::prelude::*;
use glib::Properties;
use glib::closure;
use gtk::CompositeTemplate;
use gtk::glib;
use gtk::prelude::*;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::HostPortPill)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/host_port_pill.ui")]
    pub(crate) struct HostPortPill {
        #[property(get, set, construct, nullable)]
        pub(super) port_binding: RefCell<Option<model::PortBinding>>,

        #[template_child]
        pub(super) label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for HostPortPill {
        const NAME: &'static str = "PdsHostPortPill";
        type Type = super::HostPortPill;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for HostPortPill {
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

            let port_binding_expr = Self::Type::this_expression("port-binding");
            let port_binding_list_expr =
                port_binding_expr.chain_property::<model::PortBinding>("port-binding-list");
            let container_expr =
                port_binding_list_expr.chain_property::<model::PortBindingList>("container");
            let container_status_expr = container_expr.chain_property::<model::Container>("status");

            port_binding_expr
                .chain_closure::<String>(closure!(
                    |_: Self::Type, port_binding: model::PortBinding| {
                        format!("{}/{}", port_binding.host_port(), port_binding.protocol())
                    }
                ))
                .bind(&*self.label, "label", Some(obj));

            let css_classes = utils::css_classes(&*self.label);
            container_status_expr
                .chain_closure::<Vec<String>>(closure!(
                    |_: Self::Type, status: model::ContainerStatus| {
                        css_classes
                            .iter()
                            .cloned()
                            .chain(Some(String::from(
                                super::super::container_status_css_class(status),
                            )))
                            .collect::<Vec<_>>()
                    }
                ))
                .bind(&*self.label, "css-classes", Some(obj));
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for HostPortPill {}
}

glib::wrapper! {
    pub(crate) struct HostPortPill(ObjectSubclass<imp::HostPortPill>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::PortBinding> for HostPortPill {
    fn from(value: &model::PortBinding) -> Self {
        glib::Object::builder()
            .property("port-binding", value)
            .build()
    }
}
