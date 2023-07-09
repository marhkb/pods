use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::once_cell::sync::Lazy as SyncLazy;
use glib::subclass::Signal;
use glib::Properties;
use gtk::gdk;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_EXIT: &str = "image-search-page.exit";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImageSearchPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/image_search_page.ui")]
    pub(crate) struct ImageSearchPage {
        #[property(get, set, construct_only, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[template_child]
        pub(super) header_bar: TemplateChild<adw::HeaderBar>,
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
            klass.bind_template_callbacks();

            klass.install_action(ACTION_EXIT, None, |widget, _, _| {
                if !utils::navigation_view(widget.upcast_ref()).pop() {
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
            self.obj()
                .action_set_enabled(view::ImageSearchWidget::action_select(), false);
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for ImageSearchPage {}

    #[gtk::template_callbacks]
    impl ImageSearchPage {
        #[template_callback]
        fn on_image_search_widget_notify_selected_image(&self) {
            self.obj().action_set_enabled(
                view::ImageSearchWidget::action_select(),
                self.image_search_widget.selected_image().is_some(),
            );
        }
    }
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

            utils::navigation_view(self.upcast_ref()).pop();
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
