use gtk::glib::{closure, WeakRef};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};
use once_cell::sync::Lazy;

use crate::{model, view};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Symphony/ui/image-used-by-row.ui")]
    pub(crate) struct ImageUsedByRow {
        pub(super) container_list: WeakRef<model::SimpleContainerList>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ImageUsedByRow {
        const NAME: &'static str = "ImageUsedByRow";
        type Type = super::ImageUsedByRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ImageUsedByRow {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "container-list",
                    "Container List",
                    "The list of containers associated with this Image",
                    model::SimpleContainerList::static_type(),
                    glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn set_property(
            &self,
            obj: &Self::Type,
            _id: usize,
            value: &glib::Value,
            pspec: &glib::ParamSpec,
        ) {
            match pspec.name() {
                "container-list" => obj.set_container_list(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "container-list" => obj.container_list().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            Self::Type::this_expression("container-list")
                .chain_property::<model::SimpleContainerList>("len")
                .chain_closure::<String>(closure!(|_: glib::Object, len: u32| {
                    if len == 0 {
                        "unused"
                    } else {
                        "containers"
                    }
                }))
                .bind(&*self.stack, "visible-child-name", Some(obj));
        }
    }

    impl WidgetImpl for ImageUsedByRow {}
    impl ListBoxRowImpl for ImageUsedByRow {}
}

glib::wrapper! {
    pub(crate) struct ImageUsedByRow(ObjectSubclass<imp::ImageUsedByRow>)
        @extends gtk::Widget, gtk::ListBoxRow;
}

impl Default for ImageUsedByRow {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create ImageUsedByRow")
    }
}

impl ImageUsedByRow {
    pub(crate) fn container_list(&self) -> Option<model::SimpleContainerList> {
        self.imp().container_list.upgrade()
    }

    pub(crate) fn set_container_list(&self, value: Option<&model::SimpleContainerList>) {
        if self.container_list().as_ref() == value {
            return;
        }

        self.imp().list_box.bind_model(
            value
                .map(|value| {
                    gtk::SortListModel::new(
                        Some(value),
                        Some(&gtk::CustomSorter::new(|obj1, obj2| {
                            let container1 = obj1.downcast_ref::<model::Container>().unwrap();
                            let container2 = obj2.downcast_ref::<model::Container>().unwrap();

                            if container1.name().is_none() {
                                if container2.name().is_some() {
                                    gtk::Ordering::Larger
                                } else {
                                    gtk::Ordering::Equal
                                }
                            } else if container2.name().is_none() {
                                gtk::Ordering::Smaller
                            } else {
                                container1.name().cmp(&container2.name()).into()
                            }
                        })),
                    )
                })
                .as_ref(),
            |obj| {
                gtk::ListBoxRow::builder()
                    .activatable(false)
                    .selectable(false)
                    .child(&view::ContainerRowSimple::from(obj.downcast_ref()))
                    .build()
                    .upcast()
            },
        );

        self.imp().container_list.set(value);
        self.notify("container-list");
    }
}
