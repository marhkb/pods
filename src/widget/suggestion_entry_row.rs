use std::cell::OnceCell;
use std::marker::PhantomData;
use std::sync::OnceLock;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use glib::clone;
use glib::subclass::Signal;
use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::gio;
use gtk::glib;

use crate::model;
use crate::utils;
use crate::widget;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "SuggestionEntryVisibleStackPage")]
pub(crate) enum SuggestionEntryVisibleStackPage {
    #[default]
    Searching,
    Results,
}

mod imp {
    use super::*;

    #[derive(Default)]
    pub(crate) struct SuggestionRowLayout;

    #[glib::object_subclass]
    impl ObjectSubclass for SuggestionRowLayout {
        const NAME: &'static str = "PdsSuggestionRowLayout";
        type Type = SuggestionRowLayoutWrapper;
        type ParentType = gtk::LayoutManager;
    }

    impl ObjectImpl for SuggestionRowLayout {}

    impl LayoutManagerImpl for SuggestionRowLayout {
        fn measure(
            &self,
            widget: &gtk::Widget,
            orientation: gtk::Orientation,
            for_size: i32,
        ) -> (i32, i32, i32, i32) {
            utils::ChildIter::from(widget)
                .filter(|child| child.downcast_ref::<gtk::Popover>().is_none())
                .fold(
                    (-1, -1, -1, -1),
                    |(max_min, max_nat, max_min_base, max_nat_base), child| {
                        let (min, nat, min_base, nat_base) = child.measure(orientation, for_size);
                        (
                            max_min.max(min),
                            max_nat.max(nat),
                            max_min_base.max(min_base),
                            max_nat_base.max(nat_base),
                        )
                    },
                )
        }

        fn allocate(&self, widget: &gtk::Widget, width: i32, height: i32, baseline: i32) {
            utils::ChildIter::from(widget)
                .filter(|child| child.downcast_ref::<gtk::Popover>().is_none())
                .for_each(|child| child.allocate(width, height, baseline, None));

            let popover = &*widget
                .downcast_ref::<super::SuggestionEntryRow>()
                .unwrap()
                .imp()
                .popover;

            popover.set_width_request(width + 6);
            popover.present();
        }
    }

    glib::wrapper! {
        pub(crate) struct SuggestionRowLayoutWrapper(ObjectSubclass<SuggestionRowLayout>)
            @extends gtk::LayoutManager;
    }

    #[derive(Default, CompositeTemplate, Properties)]
    #[properties(wrapper_type = super::SuggestionEntryRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/suggestion_entry_row.ui")]
    pub(crate) struct SuggestionEntryRow {
        pub(super) changed_handler_id: OnceCell<glib::SignalHandlerId>,

        #[property(get = Self::model, set = Self::set_model, nullable, explicit_notify)]
        _model: PhantomData<Option<gio::ListModel>>,
        #[property(get = Self::factory, set = Self::set_factory, nullable, explicit_notify)]
        _factory: PhantomData<Option<gtk::ListItemFactory>>,
        #[property(get = Self::visible_stack_page, set = Self::set_visible_stack_page, explicit_notify, default)]
        _visible_stack_page: PhantomData<widget::SuggestionEntryVisibleStackPage>,

        #[template_child]
        pub(super) popover: TemplateChild<gtk::Popover>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) list_view: TemplateChild<gtk::ListView>,
        #[template_child]
        pub(super) selection: TemplateChild<gtk::SingleSelection>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SuggestionEntryRow {
        const NAME: &'static str = "PdsSuggestionEntryRow";
        type Type = super::SuggestionEntryRow;
        type ParentType = adw::EntryRow;
        type Interfaces = (gtk::Editable,);

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
            klass.set_layout_manager_type::<SuggestionRowLayoutWrapper>();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SuggestionEntryRow {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| vec![Signal::builder("changed-by-typing").build()])
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

            self.changed_handler_id
                .set(obj.connect_changed(clone!(
                    #[weak]
                    obj,
                    move |_| {
                        obj.imp().popover.popup();
                        obj.emit_by_name::<()>("changed-by-typing", &[]);
                    }
                )))
                .unwrap();

            self.popover.set_parent(obj);
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for SuggestionEntryRow {}
    impl ListBoxRowImpl for SuggestionEntryRow {}
    impl PreferencesRowImpl for SuggestionEntryRow {}
    impl EntryRowImpl for SuggestionEntryRow {}
    impl EditableImpl for SuggestionEntryRow {}

    #[gtk::template_callbacks]
    impl SuggestionEntryRow {
        fn model(&self) -> Option<gio::ListModel> {
            self.selection.model()
        }

        fn set_model(&self, model: Option<&gio::ListModel>) {
            self.selection.set_model(model);
        }

        fn factory(&self) -> Option<gtk::ListItemFactory> {
            self.list_view.factory()
        }

        fn set_factory(&self, factory: Option<&gtk::ListItemFactory>) {
            self.list_view.set_factory(factory);
        }

        fn visible_stack_page(&self) -> widget::SuggestionEntryVisibleStackPage {
            match self.stack.visible_child_name().unwrap_or_default().as_str() {
                "searching" => widget::SuggestionEntryVisibleStackPage::Searching,
                "results" => widget::SuggestionEntryVisibleStackPage::Results,
                _ => unreachable!(),
            }
        }

        fn set_visible_stack_page(&self, value: widget::SuggestionEntryVisibleStackPage) {
            self.stack.set_visible_child_name(match value {
                widget::SuggestionEntryVisibleStackPage::Searching => "searching",
                widget::SuggestionEntryVisibleStackPage::Results => "results",
            });
        }

        #[template_callback]
        fn on_stack_notify_visible_child_name(&self) {
            self.obj().notify_visible_stack_page();
        }

        #[template_callback]
        fn on_selection_notify_model(&self) {
            self.obj().notify_model();
        }

        #[template_callback]
        fn on_list_view_notify_factory(&self) {
            self.obj().notify_factory();
        }

        #[template_callback]
        fn on_list_view_key_pressed(
            &self,
            key: gdk::Key,
            _: u32,
            _: gdk::ModifierType,
            _: &gtk::EventControllerKey,
        ) -> glib::Propagation {
            let obj = &*self.obj();

            if self.selection.selected() == 0 && key == gdk::Key::Up {
                obj.grab_focus();
                obj.select_region(0, -1);
                self.selection.unselect_all();

                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        }

        #[template_callback]
        fn on_selection_changed(&self) {
            let Some(item) = self
                .selection
                .selected_item()
                .and_then(|item| item.downcast::<model::SuggestionItem>().ok())
            else {
                return;
            };

            let obj = &*self.obj();

            let changed_handler_id = self.changed_handler_id.get().unwrap();

            obj.block_signal(changed_handler_id);
            obj.set_text(
                &item
                    .suggestion_postfix()
                    .map(|postfix| format!("{}{}", item.name(), postfix))
                    .unwrap_or_else(|| item.name()),
            );
            obj.unblock_signal(changed_handler_id);
        }

        #[template_callback]
        fn on_selection_activated(&self, _: u32) {
            self.popover.popdown();

            let obj = &*self.obj();
            glib::idle_add_local_once(clone!(
                #[weak]
                obj,
                move || obj.set_position(-1)
            ));
        }

        #[template_callback]
        fn on_key_pressed(
            &self,
            key: gdk::Key,
            _: u32,
            _: gdk::ModifierType,
            _: &gtk::EventControllerKey,
        ) -> glib::Propagation {
            if key == gdk::Key::Down {
                self.list_view
                    .scroll_to(0, gtk::ListScrollFlags::SELECT, None);
                self.list_view.grab_focus();
                self.selection.select_item(0, true);

                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct SuggestionEntryRow(ObjectSubclass<imp::SuggestionEntryRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::EntryRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget, gtk::Editable;
}

impl Default for SuggestionEntryRow {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl SuggestionEntryRow {
    pub(crate) fn popdown(&self) {
        self.imp().popover.popdown();
    }
}

unsafe impl<T: ObjectSubclass + ObjectImpl + WidgetImpl> IsSubclassable<T> for SuggestionEntryRow {}
