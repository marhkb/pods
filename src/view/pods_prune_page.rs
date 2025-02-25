use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_PRUNE: &str = "pods-prune-page.prune";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::PodsPrunePage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/pods_prune_page.ui")]
    pub(crate) struct PodsPrunePage {
        #[property(get, set, construct_only, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[template_child]
        pub(super) navigation_view: TemplateChild<adw::NavigationView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PodsPrunePage {
        const NAME: &'static str = "PdsPodsPrunePage";
        type Type = super::PodsPrunePage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_PRUNE, None, |widget, _, _| {
                widget.prune();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PodsPrunePage {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for PodsPrunePage {}
}

glib::wrapper! {
    pub(crate) struct PodsPrunePage(ObjectSubclass<imp::PodsPrunePage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Client> for PodsPrunePage {
    fn from(client: &model::Client) -> Self {
        glib::Object::builder().property("client", client).build()
    }
}

impl PodsPrunePage {
    pub(crate) fn prune(&self) {
        let imp = self.imp();

        let action = self.client().unwrap().action_list().prune_pods();
        let page = view::ActionPage::from(&action);

        imp.navigation_view.push(
            &adw::NavigationPage::builder()
                .can_pop(false)
                .child(&page)
                .build(),
        );
    }
}
