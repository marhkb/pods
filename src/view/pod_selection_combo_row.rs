use std::cell::Cell;
use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::gio;
use gtk::glib;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::PodSelectionComboRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/pod_selection_combo_row.ui")]
    pub(crate) struct PodSelectionComboRow {
        pub(super) baking_model: RefCell<Option<gio::ListModel>>,

        #[property(get, set)]
        pub(super) active: Cell<bool>,
        #[property(get, set, nullable)]
        pub(super) pod_list: glib::WeakRef<model::PodList>,
        #[property(get, set, nullable)]
        pub(super) selected_pod: glib::WeakRef<model::Pod>,

        #[template_child]
        pub(super) switch: TemplateChild<gtk::Switch>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodSelectionComboRow {
        const NAME: &'static str = "PdsPodSelectionComboRow";
        type Type = super::PodSelectionComboRow;
        type ParentType = adw::ComboRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PodSelectionComboRow {
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
            self.on_switch_notify_active();
        }
    }

    impl WidgetImpl for PodSelectionComboRow {}
    impl ListBoxRowImpl for PodSelectionComboRow {}
    impl PreferencesRowImpl for PodSelectionComboRow {}
    impl ActionRowImpl for PodSelectionComboRow {}
    impl ComboRowImpl for PodSelectionComboRow {}

    #[gtk::template_callbacks]
    impl PodSelectionComboRow {
        #[template_callback]
        fn on_notify_pod_list(&self) {
            self.baking_model
                .replace(self.obj().pod_list().map(|pod_list| {
                    gtk::SortListModel::new(
                        Some(pod_list),
                        Some(
                            gtk::StringSorter::builder()
                                .expression(model::Pod::this_expression("name"))
                                .build(),
                        ),
                    )
                    .upcast()
                }));
        }

        #[template_callback]
        fn on_switch_notify_active(&self) {
            let obj = &*self.obj();
            if self.switch.is_active() {
                obj.set_model(self.baking_model.borrow().as_ref());
                obj.remove_css_class("pod-disabled");
            } else {
                obj.set_model(gio::ListModel::NONE);
                obj.add_css_class("pod-disabled");
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct PodSelectionComboRow(ObjectSubclass<imp::PodSelectionComboRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow, adw::ComboRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

impl PodSelectionComboRow {
    pub(crate) fn select_pod(&self, pod_id: Option<&str>) {
        let baking_model = self.imp().baking_model.borrow();
        let Some(baking_model) = &*baking_model else {
            return;
        };

        let position = pod_id
            .and_then(|id| {
                baking_model
                    .iter::<glib::Object>()
                    .map(Result::unwrap)
                    .map(|item| item.downcast::<model::Pod>().unwrap())
                    .position(|pod| pod.id().as_str() == id)
                    .map(|position| position as u32)
            })
            .unwrap_or(gtk::INVALID_LIST_POSITION);

        self.set_active(position != gtk::INVALID_LIST_POSITION);
        self.set_selected(position);
    }
}
