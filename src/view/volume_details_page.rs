use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::Properties;
use glib::clone;
use glib::closure;
use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::glib;

use crate::model;
use crate::utils;
use crate::view;
use crate::widget;

const ACTION_INSPECT_VOLUME: &str = "volume-details-page.inspect-volume";
const ACTION_DELETE_VOLUME: &str = "volume-details-page.delete-volume";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::VolumeDetailsPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/volume_details_page.ui")]
    pub(crate) struct VolumeDetailsPage {
        pub(super) handler_id: RefCell<Option<glib::SignalHandlerId>>,
        #[property(get, set = Self::set_volume, construct, explicit_notify, nullable)]
        pub(super) volume: glib::WeakRef<model::Volume>,
        #[template_child]
        pub(super) window_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub(super) name_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) created_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) driver_row: TemplateChild<widget::PropertyRow>,
        #[template_child]
        pub(super) mountpoint_row: TemplateChild<widget::PropertyRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VolumeDetailsPage {
        const NAME: &'static str = "PdsVolumeDetailsPage";
        type Type = super::VolumeDetailsPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_INSPECT_VOLUME, None, |widget, _, _| {
                widget.show_inspection();
            });

            klass.install_action(ACTION_DELETE_VOLUME, None, |widget, _, _| {
                widget.delete_volume();
            });

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                view::ContainersGroup::action_create_container(),
            );
            klass.install_action(
                view::ContainersGroup::action_create_container(),
                None,
                move |widget, _, _| {
                    widget.create_container();
                },
            );
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for VolumeDetailsPage {
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

            let volume_expr = Self::Type::this_expression("volume");
            let volume_inner_expr = volume_expr.chain_property::<model::Volume>("inner");

            volume_expr
                .chain_property::<model::Volume>("to-be-deleted")
                .watch(
                    Some(obj),
                    clone!(
                        #[weak]
                        obj,
                        move || {
                            obj.action_set_enabled(
                                ACTION_DELETE_VOLUME,
                                obj.volume()
                                    .map(|volume| !volume.to_be_deleted())
                                    .unwrap_or(false),
                            );
                        }
                    ),
                );

            volume_inner_expr
                .chain_closure::<String>(closure!(|_: Self::Type, inner: &model::BoxedVolume| {
                    utils::format_volume_name(&inner.name)
                }))
                .bind(&*self.name_row, "value", Some(obj));

            gtk::ClosureExpression::new::<String>(
                &[
                    &Self::Type::this_expression("root")
                        .chain_property::<gtk::Window>("application")
                        .chain_property::<crate::Application>("ticks"),
                    &volume_inner_expr,
                ],
                closure!(|_: Self::Type, _ticks: u64, inner: &model::BoxedVolume| {
                    utils::format_ago(utils::timespan_now(
                        inner
                            .created_at
                            .as_ref()
                            .and_then(|created_at| {
                                glib::DateTime::from_iso8601(created_at, None).ok()
                            })
                            .map(|date_time| date_time.to_unix())
                            .unwrap_or(0),
                    ))
                }),
            )
            .bind(&*self.created_row, "value", Some(obj));

            volume_inner_expr
                .chain_closure::<String>(closure!(|_: Self::Type, inner: &model::BoxedVolume| {
                    inner.driver.clone()
                }))
                .bind(&*self.driver_row, "value", Some(obj));

            volume_inner_expr
                .chain_closure::<String>(closure!(|_: Self::Type, inner: &model::BoxedVolume| {
                    inner.mountpoint.clone()
                }))
                .bind(&*self.mountpoint_row, "value", Some(obj));
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for VolumeDetailsPage {}

    impl VolumeDetailsPage {
        pub(super) fn set_volume(&self, value: Option<&model::Volume>) {
            let obj = &*self.obj();
            if obj.volume().as_ref() == value {
                return;
            }

            self.window_title.set_subtitle("");
            if let Some(volume) = obj.volume() {
                volume.disconnect(self.handler_id.take().unwrap());
            }

            if let Some(volume) = value {
                self.window_title
                    .set_subtitle(&utils::format_volume_name(&volume.inner().name));

                let handler_id = volume.connect_deleted(clone!(
                    #[weak]
                    obj,
                    move |volume| {
                        utils::show_toast(
                            &obj,
                            gettext!("Volume '{}' has been deleted", volume.inner().name),
                        );
                        utils::navigation_view(&obj).pop();
                    }
                ));
                self.handler_id.replace(Some(handler_id));
            }

            self.volume.set(value);
            obj.notify("volume");
        }
    }
}

glib::wrapper! {
    pub(crate) struct VolumeDetailsPage(ObjectSubclass<imp::VolumeDetailsPage>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Volume> for VolumeDetailsPage {
    fn from(volume: &model::Volume) -> Self {
        glib::Object::builder().property("volume", volume).build()
    }
}

impl VolumeDetailsPage {
    pub(crate) fn show_inspection(&self) {
        self.exec_action(|| {
            if let Some(volume) = self.volume() {
                let weak_ref = glib::WeakRef::new();
                weak_ref.set(Some(&volume));

                utils::navigation_view(self).push(
                    &adw::NavigationPage::builder()
                        .child(&view::ScalableTextViewPage::from(view::Entity::Volume(
                            weak_ref,
                        )))
                        .build(),
                );
            }
        });
    }

    pub(crate) fn delete_volume(&self) {
        self.exec_action(|| {
            view::volume::delete_volume_show_confirmation(self, self.volume());
        });
    }

    fn exec_action<F: Fn()>(&self, op: F) {
        if utils::navigation_view(self)
            .visible_page()
            .filter(|page| page.child().as_ref() == Some(self.upcast_ref()))
            .is_some()
        {
            op();
        }
    }

    pub(crate) fn create_container(&self) {
        self.exec_action(|| {
            view::volume::create_container(self, self.volume());
        });
    }
}
