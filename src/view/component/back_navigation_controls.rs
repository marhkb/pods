use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/component/back-navigation-controls.ui")]
    pub(crate) struct PdsBackNavigationControls;

    #[glib::object_subclass]
    impl ObjectSubclass for PdsBackNavigationControls {
        const NAME: &'static str = "PdsBackNavigationControls";
        type Type = super::BackNavigationControls;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action("navigation.go-first", None, move |widget, _, _| {
                widget.navigate_to_first();
            });
            klass.install_action("navigation.back", None, move |widget, _, _| {
                widget.navigate_back();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PdsBackNavigationControls {
        fn dispose(&self, obj: &Self::Type) {
            let mut child = obj.first_child();
            while let Some(child_) = child {
                child = child_.next_sibling();
                child_.unparent();
            }
        }
    }

    impl WidgetImpl for PdsBackNavigationControls {
        fn root(&self, widget: &Self::Type) {
            self.parent_root(widget);

            widget.action_set_enabled(
                "navigation.go-first",
                widget.previous_leaflet_overlay() != widget.root_leaflet_overlay(),
            );
        }
    }
}

glib::wrapper! {
    pub(crate) struct BackNavigationControls(ObjectSubclass<imp::PdsBackNavigationControls>)
        @extends gtk::Widget;
}

impl BackNavigationControls {
    pub(crate) fn navigate_to_first(&self) {
        self.root_leaflet_overlay().hide_details();
    }

    pub(crate) fn navigate_back(&self) {
        self.previous_leaflet_overlay().hide_details();
    }

    fn previous_leaflet_overlay(&self) -> view::LeafletOverlay {
        utils::find_parent_leaflet_overlay(self)
    }

    fn root_leaflet_overlay(&self) -> view::LeafletOverlay {
        utils::root(self).leaflet_overlay()
    }
}
