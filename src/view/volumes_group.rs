use std::cell::RefCell;

use adw::subclass::prelude::PreferencesGroupImpl;
use gettextrs::gettext;
use gettextrs::ngettext;
use glib::clone;
use glib::closure;
use glib::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::unsync::OnceCell as UnsyncOnceCell;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_PRUNE_VOLUMES: &str = "volumes-group.prune-volumes";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::VolumesGroup)]
    #[template(file = "volumes_group.ui")]
    pub(crate) struct VolumesGroup {
        pub(super) settings: utils::PodsSettings,
        pub(super) properties_filter: UnsyncOnceCell<gtk::Filter>,
        pub(super) sorter: UnsyncOnceCell<gtk::Sorter>,
        #[property(get, set, nullable)]
        pub(super) no_volumes_label: RefCell<Option<String>>,
        #[property(get, set = Self::set_show_used_settings_key, explicit_notify)]
        pub(super) show_used_settings_key: RefCell<String>,
        #[property(get, set = Self::set_volume_list, explicit_notify, nullable)]
        pub(super) volume_list: glib::WeakRef<model::VolumeList>,
        #[template_child]
        pub(super) create_volume_row: TemplateChild<gtk::ListBoxRow>,
        #[template_child]
        pub(super) show_only_used_switch: TemplateChild<gtk::Switch>,
        #[template_child]
        pub(super) create_volume_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) list_box: TemplateChild<gtk::ListBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VolumesGroup {
        const NAME: &'static str = "PdsVolumesGroup";
        type Type = super::VolumesGroup;
        type ParentType = adw::PreferencesGroup;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_PRUNE_VOLUMES, None, |widget, _, _| {
                widget.show_prune_page();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for VolumesGroup {
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

            let volume_list_expr = Self::Type::this_expression("volume-list");
            let volume_list_len_expr = volume_list_expr.chain_property::<model::VolumeList>("len");
            let is_selection_mode_expr = volume_list_expr
                .chain_property::<model::SelectableList>("selection-mode")
                .chain_closure::<bool>(closure!(|_: Self::Type, selection_mode: bool| {
                    !selection_mode
                }));

            is_selection_mode_expr.bind(&*self.create_volume_button, "visible", Some(obj));
            is_selection_mode_expr.bind(&*self.create_volume_row, "visible", Some(obj));

            gtk::ClosureExpression::new::<Option<String>>(
                &[
                    volume_list_len_expr,
                    volume_list_expr.chain_property::<model::VolumeList>("used"),
                ],
                closure!(|obj: Self::Type, len: u32, used: u32| {
                    if len == 0 {
                        obj.no_volumes_label()
                    } else {
                        Some(if len == 1 {
                            if used == 1 {
                                gettext("1 volume, used")
                            } else {
                                gettext("1 volume, stopped")
                            }
                        } else {
                            ngettext!(
                                // Translators: There's a wide space (U+2002) between ", {}".
                                "{} volumes total, {} used",
                                "{} volumes total, {} used",
                                len,
                                len,
                                used,
                            )
                        })
                    }
                }),
            )
            .bind(obj, "description", Some(obj));

            let properties_filter = gtk::AnyFilter::new();
            properties_filter.append(gtk::CustomFilter::new(
                clone!(@weak obj => @default-return false, move |_| {
                    !obj.imp().show_only_used_switch.is_active()
                }),
            ));
            properties_filter.append(gtk::BoolFilter::new(Some(
                model::Volume::this_expression("container-list")
                    .chain_property::<model::SimpleContainerList>("len")
                    .chain_closure::<bool>(closure!(|_: model::Volume, len: u32| len > 0)),
            )));

            let sorter = gtk::StringSorter::new(Some(
                model::Volume::this_expression("inner").chain_closure::<String>(closure!(
                    |_: model::Volume, inner: model::BoxedVolume| inner.name.clone()
                )),
            ));

            self.properties_filter
                .set(properties_filter.upcast())
                .unwrap();
            self.sorter.set(sorter.upcast()).unwrap();

            self.show_only_used_switch
                .connect_active_notify(clone!(@weak obj => move |switch| {
                    obj.update_properties_filter(
                        if switch.is_active() {
                            gtk::FilterChange::MoreStrict
                        } else {
                            gtk::FilterChange::LessStrict
                        }
                    );
                }));
        }
    }

    impl WidgetImpl for VolumesGroup {}
    impl PreferencesGroupImpl for VolumesGroup {}

    impl VolumesGroup {
        pub(super) fn set_show_used_settings_key(&self, value: String) {
            let obj = &*self.obj();
            if obj.show_used_settings_key() == value {
                return;
            }

            self.settings
                .bind(&value, &*self.show_only_used_switch, "active")
                .build();

            self.show_used_settings_key.replace(value);
            obj.notify("show-used-settings-key");
        }

        pub(super) fn set_volume_list(&self, value: Option<&model::VolumeList>) {
            let obj = &*self.obj();
            if obj.volume_list().as_ref() == value {
                return;
            }

            if let Some(volume_list) = value {
                volume_list.connect_notify_local(
                    Some("used"),
                    clone!(@weak obj => move |_, _| {
                        obj.update_properties_filter(gtk::FilterChange::Different);
                    }),
                );

                let model = gtk::SortListModel::new(
                    Some(gtk::FilterListModel::new(
                        Some(volume_list.to_owned()),
                        self.properties_filter.get().cloned(),
                    )),
                    self.sorter.get().cloned(),
                );

                self.list_box.bind_model(Some(&model), |item| {
                    view::VolumeRow::from(item.downcast_ref().unwrap()).upcast()
                });
                self.list_box.append(&*self.create_volume_row);
            }

            self.volume_list.set(value);
            obj.notify("volume-list");
        }
    }
}

glib::wrapper! {
    pub(crate) struct VolumesGroup(ObjectSubclass<imp::VolumesGroup>)
        @extends gtk::Widget, adw::PreferencesGroup,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl VolumesGroup {
    pub(crate) fn action_create_volume() -> &'static str {
        "volumes-group.create-volume"
    }

    pub(crate) fn show_prune_page(&self) {
        if let Some(client) = self.volume_list().and_then(|list| list.client()) {
            utils::show_dialog(
                self.upcast_ref(),
                view::VolumesPrunePage::from(&client).upcast_ref(),
            );
        }
    }

    fn update_properties_filter(&self, filter_change: gtk::FilterChange) {
        self.imp()
            .properties_filter
            .get()
            .unwrap()
            .changed(filter_change);
    }
}
