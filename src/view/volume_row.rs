use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::model::SelectableExt;
use crate::model::SelectableListExt;
use crate::utils;
use crate::view;
use crate::widget;

const ACTION_DELETE_VOLUME: &str = "volume-row.delete-volume";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::VolumeRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/volume_row.ui")]
    pub(crate) struct VolumeRow {
        #[property(get, set = Self::set_volume, construct, nullable)]
        pub(super) volume: RefCell<Option<model::Volume>>,
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        #[template_child]
        pub(super) check_button_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub(super) check_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) age_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) spinner: TemplateChild<widget::EfficientSpinner>,
        #[template_child]
        pub(super) containers_count_bar: TemplateChild<view::ContainersCountBar>,
        #[template_child]
        pub(super) end_box_revealer: TemplateChild<gtk::Revealer>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VolumeRow {
        const NAME: &'static str = "PdsVolumeRow";
        type Type = super::VolumeRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("volume-row.activate", None, |widget, _, _| {
                widget.activate();
            });

            klass.install_action(ACTION_DELETE_VOLUME, None, |widget, _, _| {
                widget.delete_volume();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for VolumeRow {
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

            let ticks_expr = Self::Type::this_expression("root")
                .chain_property::<gtk::Window>("application")
                .chain_property::<crate::Application>("ticks");

            let volume_expr = Self::Type::this_expression("volume");
            let volume_inner_expr = volume_expr.chain_property::<model::Volume>("inner");
            let volume_name_is_id_expr = volume_inner_expr.chain_closure::<bool>(closure!(
                |_: Self::Type, inner: model::BoxedVolume| utils::is_podman_id(&inner.name)
            ));
            let volume_to_be_deleted_expr =
                volume_expr.chain_property::<model::Volume>("to-be-deleted");
            let container_list_expr = volume_expr.chain_property::<model::Volume>("container-list");

            let selection_mode_expr = volume_expr
                .chain_property::<model::Volume>("volume-list")
                .chain_property::<model::VolumeList>("selection-mode");

            selection_mode_expr.bind(&*self.check_button_revealer, "reveal-child", Some(obj));
            selection_mode_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, is_selection_mode: bool| {
                    !is_selection_mode
                }))
                .bind(&*self.end_box_revealer, "reveal-child", Some(obj));

            gtk::ClosureExpression::new::<String>(
                [
                    gtk::ClosureExpression::new::<String>(
                        [
                            volume_name_is_id_expr.upcast_ref(),
                            volume_inner_expr.upcast_ref(),
                        ],
                        closure!(
                            |_: Self::Type, name_is_id: bool, inner: &model::BoxedVolume| {
                                if name_is_id {
                                    utils::format_id(&inner.name)
                                } else {
                                    inner.name.clone()
                                }
                            }
                        ),
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

            let css_classes = utils::css_classes(self.name_label.upcast_ref());
            gtk::ClosureExpression::new::<Vec<String>>(
                [
                    container_list_expr
                        .chain_property::<model::SimpleContainerList>("len")
                        .upcast_ref(),
                    volume_name_is_id_expr.upcast_ref(),
                ],
                closure!(|_: Self::Type, len: u32, name_is_id: bool| {
                    css_classes
                        .iter()
                        .cloned()
                        .chain(if len == 0 {
                            Some(String::from("dim-label"))
                        } else {
                            None
                        })
                        .chain(if name_is_id {
                            Some(String::from("numeric"))
                        } else {
                            None
                        })
                        .collect::<Vec<_>>()
                }),
            )
            .bind(&*self.name_label, "css-classes", Some(obj));

            gtk::ClosureExpression::new::<String>(
                [&ticks_expr, &volume_inner_expr],
                closure!(|_: Self::Type, _ticks: u64, inner: model::BoxedVolume| {
                    // Translators: This will resolve to sth. like "{a few minutes} old" or "{15 days} old".
                    gettext!(
                        "{} old",
                        utils::human_friendly_timespan(utils::timespan_now(
                            glib::DateTime::from_iso8601(
                                inner.created_at.as_deref().unwrap(),
                                None
                            )
                            .unwrap()
                            .to_unix(),
                        ))
                    )
                }),
            )
            .bind(&*self.age_label, "label", Some(obj));

            volume_expr
                .chain_property::<model::Volume>("searching-containers")
                .bind(&self.spinner.get(), "visible", Some(obj));

            container_list_expr.bind(&*self.containers_count_bar, "container-list", Some(obj));

            volume_to_be_deleted_expr.watch(
                Some(obj),
                clone!(@weak obj, @strong volume_to_be_deleted_expr => move || {
                    obj.action_set_enabled(
                        ACTION_DELETE_VOLUME,
                        !volume_to_be_deleted_expr.evaluate_as::<bool, _>(Some(&obj)).unwrap()
                    );
                }),
            );

            if let Some(volume) = obj.volume() {
                obj.action_set_enabled("volume.show-details", !volume.to_be_deleted());
                volume.connect_notify_local(
                    Some("to-be-deleted"),
                    clone!(@weak obj => move|volume, _| {
                        obj.action_set_enabled("volume.show-details", !volume.to_be_deleted());
                    }),
                );
            }
        }
    }

    impl WidgetImpl for VolumeRow {}
    impl ListBoxRowImpl for VolumeRow {}

    impl VolumeRow {
        pub(super) fn set_volume(&self, value: Option<model::Volume>) {
            let obj = &*self.obj();
            if obj.volume() == value {
                return;
            }

            let mut bindings = self.bindings.borrow_mut();
            while let Some(binding) = bindings.pop() {
                binding.unbind();
            }

            if let Some(ref volume) = value {
                let binding = volume
                    .bind_property("selected", &*self.check_button, "active")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();

                bindings.push(binding);
            }

            self.volume.set(value);
            obj.notify("volume")
        }
    }
}

glib::wrapper! {
    pub(crate) struct VolumeRow(ObjectSubclass<imp::VolumeRow>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::Actionable, gtk::ConstraintTarget;
}

impl From<&model::Volume> for VolumeRow {
    fn from(volume: &model::Volume) -> Self {
        glib::Object::builder().property("volume", volume).build()
    }
}

impl VolumeRow {
    pub(crate) fn activate(&self) {
        if let Some(volume) = self.volume().as_ref() {
            if volume
                .volume_list()
                .map(|list| list.is_selection_mode())
                .unwrap_or(false)
            {
                volume.select();
            } else {
                utils::navigation_view(self.upcast_ref()).push(
                    &adw::NavigationPage::builder()
                        .title(gettext!(
                            "Volume {}",
                            utils::format_volume_name(&volume.inner().name)
                        ))
                        .child(&view::VolumeDetailsPage::from(volume))
                        .build(),
                );
            }
        }
    }

    pub(crate) fn delete_volume(&self) {
        view::volume::delete_volume_show_confirmation(self.upcast_ref(), self.volume());
    }
}
