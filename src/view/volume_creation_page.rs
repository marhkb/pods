use std::cell::Cell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::clone;
use glib::Properties;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::podman;
use crate::utils;
use crate::view;
use crate::widget;

const ACTION_CREATE_VOLUME: &str = "volume-creation-page.create-volume";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::VolumeCreationPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/volume_creation_page.ui")]
    pub(crate) struct VolumeCreationPage {
        #[property(get, set, construct_only)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[property(get, set, construct_only)]
        pub(super) show_view_artifact: Cell<bool>,
        #[template_child]
        pub(super) navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub(super) create_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) name_entry_row: TemplateChild<widget::RandomNameEntryRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VolumeCreationPage {
        const NAME: &'static str = "PdsVolumeCreationPage";
        type Type = super::VolumeCreationPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_CREATE_VOLUME, None, |widget, _, _| {
                widget.create_volume();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for VolumeCreationPage {
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

    impl WidgetImpl for VolumeCreationPage {
        fn root(&self) {
            self.parent_root();

            let widget = &*self.obj();

            glib::idle_add_local(
                clone!(@weak widget => @default-return glib::ControlFlow::Break, move || {
                    widget.imp().name_entry_row.grab_focus();
                    glib::ControlFlow::Break
                }),
            );
            utils::root(widget.upcast_ref()).set_default_widget(Some(&*self.create_button));
        }

        fn unroot(&self) {
            utils::root(self.obj().upcast_ref()).set_default_widget(gtk::Widget::NONE);
            self.parent_unroot()
        }
    }
}

glib::wrapper! {
    pub(crate) struct VolumeCreationPage(ObjectSubclass<imp::VolumeCreationPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Client> for VolumeCreationPage {
    fn from(client: &model::Client) -> Self {
        Self::new(client, true)
    }
}

impl VolumeCreationPage {
    pub(crate) fn new(client: &model::Client, show_view_artifact: bool) -> Self {
        glib::Object::builder()
            .property("client", client)
            .property("show-view-artifact", show_view_artifact)
            .build()
    }

    fn create_volume(&self) {
        if let Some(client) = self.client() {
            let imp = self.imp();

            let name = imp.name_entry_row.text();

            let page = view::ActionPage::new(
                &client.action_list().create_volume(
                    name.as_str(),
                    podman::opts::VolumeCreateOpts::builder()
                        .name(name.as_str())
                        .build(),
                ),
                self.show_view_artifact(),
            );

            imp.navigation_view.push(
                &adw::NavigationPage::builder()
                    .can_pop(false)
                    .child(&page)
                    .build(),
            );
        }
    }
}
