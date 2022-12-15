use gettextrs::gettext;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::utils;
use crate::view;

const ACTION_GO_FIRST: &str = "back-navigation-controls.go-first";
const ACTION_BACK: &str = "back-navigation-controls.back";

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/component/back-navigation-controls.ui")]
    pub(crate) struct BackNavigationControls {
        #[template_child]
        pub(super) box_: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for BackNavigationControls {
        const NAME: &'static str = "PdsBackNavigationControls";
        type Type = super::BackNavigationControls;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action(ACTION_GO_FIRST, None, move |widget, _, _| {
                widget.navigate_to_first();
            });
            klass.install_action(ACTION_BACK, None, move |widget, _, _| {
                widget.navigate_back();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for BackNavigationControls {
        fn dispose(&self) {
            utils::ChildIter::from(&*self.obj()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for BackNavigationControls {
        fn root(&self) {
            self.parent_root();

            let widget = &*self.obj();

            if let Some(leaflet_overlay) = widget.previous_leaflet_overlay() {
                self.box_.append(
                    &gtk::Button::builder()
                        .icon_name("go-previous-symbolic")
                        .action_name("back-navigation-controls.back")
                        .tooltip_text(&gettext("Go Back"))
                        .build(),
                );

                if leaflet_overlay != widget.root_leaflet_overlay() {
                    self.box_.append(
                        &gtk::Button::builder()
                            .icon_name("go-home-symbolic")
                            .action_name("back-navigation-controls.go-first")
                            .tooltip_text(&gettext("Main View"))
                            .build(),
                    );
                }
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct BackNavigationControls(ObjectSubclass<imp::BackNavigationControls>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl BackNavigationControls {
    pub(crate) fn navigate_to_first(&self) {
        self.root_leaflet_overlay().hide_details();
    }

    pub(crate) fn navigate_back(&self) {
        if let Some(leaflet_overlay) = self.previous_leaflet_overlay() {
            leaflet_overlay.hide_details();
        }
    }

    fn previous_leaflet_overlay(&self) -> Option<view::LeafletOverlay> {
        utils::parent_leaflet_overlay(self)
    }

    fn root_leaflet_overlay(&self) -> view::LeafletOverlay {
        utils::topmost_leaflet_overlay(self).unwrap()
    }
}
