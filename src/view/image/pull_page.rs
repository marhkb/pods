use adw::subclass::prelude::*;
use adw::traits::BinExt;
use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::podman;
use crate::utils;
use crate::view;

const ACTION_PULL: &str = "image-pull-page.pull";

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/image/pull-page.ui")]
    pub(crate) struct PullPage {
        pub(super) client: glib::WeakRef<model::Client>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) pull_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) image_search_widget: TemplateChild<view::ImageSearchWidget>,
        #[template_child]
        pub(super) action_page_bin: TemplateChild<adw::Bin>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PullPage {
        const NAME: &'static str = "PdsImagePullPage";
        type Type = super::PullPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action(ACTION_PULL, None, |widget, _, _| {
                widget.pull();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PullPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::builder::<model::Client>("client")
                    .construct_only()
                    .build()]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "client" => self.client.set(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "client" => self.obj().client().to_value(),
                _ => unimplemented!(),
            }
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
            utils::ChildIter::from(self.obj().upcast_ref()).for_each(|child| child.unparent());
        }
    }

    impl WidgetImpl for PullPage {
        fn root(&self) {
            self.parent_root();
            utils::root(self.obj().upcast_ref()).set_default_widget(Some(&*self.pull_button));
        }

        fn unroot(&self) {
            utils::root(self.obj().upcast_ref()).set_default_widget(gtk::Widget::NONE);
            self.parent_unroot()
        }
    }
}

glib::wrapper! {
    pub(crate) struct PullPage(ObjectSubclass<imp::PullPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Client> for PullPage {
    fn from(client: &model::Client) -> Self {
        glib::Object::builder().property("client", client).build()
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

            imp.action_page_bin.set_child(Some(&page));
            imp.stack.set_visible_child(&*imp.action_page_bin);
        }
    }
}
