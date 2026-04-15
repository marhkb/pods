use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use glib::clone;
use glib::closure;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::model;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ContainerVolumeRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/container_volume_row.ui")]
    pub(crate) struct ContainerVolumeRow {
        #[property(get, set, construct, nullable)]
        pub(super) container_volume: glib::WeakRef<model::ContainerVolume>,
        #[template_child]
        pub(super) name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) path_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) containers_count_bar: TemplateChild<view::ContainersCountBar>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerVolumeRow {
        const NAME: &'static str = "PdsContainerVolumeRow";
        type Type = super::ContainerVolumeRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("container-volume-row.show-details", None, |widget, _, _| {
                widget.show_details();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ContainerVolumeRow {
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

            let container_volume_expr = Self::Type::this_expression("container-volume");
            let volume_expr =
                container_volume_expr.chain_property::<model::ContainerVolume>("volume");
            let volume_name_expr = volume_expr.chain_property::<model::Volume>("name");
            let volume_name_is_id_expr =
                volume_name_expr.chain_closure::<bool>(closure!(|_: Self::Type, name: &str| {
                    utils::as_id(name).is_some()
                }));
            let volume_to_be_deleted_expr =
                volume_expr.chain_property::<model::Volume>("to-be-deleted");
            let container_list_expr = volume_expr.chain_property::<model::Volume>("container-list");

            gtk::ClosureExpression::new::<String>(
                [
                    gtk::ClosureExpression::new::<String>(
                        [
                            volume_name_is_id_expr.upcast_ref(),
                            volume_name_expr.upcast_ref(),
                        ],
                        closure!(|_: Self::Type, name_is_id: bool, name: &str| {
                            if name_is_id {
                                utils::format_id(name)
                            } else {
                                name
                            }
                            .to_owned()
                        }),
                    )
                    .upcast_ref(),
                    volume_to_be_deleted_expr.upcast_ref(),
                ],
                closure!(|_: Self::Type, name: String, to_be_deleted: bool| {
                    if to_be_deleted {
                        format!("<s>{name}</s>")
                    } else {
                        name
                    }
                }),
            )
            .bind(&*self.name_label, "label", Some(obj));

            let css_classes = utils::css_classes(&*self.name_label);
            volume_name_is_id_expr
                .chain_closure::<Vec<String>>(closure!(|_: Self::Type, name_is_id: bool| {
                    css_classes
                        .iter()
                        .cloned()
                        .chain(if name_is_id {
                            Some(String::from("numeric"))
                        } else {
                            None
                        })
                        .collect::<Vec<_>>()
                }))
                .bind(&*self.name_label, "css-classes", Some(obj));

            container_list_expr.bind(&*self.containers_count_bar, "container-list", Some(obj));

            let style_manager = adw::StyleManager::default();
            style_manager.connect_dark_notify(clone!(
                #[weak]
                obj,
                move |style_manager| {
                    obj.imp().set_path_label(style_manager);
                }
            ));
            style_manager.connect_accent_color_notify(clone!(
                #[weak]
                obj,
                move |style_manager| {
                    obj.imp().set_path_label(style_manager);
                }
            ));
            style_manager.connect_high_contrast_notify(clone!(
                #[weak]
                obj,
                move |style_manager| {
                    obj.imp().set_path_label(style_manager);
                }
            ));
            self.set_path_label(&style_manager);
        }
    }

    impl WidgetImpl for ContainerVolumeRow {}
    impl ListBoxRowImpl for ContainerVolumeRow {}

    impl ContainerVolumeRow {
        fn set_path_label(&self, style_manager: &adw::StyleManager) {
            if let Some(container_volume) = self.obj().container_volume() {
                let destination = container_volume.destination();
                let mut label = if style_manager.is_high_contrast() {
                    destination
                } else {
                    format!("<span alpha=\"55%\">{destination}</span>")
                };
                label.push(' ');

                let accent_color = style_manager
                    .accent_color()
                    .to_standalone_rgba(style_manager.is_dark());
                label.push_str(&format!(
                    "<span foreground=\"#{:02x}{:02x}{:02x}\"{}>",
                    (accent_color.red() * 255.0) as i32,
                    (accent_color.green() * 255.0) as i32,
                    (accent_color.blue() * 255.0) as i32,
                    if style_manager.is_high_contrast() {
                        " weight=\"bold\""
                    } else {
                        ""
                    },
                ));
                label.push_str(if container_volume.rw() { "rw" } else { "ro" });

                let mode = container_volume.mode();
                if !mode.is_empty() {
                    label.push(',');
                    label.push_str(&mode);
                }
                label.push_str("</span>");

                self.path_label.set_markup(&label);
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct ContainerVolumeRow(ObjectSubclass<imp::ContainerVolumeRow>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::Actionable, gtk::ConstraintTarget;

}

impl From<&model::ContainerVolume> for ContainerVolumeRow {
    fn from(container_volume: &model::ContainerVolume) -> Self {
        glib::Object::builder()
            .property("container-volume", container_volume)
            .build()
    }
}

impl ContainerVolumeRow {
    pub(crate) fn show_details(&self) {
        if let Some(ref volume) = self
            .container_volume()
            .as_ref()
            .and_then(model::ContainerVolume::volume)
        {
            utils::navigation_view(self).push(
                &adw::NavigationPage::builder()
                    .child(&view::VolumeDetailsPage::from(volume))
                    .build(),
            );
        }
    }
}
