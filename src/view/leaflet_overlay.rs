use adw::subclass::prelude::BinImpl;
use adw::traits::BinExt;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/leaflet-overlay.ui")]
    pub(crate) struct LeafletOverlay;

    #[glib::object_subclass]
    impl ObjectSubclass for LeafletOverlay {
        const NAME: &'static str = "LeafletOverlay";
        type Type = super::LeafletOverlay;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for LeafletOverlay {}
    impl WidgetImpl for LeafletOverlay {}
    impl BinImpl for LeafletOverlay {}
}

glib::wrapper! {
    pub(crate) struct LeafletOverlay(ObjectSubclass<imp::LeafletOverlay>)
        @extends gtk::Widget, adw::Bin;
}

impl Default for LeafletOverlay {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create LeafletOverlay")
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
        self.set_child(gtk::Widget::NONE);
    }

    pub(crate) fn topmost_leaflet_overlay(&self) -> Self {
        match first_leaflet_overlay(self) {
            None => self.to_owned(),
            Some(first) => first.topmost_leaflet_overlay(),
        }
    }
}

fn first_leaflet_overlay<W: glib::IsA<gtk::Widget>>(widget: &W) -> Option<LeafletOverlay> {
    utils::ChildIter::from(widget)
        .find_map(|child| child.downcast::<LeafletOverlay>().ok())
        .or_else(|| utils::ChildIter::from(widget).find_map(|child| first_leaflet_overlay(&child)))
}
