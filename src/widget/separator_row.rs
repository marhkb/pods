use adw::subclass::prelude::*;
use gtk::glib;
use gtk::CompositeTemplate;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/separator_row.ui")]
    pub(crate) struct SeparatorRow;

    #[glib::object_subclass]
    impl ObjectSubclass for SeparatorRow {
        const NAME: &'static str = "PdsSeparatorRow";
        type Type = super::SeparatorRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SeparatorRow {}
    impl WidgetImpl for SeparatorRow {}
    impl ListBoxRowImpl for SeparatorRow {}
}

glib::wrapper! {
    pub(crate) struct SeparatorRow(ObjectSubclass<imp::SeparatorRow>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
