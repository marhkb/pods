use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::Properties;
use glib::closure;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::model;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainersPruneActionView)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/containers_prune_action_view.ui")]
    pub(crate) struct ContainersPruneActionView {
        #[property(get, set, construct_only, nullable)]
        pub(super) action: glib::WeakRef<model::ContainersPruneAction>,

        #[template_child]
        pub(super) space_reclaimed_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainersPruneActionView {
        const NAME: &'static str = "PdsContainersPruneActionView";
        type Type = super::ContainersPruneActionView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainersPruneActionView {
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

            let action_expr = Self::Type::this_expression("action");
            let action_space_reclaimed_expr =
                action_expr.chain_property::<model::ContainersPruneAction>("space-reclaimed");
            let action_space_reclaimed_formatted_expr = action_space_reclaimed_expr
                .chain_closure::<String>(closure!(|_: Self::Type, space_reclaimed: u64| gettext!(
                    "Space Reclaimed: <b>{}</b>",
                    glib::format_size(space_reclaimed)
                )));

            action_space_reclaimed_formatted_expr.bind(
                &*self.space_reclaimed_label,
                "label",
                Some(obj),
            );
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ContainersPruneActionView {}
}

glib::wrapper! {
    pub(crate) struct ContainersPruneActionView(ObjectSubclass<imp::ContainersPruneActionView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::ContainersPruneAction> for view::ActionDialog {
    fn from(value: &model::ContainersPruneAction) -> Self {
        Self::new(
            value.upcast_ref(),
            &gettext("Prune Containers"),
            None,
            &glib::Object::builder::<ContainersPruneActionView>()
                .property("action", value)
                .build(),
            400,
        )
    }
}
