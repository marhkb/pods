use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::gio;
use gtk::glib;

use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::NetworksGridView)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/networks_grid_view.ui")]
    pub(crate) struct NetworksGridView {
        #[property(get, set = Self::set_model, nullable, construct)]
        pub(super) model: glib::WeakRef<gio::ListModel>,
        #[template_child]
        pub(super) flow_box: TemplateChild<gtk::FlowBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NetworksGridView {
        const NAME: &'static str = "PdsNetworksGridView";
        type Type = super::NetworksGridView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for NetworksGridView {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for NetworksGridView {}

    impl NetworksGridView {
        pub(super) fn set_model(&self, value: Option<&gio::ListModel>) {
            let obj = &*self.obj();
            if obj.model().as_ref() == value {
                return;
            }

            self.flow_box.bind_model(value, |item| {
                gtk::FlowBoxChild::builder()
                    .focusable(false)
                    .child(&view::NetworkCard::from(item.downcast_ref().unwrap()))
                    .build()
                    .upcast()
            });

            self.model.set(value);
        }
    }
}

glib::wrapper! {
    pub(crate) struct NetworksGridView(ObjectSubclass<imp::NetworksGridView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for NetworksGridView {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl From<Option<&gio::ListModel>> for NetworksGridView {
    fn from(model: Option<&gio::ListModel>) -> Self {
        glib::Object::builder().property("model", model).build()
    }
}

impl NetworksGridView {
    pub(crate) fn select_visible(&self) {
        (0..)
            .map(|pos| self.imp().flow_box.child_at_index(pos))
            .take_while(Option::is_some)
            .flatten()
            .for_each(|row| {
                row.child()
                    .unwrap()
                    .downcast_ref::<view::NetworkCard>()
                    .unwrap()
                    .network()
                    .unwrap()
                    .set_selected(row.is_visible());
            });
    }
}
