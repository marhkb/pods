use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use glib::clone;
use glib::closure;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::expression::ExpressionExt;
use crate::gettext::gettext;
use crate::model;
use crate::utils;
use crate::view;

const ACTION_SHOW_DETAILS: &str = "container-volume-row.show-details";
const ACTION_OPEN_HOST_PATH: &str = "container-volume-row.open-host-path";
const ACTION_COPY_HOST_PATH: &str = "container-volume-row.copy-host-path";
const ACTION_COPY_CONTAINER_PATH: &str = "container-volume-row.copy-container-path";

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
        pub(super) containers_count_bar: TemplateChild<view::ContainersCountBar>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ContainerVolumeRow {
        const NAME: &'static str = "PdsContainerVolumeRow";
        type Type = super::ContainerVolumeRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.install_action(ACTION_SHOW_DETAILS, None, |widget, _, _| {
                widget.show_details();
            });

            klass.install_action_async(ACTION_OPEN_HOST_PATH, None, async |widget, _, _| {
                widget.open_host_path().await;
            });
            klass.install_action(ACTION_COPY_HOST_PATH, None, |widget, _, _| {
                widget.copy_host_path();
            });
            klass.install_action(ACTION_COPY_CONTAINER_PATH, None, |widget, _, _| {
                widget.copy_container_path();
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
                volume_name_expr.map(|name: String| utils::as_id(&name).is_some());
            let volume_to_be_deleted_expr =
                volume_expr.chain_property::<model::Volume>("to-be-deleted");
            let containers_expr = volume_expr.chain_property::<model::Volume>("containers");
            let volume_list_expr = volume_expr.chain_property::<model::Volume>("volume-list");
            let client_expr = volume_list_expr.chain_property::<model::VolumeList>("client");
            let connection_expr = client_expr.chain_property::<model::Client>("connection");
            let is_local_expr = connection_expr.chain_property::<model::Connection>("is-local");

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
                .map(move |name_is_id: bool| {
                    css_classes
                        .iter()
                        .cloned()
                        .chain(if name_is_id {
                            Some(String::from("numeric"))
                        } else {
                            None
                        })
                        .collect::<Vec<_>>()
                })
                .bind(&*self.name_label, "css-classes", Some(obj));

            containers_expr.bind(&*self.containers_count_bar, "container-list", Some(obj));

            is_local_expr.watch(
                Some(obj),
                clone!(
                    #[weak]
                    obj,
                    #[strong]
                    is_local_expr,
                    move || obj.action_set_enabled(
                        ACTION_OPEN_HOST_PATH,
                        is_local_expr.evaluate_as(Some(&obj)).unwrap_or(false)
                    )
                ),
            );
        }
    }

    #[gtk::template_callbacks]
    impl ContainerVolumeRow {
        #[template_callback]
        fn format_path(&self, destination: String, rw: bool, mode: String) -> String {
            let mut path = format!("{destination}:{}", if rw { "rw" } else { "ro" });
            if !mode.is_empty() {
                path.push(',');
                path.push_str(&mode);
            }
            path
        }
    }

    impl WidgetImpl for ContainerVolumeRow {}
    impl ListBoxRowImpl for ContainerVolumeRow {}
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
    fn volume(&self) -> Option<model::Volume> {
        self.container_volume()
            .as_ref()
            .and_then(model::ContainerVolume::volume)
    }

    pub(crate) fn show_details(&self) {
        if let Some(ref volume) = self.volume() {
            utils::navigation_view(self).push(&view::VolumeDetailsPage::from(volume));
        }
    }

    pub(crate) async fn open_host_path(&self) {
        if let Some(ref volume) = self.volume() {
            view::volume::open_mount_path(self, volume).await;
        }
    }

    pub(crate) fn copy_host_path(&self) {
        if let Some(volume) = self.volume() {
            self.copy_to_clipboard(&volume.mountpoint());
        }
    }

    pub(crate) fn copy_container_path(&self) {
        if let Some(container_volume) = self.container_volume() {
            self.copy_to_clipboard(&container_volume.destination());
        }
    }

    fn copy_to_clipboard(&self, text: &str) {
        self.display().clipboard().set_text(text);
        utils::show_toast(self, gettext("Copied to clipboard"));
    }
}
