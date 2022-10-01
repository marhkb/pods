use adw::subclass::prelude::*;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::podman;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/image/pull-page.ui")]
    pub(crate) struct PullPage {
        pub(super) client: WeakRef<model::Client>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) image_search_widget: TemplateChild<view::ImageSearchWidget>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PullPage {
        const NAME: &'static str = "PdsImagePullPage";
        type Type = super::PullPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action(
                view::ImageSearchWidget::action_select(),
                None,
                |widget, _, _| {
                    widget.pull();
                },
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PullPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "client",
                    "Client",
                    "The client of this image pull page",
                    model::Client::static_type(),
                    glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            _obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "client" => self.client.set(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "client" => obj.client().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            obj.action_set_enabled(view::ImageSearchWidget::action_select(), false);
            self.image_search_widget.connect_notify_local(
                Some("selected-image"),
                clone!(@weak obj => move |widget, _| {
                    obj.action_set_enabled(view::ImageSearchWidget::action_select(), widget.selected_image().is_some());
                }),
            );
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.stack.unparent();
        }
    }

    impl WidgetImpl for PullPage {}
}

glib::wrapper! {
    pub(crate) struct PullPage(ObjectSubclass<imp::PullPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<Option<&model::Client>> for PullPage {
    fn from(client: Option<&model::Client>) -> Self {
        glib::Object::new(&[("client", &client)]).expect("Failed to create PdsImagePullPage")
    }
}

impl PullPage {
    fn client(&self) -> Option<model::Client> {
        self.imp().client.upgrade()
    }

    fn pull(&self) {
        let imp = self.imp();

        if let Some(search_response) = imp.image_search_widget.selected_image() {
            let reference = format!(
                "{}:{}",
                search_response.name().unwrap(),
                imp.image_search_widget.tag(),
            );
            let opts = podman::opts::PullOpts::builder()
                .reference(&reference)
                .quiet(false)
                .build();

            let page = view::ActionPage::from(
                &self
                    .client()
                    .unwrap()
                    .action_list()
                    .download_image(&reference, opts),
            );

            imp.stack.add_child(&page);
            imp.stack.set_visible_child(&page);
        }
    }
}
