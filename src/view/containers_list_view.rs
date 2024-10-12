use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::gio;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainersListView)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/containers_list_view.ui")]
    pub(crate) struct ContainersListView {
        #[property(get, set = Self::set_model, nullable, construct)]
        pub(super) model: glib::WeakRef<gio::ListModel>,
        #[template_child]
        pub(super) list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainersListView {
        const NAME: &'static str = "PdsContainersListView";
        type Type = super::ContainersListView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainersListView {
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

    impl WidgetImpl for ContainersListView {}

    impl ContainersListView {
        pub(super) fn set_model(&self, value: Option<&gio::ListModel>) {
            let obj = &*self.obj();
            if obj.model().as_ref() == value {
                return;
            }

            self.list_box.bind_model(value, |item| {
                view::ContainerRow::from(item.downcast_ref().unwrap()).upcast()
            });

            self.model.set(value);
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainersListView(ObjectSubclass<imp::ContainersListView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for ContainersListView {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl From<Option<&gio::ListModel>> for ContainersListView {
    fn from(model: Option<&gio::ListModel>) -> Self {
        glib::Object::builder().property("model", model).build()
    }
}

impl ContainersListView {
    pub(crate) fn select_visible(&self) {
        (0..)
            .map(|pos| self.imp().list_box.row_at_index(pos))
            .take_while(Option::is_some)
            .flatten()
            .for_each(|row| {
                row.downcast_ref::<view::ContainerRow>()
                    .unwrap()
                    .container()
                    .unwrap()
                    .set_selected(row.is_visible());
            });
    }
}
