use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use glib::closure;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::VolumeSelectionRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/volume_selection_row.ui")]
    pub(crate) struct VolumeSelectionRow {
        #[property(get, set, nullable)]
        pub(super) volume: glib::WeakRef<model::Volume>,
        #[template_child]
        pub(super) label: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VolumeSelectionRow {
        const NAME: &'static str = "PdsVolumeSelectionRow";
        type Type = super::VolumeSelectionRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for VolumeSelectionRow {
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

            let volume_expr = Self::Type::this_expression("volume");
            let volume_name_expr = volume_expr.chain_property::<model::Volume>("name");
            let volume_formatted_name_expr =
                volume_name_expr.chain_closure::<String>(closure!(|_: Self::Type, name: &str| {
                    utils::format_volume_name(name).to_owned()
                }));

            volume_formatted_name_expr.bind(&*self.label, "label", Some(obj));
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for VolumeSelectionRow {}
}

glib::wrapper! {
    pub(crate) struct VolumeSelectionRow(ObjectSubclass<imp::VolumeSelectionRow>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
