use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::closure;
use glib::Properties;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::InfoRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/info_row.ui")]
    pub(crate) struct InfoRow {
        #[property(get, set)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[template_child]
        pub(super) version_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) version_label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for InfoRow {
        const NAME: &'static str = "PdsInfoRow";
        type Type = super::InfoRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for InfoRow {
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

            let version_expr =
                Self::Type::this_expression("client").chain_property::<model::Client>("version");

            version_expr
                .chain_closure::<String>(closure!(|_: Self::Type, version: Option<&str>| version
                    .map(|_| "version")
                    .unwrap_or("loading")))
                .bind(&*self.version_stack, "visible-child-name", Some(obj));

            version_expr
                .chain_closure::<String>(closure!(|_: Self::Type, version: Option<&str>| {
                    version
                        .map(|version| format!("v{version}"))
                        .unwrap_or_default()
                }))
                .bind(&*self.version_label, "label", Some(obj));
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for InfoRow {}
}

glib::wrapper! {
    pub(crate) struct InfoRow(ObjectSubclass<imp::InfoRow>) @extends gtk::Widget;
}
