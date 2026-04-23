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
    #[properties(wrapper_type = super::ImagesPruneActionView)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/images_prune_action_view.ui")]
    pub(crate) struct ImagesPruneActionView {
        #[property(get, set, construct_only, nullable)]
        pub(super) action: glib::WeakRef<model::ImagesPruneAction>,

        #[template_child]
        pub(super) space_reclaimed_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImagesPruneActionView {
        const NAME: &'static str = "PdsImagesPruneActionView";
        type Type = super::ImagesPruneActionView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImagesPruneActionView {
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
                action_expr.chain_property::<model::ImagesPruneAction>("space-reclaimed");
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

    impl WidgetImpl for ImagesPruneActionView {}
}

glib::wrapper! {
    pub(crate) struct ImagesPruneActionView(ObjectSubclass<imp::ImagesPruneActionView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::ImagesPruneAction> for view::ActionDialog {
    fn from(value: &model::ImagesPruneAction) -> Self {
        Self::new(
            value.upcast_ref(),
            &gettext("Prune Images"),
            None,
            &glib::Object::builder::<ImagesPruneActionView>()
                .property("action", value)
                .build(),
            400,
        )
    }
}
