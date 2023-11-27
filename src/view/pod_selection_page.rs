use std::cell::OnceCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::clone;
use glib::once_cell::sync::Lazy as SyncLazy;
use glib::Properties;
use gtk::gdk;
use gtk::glib;
use gtk::glib::subclass::Signal;
use gtk::pango;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;

const ACTION_FILTER: &str = "pod-selection-page.filter";
const ACTION_SELECT: &str = "pod-selection-page.select";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::PodSelectionPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/pod_selection_page.ui")]
    pub(crate) struct PodSelectionPage {
        pub(super) filter: OnceCell<gtk::Filter>,
        #[property(get, set = Self::set_pod_list, nullable)]
        pub(super) pod_list: glib::WeakRef<model::PodList>,
        #[template_child]
        pub(super) filter_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) title_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) filter_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub(super) select_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) selection: TemplateChild<gtk::SingleSelection>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodSelectionPage {
        const NAME: &'static str = "PdsPodSelectionPage";
        type Type = super::PodSelectionPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action(ACTION_FILTER, Some("b"), |widget, _, data| {
                widget.enable_search_mode(data.unwrap().get().unwrap());
            });
            klass.add_binding_action(
                gdk::Key::F,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_FILTER,
                Some(&true.to_variant()),
            );
            klass.add_binding_action(
                gdk::Key::Escape,
                gdk::ModifierType::empty(),
                ACTION_FILTER,
                Some(&false.to_variant()),
            );

            klass.install_action(ACTION_SELECT, None, |widget, _, _| {
                widget.select();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PodSelectionPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: SyncLazy<Vec<Signal>> = SyncLazy::new(|| {
                vec![Signal::builder("pod-selected")
                    .param_types([model::Pod::static_type()])
                    .build()]
            });
            SIGNALS.as_ref()
        }

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

            let obj = &*self.obj();

            self.filter_entry.set_key_capture_widget(Some(obj));

            let filter =
                gtk::CustomFilter::new(clone!(@weak obj => @default-return false, move |item| {
                    let term = obj.imp().filter_entry.text().to_lowercase();
                    let pod = item.downcast_ref::<model::Pod>().unwrap();

                    pod.name().to_lowercase().contains(&term)
                }));
            self.filter.set(filter.upcast()).unwrap();
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for PodSelectionPage {}

    #[gtk::template_callbacks]
    impl PodSelectionPage {
        #[template_callback]
        fn on_filter_button_toggled(&self) {
            if self.filter_button.is_active() {
                self.filter_entry.grab_focus();
                self.title_stack.set_visible_child(&self.filter_entry.get());
            } else {
                self.filter_entry.set_text("");
                self.title_stack.set_visible_child_name("title");
            }
        }

        #[template_callback]
        fn on_filter_started(&self) {
            self.filter_button.set_active(true)
        }

        #[template_callback]
        fn on_filter_changed(&self) {
            self.update_filter(gtk::FilterChange::Different);
        }

        #[template_callback]
        fn on_filter_key_pressed(
            &self,
            key: gdk::Key,
            _: u32,
            _: gdk::ModifierType,
            _: &gtk::EventControllerKey,
        ) -> glib::Propagation {
            if key == gdk::Key::Escape {
                self.obj().enable_search_mode(false);
            } else if key == gdk::Key::KP_Enter {
                self.obj().activate_action(ACTION_SELECT, None).unwrap();
            }

            glib::Propagation::Proceed
        }

        #[template_callback]
        fn on_signal_list_item_factory_setup(&self, list_item: &gtk::ListItem) {
            let label = gtk::Label::builder()
                .margin_top(9)
                .margin_end(12)
                .margin_bottom(9)
                .margin_start(12)
                .xalign(0.0)
                .wrap(true)
                .wrap_mode(pango::WrapMode::WordChar)
                .build();

            list_item.set_child(Some(&label));
        }

        #[template_callback]
        fn on_signal_list_item_factory_bind(&self, list_item: &gtk::ListItem) {
            let pod = list_item.item().and_downcast::<model::Pod>().unwrap();

            list_item
                .child()
                .and_downcast::<gtk::Label>()
                .unwrap()
                .set_label(&pod.name());
        }

        #[template_callback]
        fn on_pod_selected(&self) {
            self.obj()
                .action_set_enabled(ACTION_SELECT, self.selection.selected_item().is_some());
        }

        #[template_callback]
        fn on_pod_activated(&self, _: u32) {
            self.obj().activate_action(ACTION_SELECT, None).unwrap();
        }

        pub(super) fn set_pod_list(&self, value: Option<&model::PodList>) {
            let obj = &*self.obj();
            if obj.pod_list().as_ref() == value {
                return;
            }

            if let Some(pod_list) = value {
                let model = gtk::FilterListModel::new(
                    Some(pod_list.to_owned()),
                    self.filter.get().cloned(),
                );

                let model = gtk::SortListModel::new(
                    Some(model),
                    Some(gtk::StringSorter::new(Some(model::Pod::this_expression(
                        "name",
                    )))),
                );

                let model = gtk::SingleSelection::new(Some(model));

                model.connect_selected_item_notify(clone!(@weak obj => move |selection| {
                    obj.action_set_enabled(ACTION_SELECT, selection.selected_item().is_some());
                }));

                self.selection.set_model(Some(&model));

                obj.action_set_enabled(ACTION_SELECT, self.selection.selected_item().is_some());
            }

            self.pod_list.set(value);
        }

        pub(super) fn update_filter(&self, change: gtk::FilterChange) {
            self.filter.get().unwrap().changed(change);
        }
    }
}

glib::wrapper! {
    pub(crate) struct PodSelectionPage(ObjectSubclass<imp::PodSelectionPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::PodList> for PodSelectionPage {
    fn from(pod_list: &model::PodList) -> Self {
        glib::Object::builder()
            .property("pod-list", pod_list)
            .build()
    }
}

impl PodSelectionPage {
    pub(crate) fn enable_search_mode(&self, enable: bool) {
        let imp = self.imp();

        if !enable && !imp.filter_button.is_active() {
            utils::navigation_view(self.upcast_ref()).pop();
        } else {
            imp.filter_button.set_active(enable);
            if !enable {
                imp.update_filter(gtk::FilterChange::LessStrict);
            }
        }
    }

    pub(crate) fn selected_pod(&self) -> Option<model::Pod> {
        self.imp()
            .selection
            .selected_item()
            .and_then(|item| item.downcast().ok())
    }

    pub(crate) fn select(&self) {
        if let Some(pod) = self.selected_pod() {
            self.emit_by_name::<()>("pod-selected", &[&pod]);

            utils::navigation_view(self.upcast_ref()).pop();
        }
    }

    pub(crate) fn connect_pod_selected<F: Fn(&Self, model::Pod) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("pod-selected", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let pod = values[1].get::<model::Pod>().unwrap();
            f(&obj, pod);

            None
        })
    }
}
