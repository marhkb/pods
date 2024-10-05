use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::podman;
use crate::utils;
use crate::view;
use crate::widget;

const ACTION_PRUNE: &str = "containers-prune-page.prune";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainersPrunePage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/containers_prune_page.ui")]
    pub(crate) struct ContainersPrunePage {
        #[property(get, set, construct_only, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[template_child]
        pub(super) navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub(super) prune_until_row: TemplateChild<widget::DateTimeRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainersPrunePage {
        const NAME: &'static str = "PdsContainersPrunePage";
        type Type = super::ContainersPrunePage;
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

    impl ObjectImpl for ContainersPrunePage {
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

    impl WidgetImpl for ContainersPrunePage {}
}

glib::wrapper! {
    pub(crate) struct ContainersPrunePage(ObjectSubclass<imp::ContainersPrunePage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Client> for ContainersPrunePage {
    fn from(client: &model::Client) -> Self {
        glib::Object::builder().property("client", client).build()
    }
}

impl ContainersPrunePage {
    fn prune(&self) {
        let imp = self.imp();

        let action = self.client().unwrap().action_list().prune_containers(
            podman::opts::ContainerPruneOpts::builder()
                .filter(if imp.prune_until_row.enables_expansion() {
                    Some(podman::opts::ContainerPruneFilter::Until(
                        imp.prune_until_row.prune_until_timestamp().to_string(),
                    ))
                } else {
                    None
                })
                .build(),
        );

        let page = view::ActionPage::from(&action);

        imp.navigation_view.push(
            &adw::NavigationPage::builder()
                .can_pop(false)
                .child(&page)
                .build(),
        );
    }
}
