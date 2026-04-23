use std::cell::Cell;
use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::model;
use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::VolumeSelectionComboRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/volume_selection_combo_row.ui")]
    pub(crate) struct VolumeSelectionComboRow {
        pub(super) bindings: RefCell<Vec<glib::Binding>>,

        #[property(get, set)]
        pub(super) active: Cell<bool>,
        #[property(get, set, nullable)]
        pub(super) mount: glib::WeakRef<model::Mount>,
        #[property(get, set = Self::set_selected_volume, explicit_notify, nullable)]
        pub(super) selected_volume: glib::WeakRef<model::Volume>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VolumeSelectionComboRow {
        const NAME: &'static str = "PdsVolumeSelectionComboRow";
        type Type = super::VolumeSelectionComboRow;
        type ParentType = adw::ComboRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for VolumeSelectionComboRow {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }
    }

    impl WidgetImpl for VolumeSelectionComboRow {}
    impl ListBoxRowImpl for VolumeSelectionComboRow {}
    impl PreferencesRowImpl for VolumeSelectionComboRow {}
    impl ActionRowImpl for VolumeSelectionComboRow {}
    impl ComboRowImpl for VolumeSelectionComboRow {}

    #[gtk::template_callbacks]
    impl VolumeSelectionComboRow {
        fn set_selected_volume(&self, volume: Option<&model::Volume>) {
            let obj = &*self.obj();

            if obj.selected_volume().as_ref() == volume {
                return;
            }

            let Some(model) = obj.model() else {
                return;
            };

            let position = volume
                .map(|volume| volume.name())
                .and_then(|name| {
                    model
                        .iter::<glib::Object>()
                        .map(Result::unwrap)
                        .map(|item| item.downcast::<model::Volume>().unwrap())
                        .position(|volume| volume.name().as_str() == name)
                        .map(|position| position as u32)
                })
                .unwrap_or(gtk::INVALID_LIST_POSITION);

            obj.set_active(position != gtk::INVALID_LIST_POSITION);
            obj.set_selected(position);

            self.selected_volume.set(volume);
            obj.notify_selected_volume();
        }

        #[template_callback]
        fn on_notify_mount(&self) {
            let obj = &*self.obj();

            while let Some(binding) = self.bindings.borrow_mut().pop() {
                binding.unbind();
            }

            obj.set_model(
                obj.mount()
                    .and_then(|mount| mount.client())
                    .map(|client| {
                        gtk::SortListModel::new(
                            Some(client.volume_list()),
                            Some(
                                gtk::StringSorter::builder()
                                    .expression(model::Volume::this_expression("name"))
                                    .build(),
                            ),
                        )
                    })
                    .as_ref(),
            );

            match obj.mount() {
                Some(mount) => {
                    match mount.volume() {
                        Some(volume) => self.set_selected_volume(Some(&volume)),
                        None => mount.set_volume(obj.selected_volume().as_ref()),
                    }
                    let binding = mount
                        .bind_property("volume", obj, "selected-volume")
                        .bidirectional()
                        .build();
                    self.bindings.borrow_mut().push(binding);
                }
                None => {
                    self.set_selected_volume(None);
                }
            }
        }

        #[template_callback]
        fn on_setup(&self, list_item: &gtk::ListItem) {
            list_item.set_child(Some(&gtk::Label::builder().xalign(0.0).build()));
        }

        #[template_callback]
        fn on_bind(&self, list_item: &gtk::ListItem) {
            let volume = list_item.item().and_downcast::<model::Volume>().unwrap();
            let label = list_item.child().and_downcast::<gtk::Label>().unwrap();

            label.set_label(utils::format_volume_name(&volume.name()));
        }
    }
}

glib::wrapper! {
    pub(crate) struct VolumeSelectionComboRow(ObjectSubclass<imp::VolumeSelectionComboRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow, adw::ComboRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}
