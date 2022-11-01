use adw::subclass::prelude::BinImpl;
use adw::traits::BinExt;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/component/leaflet-overlay.ui")]
    pub(crate) struct LeafletOverlay;

    #[glib::object_subclass]
    impl ObjectSubclass for LeafletOverlay {
        const NAME: &'static str = "PdsLeafletOverlay";
        type Type = super::LeafletOverlay;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LeafletOverlay {
        fn constructed(&self) {
            self.parent_constructed();
        }
    }

    impl WidgetImpl for LeafletOverlay {
        fn realize(&self) {
            self.parent_realize();

            let widget = &*self.obj();

            widget.leaflet().connect_child_transition_running_notify(
                clone!(@weak widget => move |leaflet| {
                    if !leaflet.is_child_transition_running()
                        && &leaflet.visible_child().unwrap() != widget.upcast_ref::<gtk::Widget>() {
                            widget.set_child(gtk::Widget::NONE);
                    }
                }),
            );
        }
    }

    impl BinImpl for LeafletOverlay {}
}

glib::wrapper! {
    pub(crate) struct LeafletOverlay(ObjectSubclass<imp::LeafletOverlay>)
        @extends gtk::Widget, adw::Bin;
}

impl Default for LeafletOverlay {
    fn default() -> Self {
        glib::Object::builder::<Self>().build()
    }
}

impl LeafletOverlay {
    fn leaflet(&self) -> adw::Leaflet {
        self.ancestor(adw::Leaflet::static_type())
            .unwrap()
            .downcast::<adw::Leaflet>()
            .unwrap()
    }

    pub(crate) fn show_details<W: glib::IsA<gtk::Widget>>(&self, widget: &W) {
        self.set_child(Some(widget));
        self.leaflet().navigate(adw::NavigationDirection::Forward);
    }

    pub(crate) fn hide_details(&self) {
        self.leaflet().navigate(adw::NavigationDirection::Back);
    }
}
