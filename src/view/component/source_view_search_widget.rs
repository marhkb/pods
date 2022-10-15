use std::cell::RefCell;

use gettextrs::gettext;
use gtk::gdk;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;
use sourceview5::traits::SearchSettingsExt;

use crate::utils;
use crate::view;

const ACTION_SEARCH_BACKWARDS: &str = "source-view-search-widget.search-backward";
const ACTION_SEARCH_FORWARD: &str = "source-view-search-widget.search-forward";

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/component/source-view-search-widget.ui")]
    pub(crate) struct SourceViewSearchWidget {
        pub(super) search_settings: sourceview5::SearchSettings,
        pub(super) search_context: RefCell<Option<sourceview5::SearchContext>>,
        pub(super) search_iters: RefCell<Option<(gtk::TextIter, gtk::TextIter)>>,
        pub(super) source_view: glib::WeakRef<sourceview5::View>,
        #[template_child]
        pub(super) search_entry: TemplateChild<view::TextSearchEntry>,
        #[template_child]
        pub(super) options_toggle_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) regex_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) case_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) word_button: TemplateChild<gtk::CheckButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SourceViewSearchWidget {
        const NAME: &'static str = "PdsSourceViewSearchWidget";
        type Type = super::SourceViewSearchWidget;
        type ParentType = gtk::Widget;
        type Interfaces = (gtk::Editable,);

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.add_binding_action(
                gdk::Key::G,
                gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK,
                ACTION_SEARCH_BACKWARDS,
                None,
            );
            klass.install_action(ACTION_SEARCH_BACKWARDS, None, |widget, _, _| {
                widget.search_backward();
            });

            klass.add_binding_action(
                gdk::Key::G,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_SEARCH_FORWARD,
                None,
            );
            klass.install_action(ACTION_SEARCH_FORWARD, None, |widget, _, _| {
                widget.search_forward();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SourceViewSearchWidget {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "source-view",
                    "Source View",
                    "The source view",
                    sourceview5::View::static_type(),
                    glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "source-view" => obj.set_source_view(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "source-view" => obj.source_view().to_value(),
                other => self.search_entry.property(other),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            // Workaround for making the button non-flat.
            self.options_toggle_button.remove_css_class("image-button");

            self.search_entry
                .bind_property("text", &self.search_settings, "search-text")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();

            self.search_settings.set_wrap_around(true);

            self.regex_button
                .bind_property("active", &self.search_settings, "regex-enabled")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();

            self.case_button
                .bind_property("active", &self.search_settings, "case-sensitive")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();

            self.word_button
                .bind_property("active", &self.search_settings, "at-word-boundaries")
                .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                .build();
        }

        fn dispose(&self, obj: &Self::Type) {
            utils::ChildIter::from(obj).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for SourceViewSearchWidget {
        fn grab_focus(&self, _widget: &Self::Type) -> bool {
            self.search_entry.grab_focus()
        }
    }

    impl EditableImpl for SourceViewSearchWidget {
        fn delegate(&self, _editable: &Self::Type) -> Option<gtk::Editable> {
            Some(self.search_entry.clone().upcast())
        }
    }
}

glib::wrapper! {
    pub(crate) struct SourceViewSearchWidget(ObjectSubclass<imp::SourceViewSearchWidget>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Editable;
}

impl SourceViewSearchWidget {
    pub(crate) fn source_view(&self) -> Option<sourceview5::View> {
        self.imp().source_view.upgrade()
    }

    pub(crate) fn set_source_view(&self, value: Option<&sourceview5::View>) {
        if self.source_view().as_ref() == value {
            return;
        }

        let imp = self.imp();
        imp.source_view.set(value);

        if let Some(source_view) = value {
            let search_context = sourceview5::SearchContext::new(
                source_view
                    .buffer()
                    .downcast_ref::<sourceview5::Buffer>()
                    .unwrap(),
                Some(&imp.search_settings),
            );

            search_context.connect_occurrences_count_notify(
                clone!(@weak self as obj => move |ctx| {
                    obj.imp().search_entry.set_info(&
                        gettext!(
                            "0 of {}",
                            ctx.occurrences_count(),
                        ));
                }),
            );

            imp.search_context.replace(Some(search_context));
        }

        self.notify("source-view");
    }

    fn update_search_occurences(&self) {
        let imp = self.imp();

        if let Some(search_context) = imp.search_context.borrow().as_ref() {
            let count = search_context.occurrences_count();
            imp.search_entry.set_info(&if count > 0 {
                gettext!(
                    "{} of {}",
                    imp.search_iters
                        .borrow()
                        .as_ref()
                        .map(|(start_iter, end_iter)| search_context
                            .occurrence_position(start_iter, end_iter))
                        .unwrap_or(0),
                    count
                )
            } else {
                String::new()
            });
        }
    }

    pub(crate) fn search_backward(&self) {
        if let Some(source_view) = self.source_view() {
            let source_buffer = source_view.buffer();
            let imp = self.imp();

            let iter_at_cursor = source_buffer.iter_at_offset({
                let pos = source_buffer.cursor_position();
                if pos >= 0 {
                    pos
                } else {
                    i32::MAX
                }
            });

            imp.search_iters.replace_with(|iters| {
                match imp.search_context.borrow().as_ref().unwrap().backward(
                    &iters
                        .map(|(start_iter, end_iter)| {
                            if iter_at_cursor >= start_iter && iter_at_cursor <= end_iter {
                                start_iter
                            } else {
                                iter_at_cursor
                            }
                        })
                        .unwrap_or(iter_at_cursor),
                ) {
                    Some((mut start, end, _)) => {
                        source_view.scroll_to_iter(&mut start, 0.0, false, 0.0, 0.0);
                        source_buffer.place_cursor(&start);

                        Some((start, end))
                    }
                    None => None,
                }
            });

            self.update_search_occurences();
        }
    }

    pub(crate) fn search_forward(&self) {
        if let Some(source_view) = self.source_view() {
            let source_buffer = source_view.buffer();
            let imp = self.imp();

            let iter_at_cursor = source_buffer.iter_at_offset({
                let pos = source_buffer.cursor_position();
                if pos > 0 {
                    pos
                } else {
                    0
                }
            });

            imp.search_iters.replace_with(|iters| {
                match imp.search_context.borrow().as_ref().unwrap().forward(
                    &iters
                        .map(|(start_iter, end_iter)| {
                            if iter_at_cursor >= start_iter && iter_at_cursor <= end_iter {
                                end_iter
                            } else {
                                iter_at_cursor
                            }
                        })
                        .unwrap_or(iter_at_cursor),
                ) {
                    Some((start, mut end, _)) => {
                        source_view.scroll_to_iter(&mut end, 0.0, false, 0.0, 0.0);
                        source_buffer.place_cursor(&end);

                        Some((start, end))
                    }
                    None => None,
                }
            });

            self.update_search_occurences();
        }
    }
}
