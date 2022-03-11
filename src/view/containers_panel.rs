use gettextrs::gettext;
use gtk::glib::closure;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{glib, CompositeTemplate};
use once_cell::sync::Lazy;
use once_cell::unsync::OnceCell;

use crate::{model, view};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Symphony/ui/containers-panel.ui")]
    pub(crate) struct ContainersPanel {
        pub(super) container_list: OnceCell<model::ContainerList>,
        #[template_child]
        pub(super) overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub(super) progress_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) progress_bar: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub(super) spinner: TemplateChild<gtk::Spinner>,
        #[template_child]
        pub(super) preferences_page: TemplateChild<adw::PreferencesPage>,
        #[template_child]
        pub(super) container_group: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub(super) list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainersPanel {
        const NAME: &'static str = "ContainersPanel";
        type Type = super::ContainersPanel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainersPanel {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![glib::ParamSpecObject::new(
                    "container-list",
                    "Container List",
                    "The list of containers",
                    model::ContainerList::static_type(),
                    glib::ParamFlags::READABLE,
                )]
            });
            PROPERTIES.as_ref()
        }

        fn property(&self, obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "container-list" => obj.container_list().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let container_list_expr = Self::Type::this_expression("container-list");

            let fetched_params = &[
                container_list_expr
                    .chain_property::<model::ContainerList>("fetched")
                    .upcast(),
                container_list_expr
                    .chain_property::<model::ContainerList>("to-fetch")
                    .upcast(),
            ];

            gtk::ClosureExpression::new::<f64, _, _>(
                fetched_params,
                closure!(|_: glib::Object, fetched: u32, to_fetch: u32| {
                    fetched as f64 / to_fetch as f64
                }),
            )
            .bind(&*self.progress_bar, "fraction", Some(obj));

            gtk::ClosureExpression::new::<String, _, _>(
                fetched_params,
                closure!(|_: glib::Object, fetched: u32, to_fetch: u32| {
                    if fetched == to_fetch {
                        "empty"
                    } else {
                        "bar"
                    }
                }),
            )
            .bind(&*self.progress_stack, "visible-child-name", Some(obj));

            gtk::Stack::this_expression("visible-child-name")
                .chain_closure::<u32>(closure!(|_: glib::Object, name: &str| {
                    match name {
                        "empty" => 0_u32,
                        "bar" => 1000,
                        _ => unreachable!(),
                    }
                }))
                .bind(
                    &*self.progress_stack,
                    "transition-duration",
                    Some(&*self.progress_stack),
                );

            let container_len_expr =
                container_list_expr.chain_property::<model::ContainerList>("len");

            container_len_expr
                .chain_closure::<bool>(closure!(|_: glib::Object, len: u32| { len == 0 }))
                .bind(&*self.spinner, "visible", Some(obj));

            container_len_expr
                .chain_closure::<bool>(closure!(|_: glib::Object, len: u32| { len > 0 }))
                .bind(&*self.preferences_page, "visible", Some(obj));

            gtk::ClosureExpression::new::<f64, _, _>(
                fetched_params,
                closure!(|_: glib::Object, fetched: u32, to_fetch: u32| {
                    fetched as f64 / to_fetch as f64
                }),
            )
            .bind(&*self.container_group, "description", Some(obj));

            gtk::ClosureExpression::new::<Option<String>, _, _>(
                [
                    &fetched_params[0],
                    &fetched_params[1],
                    &container_list_expr
                        .chain_property::<model::ContainerList>("len")
                        .upcast(),
                ],
                closure!(|_: glib::Object, fetched: u32, to_fetch: u32, len: u32| {
                    if fetched == to_fetch {
                        Some(gettext!("{} Containers total", len))
                    } else {
                        None
                    }
                }),
            )
            .bind(&*self.container_group, "description", Some(obj));

            self.list_box
                .bind_model(Some(obj.container_list()), |item| {
                    view::ContainerRow::from(item.downcast_ref().unwrap()).upcast()
                })
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.overlay.unparent();
        }
    }
    impl WidgetImpl for ContainersPanel {}
}

glib::wrapper! {
    pub(crate) struct ContainersPanel(ObjectSubclass<imp::ContainersPanel>)
        @extends gtk::Widget;
}

impl Default for ContainersPanel {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create ContainersPanel")
    }
}

impl ContainersPanel {
    pub(crate) fn container_list(&self) -> &model::ContainerList {
        self.imp()
            .container_list
            .get_or_init(model::ContainerList::default)
    }
}
