use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Symphony/ui/sidebar-row.ui")]
    pub(crate) struct SidebarRow {
        pub(super) icon_name: OnceCell<Option<String>>,
        pub(super) panel_name: OnceCell<String>,
        pub(super) panel_title: OnceCell<Option<String>>,
        #[template_child]
        pub(super) content_box: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SidebarRow {
        const NAME: &'static str = "SidebarRow";
        type Type = super::SidebarRow;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SidebarRow {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecString::new(
                        "icon-name",
                        "Icon Name",
                        "The icon name",
                        Option::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecString::new(
                        "panel-name",
                        "Panel Name",
                        "The panel name",
                        Option::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                    glib::ParamSpecString::new(
                        "panel-title",
                        "Panel Title",
                        "The panel title",
                        Option::default(),
                        glib::ParamFlags::READWRITE | glib::ParamFlags::CONSTRUCT_ONLY,
                    ),
                ]
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
                "icon-name" => self.icon_name.set(value.get().unwrap()).unwrap(),
                "panel-name" => self.panel_name.set(value.get().unwrap()).unwrap(),
                "panel-title" => self.panel_title.set(value.get().unwrap()).unwrap(),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "icon-name" => obj.icon_name().to_value(),
                "panel-name" => obj.panel_name().to_value(),
                "panel-title" => obj.panel_title().to_value(),
                _ => unimplemented!(),
            }
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.content_box.unparent();
        }
    }

    impl WidgetImpl for SidebarRow {}
}

glib::wrapper! {
    pub(crate) struct SidebarRow(ObjectSubclass<imp::SidebarRow>)
        @extends gtk::Widget;
}

impl Default for SidebarRow {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create SidebarRow")
    }
}

impl SidebarRow {
    pub(crate) fn new(
        icon_name: Option<&str>,
        panel_name: &str,
        panel_title: Option<&str>,
    ) -> Self {
        glib::Object::new(&[
            ("icon-name", &icon_name),
            ("panel-name", &panel_name),
            ("panel-title", &panel_title),
        ])
        .expect("Failed to create SidebarRow")
    }

    pub(crate) fn icon_name(&self) -> Option<&str> {
        self.imp().icon_name.get().and_then(Option::as_deref)
    }

    pub(crate) fn panel_name(&self) -> &str {
        self.imp().panel_name.get().unwrap()
    }

    pub(crate) fn panel_title(&self) -> Option<&str> {
        self.imp().panel_title.get().and_then(Option::as_deref)
    }
}
