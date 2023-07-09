use adw::subclass::prelude::*;
use adw::traits::ExpanderRowExt;
use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::CompositeTemplate;

use crate::model;
use crate::podman;
use crate::utils;
use crate::view;
use crate::widget;

const ACTION_PRUNE: &str = "volumes-prune-page.prune";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::VolumesPrunePage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/volumes_prune_page.ui")]
    pub(crate) struct VolumesPrunePage {
        #[property(get, set, construct_only, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[template_child]
        pub(super) navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub(super) prune_until_row: TemplateChild<widget::DateTimeRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VolumesPrunePage {
        const NAME: &'static str = "PdsVolumesPrunePage";
        type Type = super::VolumesPrunePage;
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

    impl ObjectImpl for VolumesPrunePage {
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
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for VolumesPrunePage {}
}

glib::wrapper! {
    pub(crate) struct VolumesPrunePage(ObjectSubclass<imp::VolumesPrunePage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Client> for VolumesPrunePage {
    fn from(client: &model::Client) -> Self {
        glib::Object::builder().property("client", client).build()
    }
}

impl VolumesPrunePage {
    pub(crate) fn prune(&self) {
        let imp = self.imp();

        let action = self.client().unwrap().action_list().prune_volumes(
            podman::opts::VolumePruneOpts::builder()
                .filter(if imp.prune_until_row.enables_expansion() {
                    Some(podman::opts::VolumePruneFilter::Until(
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
