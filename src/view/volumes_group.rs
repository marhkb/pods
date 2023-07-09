use adw::subclass::prelude::PreferencesGroupImpl;
use glib::closure;
use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::unsync::OnceCell as UnsyncOnceCell;

use crate::model;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::VolumesGroup)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/volumes_group.ui")]
    pub(crate) struct VolumesGroup {
        pub(super) sorter: UnsyncOnceCell<gtk::Sorter>,
        #[property(get, set = Self::set_volume_list, explicit_notify, nullable)]
        pub(super) volume_list: glib::WeakRef<model::VolumeList>,
        #[template_child]
        pub(super) create_volume_row: TemplateChild<gtk::ListBoxRow>,
        #[template_child]
        pub(super) create_volume_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VolumesGroup {
        const NAME: &'static str = "PdsVolumesGroup";
        type Type = super::VolumesGroup;
        type ParentType = adw::PreferencesGroup;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for VolumesGroup {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();

            let sorter = gtk::StringSorter::new(Some(
                model::Volume::this_expression("inner").chain_closure::<String>(closure!(
                    |_: model::Volume, inner: model::BoxedVolume| inner.name.clone()
                )),
            ));
            self.sorter.set(sorter.upcast()).unwrap();
        }
    }

    impl WidgetImpl for VolumesGroup {}
    impl PreferencesGroupImpl for VolumesGroup {}

    impl VolumesGroup {
        pub(super) fn set_volume_list(&self, value: Option<&model::VolumeList>) {
            let obj = &*self.obj();
            if obj.volume_list().as_ref() == value {
                return;
            }

            if let Some(volume_list) = value {
                let model = gtk::SortListModel::new(
                    Some(volume_list.to_owned()),
                    self.sorter.get().cloned(),
                );

                self.list_box.bind_model(Some(&model), |item| {
                    view::VolumeRow::from(item.downcast_ref().unwrap()).upcast()
                });
                self.list_box.append(&*self.create_volume_row);
            }

            self.volume_list.set(value);
            obj.notify("volume-list");
        }
    }
}

glib::wrapper! {
    pub(crate) struct VolumesGroup(ObjectSubclass<imp::VolumesGroup>)
        @extends gtk::Widget, adw::PreferencesGroup,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}
