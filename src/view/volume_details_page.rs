use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use glib::clone;
use glib::closure;
use glib::property::PropertySet;
use gtk::CompositeTemplate;
use gtk::gdk;
use gtk::glib;

use crate::gettext::gettext;
use crate::gettext::gettext_f;
use crate::model;
use crate::rt;
use crate::utils;
use crate::view;

const ACTION_COPY_NAME_TO_CLIPBOARD: &str = "volume-details-page.copy-name-to-clipboard";
const ACTION_OPEN_MOUNT_PATH: &str = "volume-details-page.open-mount-path";
const ACTION_COPY_MOUNT_PATH_TO_CLIPBOARD: &str =
    "volume-details-page.copy-mount-path-to-clipboard";
const ACTION_INSPECT_VOLUME: &str = "volume-details-page.inspect-volume";
const ACTION_DELETE_VOLUME: &str = "volume-details-page.delete-volume";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::VolumeDetailsPage)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/volume_details_page.ui")]
    pub(crate) struct VolumeDetailsPage {
        #[property(get, set = Self::set_volume, construct, explicit_notify, nullable)]
        pub(super) volume: glib::WeakRef<model::Volume>,

        #[template_child]
        pub(super) window_title: TemplateChild<adw::WindowTitle>,
        #[template_child]
        pub(super) name_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) mountpoint_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) open_mount_path_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) created_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) driver_row: TemplateChild<adw::ActionRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VolumeDetailsPage {
        const NAME: &'static str = "PdsVolumeDetailsPage";
        type Type = super::VolumeDetailsPage;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_COPY_NAME_TO_CLIPBOARD, None, |widget, _, _| {
                widget.copy_name_to_clipboard();
            });

            klass.install_action_async(ACTION_OPEN_MOUNT_PATH, None, async |widget, _, _| {
                widget.open_mount_path().await;
            });
            klass.install_action(ACTION_COPY_MOUNT_PATH_TO_CLIPBOARD, None, |widget, _, _| {
                widget.copy_mount_path_to_clipboard();
            });

            klass.install_action_async(ACTION_INSPECT_VOLUME, None, async |widget, _, _| {
                widget.show_inspection().await;
            });

            klass.install_action_async(ACTION_DELETE_VOLUME, None, async |widget, _, _| {
                widget.delete_volume().await;
            });

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                view::ContainersGroup::action_create_container(),
            );
            klass.install_action_async(
                view::ContainersGroup::action_create_container(),
                None,
                async |widget, _, _| {
                    widget.create_container().await;
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
            let to_be_deleted_expr = volume_expr.chain_property::<model::Volume>("to-be-deleted");
            let name_expr = volume_expr.chain_property::<model::Volume>("name");
            let mountpoint_expr = volume_expr.chain_property::<model::Volume>("mountpoint");
            let driver_expr = volume_expr.chain_property::<model::Volume>("driver");
            let volume_list_expr = volume_expr.chain_property::<model::Volume>("volume-list");
            let client_expr = volume_list_expr.chain_property::<model::VolumeList>("client");
            let connection_expr = client_expr.chain_property::<model::Client>("connection");
            let is_local_expr = connection_expr.chain_property::<model::Connection>("is-local");

            to_be_deleted_expr.watch(
                Some(obj),
                clone!(
                    #[weak]
                    obj,
                    #[strong]
                    to_be_deleted_expr,
                    move || obj.action_set_enabled(
                        ACTION_DELETE_VOLUME,
                        to_be_deleted_expr.evaluate_as(Some(&obj)).unwrap_or(false),
                    )
                ),
            );

            name_expr
                .chain_closure::<String>(closure!(|_: Self::Type, name: &str| {
                    utils::format_volume_name(name).to_owned()
                }))
                .bind(&*self.name_row, "subtitle", Some(obj));

            mountpoint_expr.bind(&*self.mountpoint_row, "subtitle", Some(obj));

            is_local_expr.bind(&*self.open_mount_path_button, "visible", Some(obj));

            gtk::ClosureExpression::new::<String>(
                &[
                    &Self::Type::this_expression("root")
                        .chain_property::<gtk::Window>("application")
                        .chain_property::<crate::Application>("ticks"),
                    &volume_expr.chain_property::<model::Volume>("created-at"),
                ],
                closure!(|_: Self::Type, _ticks: u64, created_at: i64| {
                    utils::format_ago(utils::timespan_now(created_at))
                }),
            )
            .bind(&*self.created_row, "subtitle", Some(obj));

            driver_expr.bind(&*self.driver_row, "subtitle", Some(obj));

            is_local_expr.watch(
                Some(obj),
                clone!(
                    #[weak]
                    obj,
                    #[strong]
                    is_local_expr,
                    move || obj.action_set_enabled(
                        ACTION_OPEN_MOUNT_PATH,
                        is_local_expr.evaluate_as(Some(&obj)).unwrap_or(false)
                    )
                ),
            );
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

            if let Some(volume) = value {
                self.window_title
                    .set_subtitle(utils::format_volume_name(&volume.name()));

                let handler_id_ref = Rc::new(RefCell::new(None));
                let handler_id = volume.connect_deleted(clone!(
                    #[weak]
                    obj,
                    #[strong]
                    handler_id_ref,
                    move |volume| {
                        volume.disconnect(handler_id_ref.take().unwrap());

                        utils::show_toast(
                            &obj,
                            gettext_f("Volume '{}' has been deleted", [volume.name()]),
                        );

                        let navigation_view = utils::navigation_view(&obj);
                        navigation_view.pop_to_page(&utils::navigation_page(&obj));
                        navigation_view.pop();
                    }
                ));
                handler_id_ref.set(Some(handler_id));
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
    fn copy_to_clipboard<F: Fn(&model::Volume) -> String>(&self, op: F) {
        let Some(volume) = self.volume() else {
            return;
        };
        self.display().clipboard().set_text(&op(&volume));

        utils::show_toast(self, gettext("Copied to clipboard"));
    }

    pub(crate) fn copy_name_to_clipboard(&self) {
        self.copy_to_clipboard(model::Volume::name);
    }

    pub(crate) async fn open_mount_path(&self) {
        let Some(volume) = self.volume() else {
            return;
        };

        let uri = gtk::gio::File::for_path(volume.mountpoint())
            .uri()
            .to_string();

        let open_result = rt::Promise::new(async move {
            let connection = ashpd::zbus::Connection::session().await?;
            connection
                .call_method(
                    Some("org.freedesktop.FileManager1"),
                    "/org/freedesktop/FileManager1",
                    Some("org.freedesktop.FileManager1"),
                    "ShowFolders",
                    &(vec![uri.as_str()], ""), // (as uris, s startup_id)
                )
                .await
                .map(|_| ())
        })
        .exec()
        .await;

        if let Err(e) = open_result {
            utils::show_error_toast(self, &gettext("Error opening link"), &e.to_string());
        }
    }

    pub(crate) fn copy_mount_path_to_clipboard(&self) {
        self.copy_to_clipboard(model::Volume::mountpoint);
    }

    pub(crate) async fn show_inspection(&self) {
        self.exec_action(async || {
            let Some(volume) = self.volume() else {
                return;
            };

            let weak_ref = glib::WeakRef::new();
            weak_ref.set(Some(&volume));

            utils::navigation_view(self).push(
                &adw::NavigationPage::builder()
                    .child(&view::ScalableTextViewPage::from(view::Entity::Volume(
                        weak_ref,
                    )))
                    .build(),
            );
        })
        .await;
    }

    pub(crate) async fn delete_volume(&self) {
        self.exec_action(async || {
            view::volume::delete_volume_show_confirmation(self, self.volume().as_ref()).await;
        })
        .await;
    }

    pub(crate) async fn create_container(&self) {
        self.exec_action(async || {
            view::volume::create_container(self, self.volume());
        })
        .await;
    }

    async fn exec_action<F: AsyncFn()>(&self, op: F) {
        if utils::navigation_view(self)
            .visible_page()
            .filter(|page| page.child().as_ref() == Some(self.upcast_ref()))
            .is_some()
        {
            op().await;
        }
    }
}
