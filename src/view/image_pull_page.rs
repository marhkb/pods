use adw::subclass::prelude::*;
use adw::traits::BinExt;
use glib::clone;
use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::CompositeTemplate;

use crate::model;
use crate::podman;
use crate::utils;
use crate::view;

const ACTION_PULL: &str = "image-pull-page.pull";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ImagePullPage)]
    #[template(file = "image_pull_page.ui")]
    pub(crate) struct ImagePullPage {
        #[property(get, set, construct_only, nullable)]
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
    impl ObjectSubclass for ImagePullPage {
        const NAME: &'static str = "PdsImagePullPage";
        type Type = super::ImagePullPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_PULL, None, |widget, _, _| {
                widget.pull();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImagePullPage {
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
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for ImagePullPage {
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
    pub(crate) struct ImagePullPage(ObjectSubclass<imp::ImagePullPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Client> for ImagePullPage {
    fn from(client: &model::Client) -> Self {
        glib::Object::builder().property("client", client).build()
    }
}

impl ImagePullPage {
    pub(crate) fn pull(&self) {
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
