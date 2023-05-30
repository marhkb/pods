use adw::subclass::prelude::*;
use glib::clone;
use glib::subclass::Signal;
use glib::Properties;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy as SyncLazy;

use crate::model;
use crate::utils;
use crate::view;
use crate::widget;

const ACTION_EXIT: &str = "image-search-page.exit";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImageSearchPage)]
    #[template(file = "image_search_page.ui")]
    pub(crate) struct ImageSearchPage {
        #[property(get, set, construct_only, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[template_child]
        pub(super) header_bar: TemplateChild<adw::HeaderBar>,
        #[template_child]
        pub(super) back_navigation_controls: TemplateChild<widget::BackNavigationControls>,
        #[template_child]
        pub(super) image_search_widget: TemplateChild<view::ImageSearchWidget>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageSearchPage {
        const NAME: &'static str = "PdsImageSearchPage";
        type Type = super::ImageSearchPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_EXIT, None, |widget, _, _| {
                if !widget.imp().back_navigation_controls.navigate_back() {
                    utils::root(widget.upcast_ref()).close();
                }
            });

            klass.add_binding_action(
                gdk::Key::Escape,
                gdk::ModifierType::empty(),
                ACTION_EXIT,
                None,
            );

            klass.install_action(
                view::ImageSearchWidget::action_select(),
                None,
                |widget, _, _| {
                    widget.select();
                },
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImageSearchPage {
        fn signals() -> &'static [Signal] {
            static SIGNALS: SyncLazy<Vec<Signal>> = SyncLazy::new(|| {
                vec![Signal::builder("image-selected")
                    .param_types([String::static_type()])
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

            obj.action_set_enabled(view::ImageSearchWidget::action_select(), false);
            self.image_search_widget.connect_notify_local(
                Some("selected-image"),
                clone!(@weak obj => move |widget, _| {
                    obj.action_set_enabled(view::ImageSearchWidget::action_select(), widget.selected_image().is_some());
                }),
            );
        }

        fn dispose(&self) {
            self.header_bar.unparent();
            self.image_search_widget.unparent();
        }
    }

    impl WidgetImpl for ImageSearchPage {}
}

glib::wrapper! {
    pub(crate) struct ImageSearchPage(ObjectSubclass<imp::ImageSearchPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Client> for ImageSearchPage {
    fn from(client: &model::Client) -> Self {
        glib::Object::builder().property("client", client).build()
    }
}

impl ImageSearchPage {
    fn select(&self) {
        let imp = self.imp();

        if let Some(search_response) = imp.image_search_widget.selected_image() {
            let image = format!(
                "{}:{}",
                search_response.name().unwrap(),
                imp.image_search_widget.tag(),
            );

            self.emit_by_name::<()>("image-selected", &[&image]);

            imp.back_navigation_controls.navigate_back();
        }
    }

    pub(crate) fn connect_image_selected<F: Fn(&Self, String) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("image-selected", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            let image = values[1].get::<String>().unwrap();
            f(&obj, image);

            None
        })
    }
}
