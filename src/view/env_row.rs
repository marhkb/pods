use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::ExpanderRowImpl;
use adw::subclass::prelude::PreferencesRowImpl;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;
use crate::view;
use crate::widget;

const ACTION_SELECT_VOLUME: &str = "env-row.select-volume";
const ACTION_CLEAR_VOLUME: &str = "env-row.clear-volume";
const ACTION_REMOVE: &str = "env-row.remove";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::EnvRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/env_row.ui")]
    pub(crate) struct EnvRow {
        #[property(get, set = Self::set_mount, construct, nullable)]
        pub(super) mount: RefCell<Option<model::Mount>>,
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        #[template_child]
        pub(super) type_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) src_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) container_path_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) options_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) mount_type_row: TemplateChild<widget::PropertyWidgetRow>,
        #[template_child]
        pub(super) mount_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) volume_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) host_path_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) volume_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub(super) container_path_entry_row: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub(super) writable_switch_row: TemplateChild<adw::SwitchRow>,
        #[template_child]
        pub(super) selinux_combo_row: TemplateChild<adw::ComboRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EnvRow {
        const NAME: &'static str = "PdsMountRow";
        type Type = super::EnvRow;
        type ParentType = adw::ExpanderRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_SELECT_VOLUME, None, |widget, _, _| {
                widget.select_volume();
            });
            klass.install_action(ACTION_CLEAR_VOLUME, None, |widget, _, _| {
                widget.clear_volume();
            });
            klass.install_action(ACTION_REMOVE, None, |widget, _, _| {
                if let Some(mount) = widget.mount() {
                    mount.remove_request();
                }
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EnvRow {
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

            let mount_expr = Self::Type::this_expression("mount");
            let mount_type_expr = mount_expr.chain_property::<model::Mount>("mount-type");

            mount_type_expr
                .chain_closure::<String>(closure!(|_: Self::Type, mount_type: model::MountType| {
                    match mount_type {
                        model::MountType::Bind => "Bind",
                        model::MountType::Volume => "Volume",
                    }
                }))
                .bind(&self.type_label.get(), "label", Some(obj));

            gtk::ClosureExpression::new::<String>(
                [
                    &mount_type_expr,
                    &mount_expr.chain_property::<model::Mount>("host-path"),
                    &mount_expr.chain_property::<model::Mount>("volume"),
                ],
                closure!(|_: Self::Type,
                          mount_type: model::MountType,
                          host_path: &str,
                          volume: Option::<model::Volume>| {
                    match mount_type {
                        model::MountType::Bind => {
                            let host_path = host_path.trim();
                            if host_path.is_empty() { "?" } else { host_path }.to_string()
                        }
                        model::MountType::Volume => volume
                            .map(|volume| utils::format_volume_name(&volume.inner().name))
                            .unwrap_or_else(|| "?".to_string()),
                    }
                }),
            )
            .bind(&self.src_label.get(), "label", Some(obj));

            mount_expr
                .chain_property::<model::Mount>("container-path")
                .chain_closure::<String>(closure!(|_: Self::Type, path: &str| {
                    let path = path.trim();
                    if path.is_empty() { "?" } else { path }.to_string()
                }))
                .bind(&self.container_path_label.get(), "label", Some(obj));

            gtk::ClosureExpression::new::<String>(
                [
                    mount_expr.chain_property::<model::Mount>("writable"),
                    mount_expr.chain_property::<model::Mount>("selinux"),
                ],
                closure!(
                    |_: Self::Type, writable: bool, selinux: model::MountSELinux| {
                        let mut s = if writable { "rw" } else { "ro" }.to_string();
                        let selinux: &str = selinux.as_ref();
                        if !selinux.is_empty() {
                            s.push(',');
                            s.push_str(selinux);
                        }
                        s
                    }
                ),
            )
            .bind(&self.options_label.get(), "label", Some(obj));

            mount_type_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, mount_type: model::MountType| {
                    matches!(mount_type, model::MountType::Bind)
                }))
                .bind(&self.host_path_entry_row.get(), "visible", Some(obj));

            mount_type_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, mount_type: model::MountType| {
                    matches!(mount_type, model::MountType::Volume)
                }))
                .bind(&self.volume_row.get(), "visible", Some(obj));

            mount_expr
                .chain_property::<model::Mount>("volume")
                .chain_closure::<String>(closure!(
                    |_: Self::Type, volume: Option<model::Volume>| {
                        volume
                            .map(|volume| utils::format_volume_name(&volume.inner().name))
                            .unwrap_or_else(|| {
                                format!(
                                    "<i>{}</i>",
                                    gettext("No volume selected (will create a new one).")
                                )
                            })
                    }
                ))
                .bind(&self.volume_row.get(), "subtitle", Some(obj));

            obj.action_set_enabled(ACTION_CLEAR_VOLUME, false);
        }
    }

    impl WidgetImpl for EnvRow {}
    impl ListBoxRowImpl for EnvRow {}
    impl PreferencesRowImpl for EnvRow {}
    impl ExpanderRowImpl for EnvRow {}

    impl EnvRow {
        pub(super) fn set_mount(&self, value: Option<model::Mount>) {
            let obj = &*self.obj();

            if obj.mount() == value {
                return;
            }

            let mut bindings = self.bindings.borrow_mut();

            while let Some(binding) = bindings.pop() {
                binding.unbind();
            }

            if let Some(ref mount) = value {
                let binding = mount
                    .bind_property("mount-type", &*self.mount_button, "active")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .transform_to(|_, mount_type: model::MountType| {
                        matches!(mount_type, model::MountType::Bind)
                            .to_value()
                            .into()
                    })
                    .transform_from(|_, active: bool| {
                        if active {
                            model::MountType::Bind
                        } else {
                            model::MountType::Volume
                        }
                        .to_value()
                        .into()
                    })
                    .build();
                bindings.push(binding);

                let binding = mount
                    .bind_property("mount-type", &*self.volume_button, "active")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .transform_to(|_, mount_type: model::MountType| {
                        matches!(mount_type, model::MountType::Volume)
                            .to_value()
                            .into()
                    })
                    .transform_from(|_, active: bool| {
                        if active {
                            model::MountType::Volume
                        } else {
                            model::MountType::Bind
                        }
                        .to_value()
                        .into()
                    })
                    .build();
                bindings.push(binding);

                let binding = mount
                    .bind_property("host-path", &*self.host_path_entry_row, "text")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();
                bindings.push(binding);

                let binding = mount
                    .bind_property("container-path", &*self.container_path_entry_row, "text")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();
                bindings.push(binding);

                let binding = mount
                    .bind_property("writable", &*self.writable_switch_row, "active")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();
                bindings.push(binding);

                let binding = mount
                    .bind_property("selinux", &*self.selinux_combo_row, "selected")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .transform_to(|_, selinux: model::MountSELinux| {
                        Some(
                            match selinux {
                                model::MountSELinux::NoLabel => 0_u32,
                                model::MountSELinux::Shared => 1_u32,
                                model::MountSELinux::Private => 2_u32,
                            }
                            .to_value(),
                        )
                    })
                    .transform_from(|_, position: u32| {
                        Some(
                            match position {
                                0 => model::MountSELinux::NoLabel,
                                1 => model::MountSELinux::Shared,
                                _ => model::MountSELinux::Private,
                            }
                            .to_value(),
                        )
                    })
                    .build();
                bindings.push(binding);
            }

            self.mount.replace(value);
        }
    }
}

glib::wrapper! {
    pub(crate) struct EnvRow(ObjectSubclass<imp::EnvRow>)
        @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ExpanderRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::Mount> for EnvRow {
    fn from(mount: &model::Mount) -> Self {
        glib::Object::builder().property("mount", mount).build()
    }
}

impl EnvRow {
    pub(crate) fn select_volume(&self) {
        if let Some(client) = self.mount().and_then(|mount| mount.client()) {
            let volume_selection_page = view::VolumeSelectionPage::from(&client.volume_list());
            volume_selection_page.connect_volume_selected(
                clone!(@weak self as obj => move |_, volume| {
                    if let Some(mount) = obj.mount() {
                        mount.set_volume(Some(volume));
                        obj.action_set_enabled(ACTION_CLEAR_VOLUME, true);
                    }
                }),
            );
            utils::navigation_view(self.upcast_ref()).push(
                &adw::NavigationPage::builder()
                    .child(&volume_selection_page)
                    .build(),
            );
        }
    }

    pub(crate) fn clear_volume(&self) {
        if let Some(mount) = self.mount() {
            mount.set_volume(Option::<model::Volume>::None);
            self.action_set_enabled(ACTION_CLEAR_VOLUME, false);
        }
    }
}
