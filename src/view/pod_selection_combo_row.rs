use std::cell::Cell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::PodSelectionComboRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/pod_selection_combo_row.ui")]
    pub(crate) struct PodSelectionComboRow {
        #[property(get, set)]
        pub(super) active: Cell<bool>,
        #[property(get, set, nullable)]
        pub(super) pod_list: glib::WeakRef<model::PodList>,
        #[property(get, set, nullable)]
        pub(super) selected_pod: glib::WeakRef<model::Pod>,

        #[template_child]
        pub(super) drop_down: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub(super) switch: TemplateChild<gtk::Switch>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodSelectionComboRow {
        const NAME: &'static str = "PdsPodSelectionComboRow";
        type Type = super::PodSelectionComboRow;
        type ParentType = adw::ActionRow;

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
    }

    impl WidgetImpl for PodSelectionComboRow {}
    impl ListBoxRowImpl for PodSelectionComboRow {}
    impl PreferencesRowImpl for PodSelectionComboRow {}
    impl ActionRowImpl for PodSelectionComboRow {}

    #[gtk::template_callbacks]
    impl PodSelectionComboRow {
        #[template_callback]
        fn on_notify_pod_list(&self) {
            self.drop_down.set_model(
                self.obj()
                    .pod_list()
                    .map(|pod_list| {
                        gtk::SortListModel::new(
                            Some(pod_list),
                            Some(
                                gtk::StringSorter::builder()
                                    .expression(model::Pod::this_expression("name"))
                                    .build(),
                            ),
                        )
                    })
                    .as_ref(),
            );
        }
    }
}

glib::wrapper! {
    pub(crate) struct PodSelectionComboRow(ObjectSubclass<imp::PodSelectionComboRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Actionable;
}

impl PodSelectionComboRow {
    pub(crate) fn select_pod(&self, pod_id: Option<&str>) {
        let imp = self.imp();

        let Some(model) = imp.drop_down.model() else {
            return;
        };

        let position = pod_id
            .and_then(|id| {
                model
                    .iter::<glib::Object>()
                    .map(Result::unwrap)
                    .map(|item| item.downcast::<model::Pod>().unwrap())
                    .position(|pod| pod.id().as_str() == id)
                    .map(|position| position as u32)
            })
            .unwrap_or(gtk::INVALID_LIST_POSITION);

        self.set_active(position != gtk::INVALID_LIST_POSITION);
        imp.drop_down.set_selected(position);
    }
}
