use adw::subclass::prelude::*;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/search_row.ui")]
    pub(crate) struct SearchRow;

    #[glib::object_subclass]
    impl ObjectSubclass for SearchRow {
        const NAME: &'static str = "PdsSearchRow";
        type Type = super::SearchRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SearchRow {
        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for SearchRow {}
}

glib::wrapper! {
    pub(crate) struct SearchRow(ObjectSubclass<imp::SearchRow>) @extends gtk::Widget;
}
