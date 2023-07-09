use glib::Cast;
use gtk::glib;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::utils;

mod imp {
    use super::*;

    #[derive(Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/efficient_spinner.ui")]
    pub(crate) struct EfficientSpinner {
        #[template_child]
        pub(super) spinner: TemplateChild<gtk::Spinner>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EfficientSpinner {
        const NAME: &'static str = "PdsEfficientSpinner";
        type Type = super::EfficientSpinner;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EfficientSpinner {
        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }
    impl WidgetImpl for EfficientSpinner {
        fn map(&self) {
            self.parent_map();
            self.spinner.set_spinning(true);
        }

        fn unmap(&self) {
            self.parent_unmap();
            self.spinner.set_spinning(false);
        }
    }
}

glib::wrapper! {
    pub(crate) struct EfficientSpinner(ObjectSubclass<imp::EfficientSpinner>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
