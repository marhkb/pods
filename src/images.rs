use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

mod imp {
    use super::*;

    use gtk::CompositeTemplate;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Symphony/ui/images.ui")]
    pub struct Images {}

    #[glib::object_subclass]
    impl ObjectSubclass for Images {
        const NAME: &'static str = "Images";
        type Type = super::Images;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Images {}
    impl WidgetImpl for Images {}
}

glib::wrapper! {
    pub struct Images(ObjectSubclass<imp::Images>)
        @extends gtk::Widget;
}

impl Default for Images {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create Images")
    }
}
