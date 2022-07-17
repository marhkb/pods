use futures::StreamExt;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::WeakRef;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::api;
use crate::model;
use crate::utils;
use crate::view;
use crate::window::Window;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/container-processes-page.ui")]
    pub(crate) struct ContainerProcessesPage {
        pub(super) container: WeakRef<model::Container>,
        pub(super) tree_store: OnceCell<gtk::TreeStore>,
        #[template_child]
        pub(super) tree_view: TemplateChild<gtk::TreeView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerProcessesPage {
        const NAME: &'static str = "ContainerProcessesPage";
        type Type = super::ContainerProcessesPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.install_action("navigation.go-first", None, move |widget, _, _| {
                widget.navigate_to_first();
            });
            klass.install_action("navigation.back", None, move |widget, _, _| {
                widget.navigate_back();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerProcessesPage {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "container",
                    "Container",
                    "The container of this ContainerProcessesPage",
                    model::Container::static_type(),
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
                "container" => self.container.set(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "container" => obj.container().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);
            obj.connect_top_stream();
        }

        fn dispose(&self, obj: &Self::Type) {
            let mut child = obj.first_child();
            while let Some(child_) = child {
                child = child_.next_sibling();
                child_.unparent();
            }
        }
    }

    impl WidgetImpl for ContainerProcessesPage {}
}

glib::wrapper! {
    pub(crate) struct ContainerProcessesPage(ObjectSubclass<imp::ContainerProcessesPage>)
        @extends gtk::Widget;
}

impl From<&model::Container> for ContainerProcessesPage {
    fn from(image: &model::Container) -> Self {
        glib::Object::new(&[("container", image)]).expect("Failed to create ContainerProcessesPage")
    }
}

impl ContainerProcessesPage {
    fn container(&self) -> Option<model::Container> {
        self.imp().container.upgrade()
    }

    fn connect_top_stream(&self) {
        if let Some(container) = self
            .container()
            .as_ref()
            .and_then(model::Container::api_container)
        {
            utils::run_stream(
                container,
                move |container| {
                    container
                        .top_stream(&api::ContainerTopOpts::builder().delay(2).build())
                        .boxed()
                },
                clone!(@weak self as obj => @default-return glib::Continue(false), move |result: api::Result<api::ContainerTopOkBody>| {
                    glib::Continue(match result {
                        Ok(top) => {
                            let imp = obj.imp();
                            let tree_store = imp.tree_store.get_or_init(|| {
                                let tree_store = gtk::TreeStore::new(
                                    &top.titles
                                        .iter()
                                        .map(|_| String::static_type())
                                        .collect::<Vec<_>>(),
                                );
                                imp.tree_view.set_model(Some(&tree_store));

                                top.titles.iter().enumerate().for_each(|(i, title)| {
                                    let column = gtk::TreeViewColumn::with_attributes(
                                        title,
                                        &gtk::CellRendererText::new(),
                                        &[("text", i as i32)],
                                    );
                                    column.set_sort_column_id(i as i32);
                                    column.set_sizing(gtk::TreeViewColumnSizing::GrowOnly);
                                    imp.tree_view.append_column(&column);
                                });

                                tree_store
                            });

                            // Remove processes that have disappeared.
                            tree_store.foreach(|_, _, iter| {
                                if !top
                                    .processes
                                    .iter()
                                    .any(|process| process[1] == tree_store.get::<String>(iter, 1))
                                {
                                    tree_store.remove(iter);
                                }
                                false
                            });

                            // Replace and add processes.
                            top.processes.iter().for_each(|process| {
                                let row = process.iter()
                                    .enumerate()
                                    .map(|(i, v)| (i as u32, v as &dyn gtk::prelude::ToValue))
                                    .collect::<Vec<_>>();

                                let mut replaced = false;

                                tree_store.foreach(|_, _, iter| {
                                    if process[1] == tree_store.get::<String>(iter, 1) {
                                        tree_store.set(iter, row.as_slice());
                                        replaced = true;
                                        true
                                    } else {
                                        false
                                    }
                                });

                                if !replaced {
                                    tree_store.set(&tree_store.append(None), row.as_slice());
                                }
                            });

                            true
                        }
                        Err(e) => {
                            log::warn!("Stopping container top stream due to error: {e}");
                            false
                        }
                    })
                }),
            );
        }
    }

    fn navigate_to_first(&self) {
        self.root_leaflet_overlay().hide_details();
    }

    fn navigate_back(&self) {
        self.previous_leaflet_overlay().hide_details();
    }

    fn previous_leaflet_overlay(&self) -> view::LeafletOverlay {
        utils::find_parent_leaflet_overlay(self)
    }

    fn root_leaflet_overlay(&self) -> view::LeafletOverlay {
        self.root()
            .unwrap()
            .downcast::<Window>()
            .unwrap()
            .leaflet_overlay()
    }
}
