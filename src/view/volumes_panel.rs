use adw::prelude::MessageDialogExtManual;
use adw::traits::MessageDialogExt;
use gettextrs::gettext;
use glib::clone;
use glib::closure;
use glib::subclass::Signal;
use glib::Properties;
use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy as SyncLazy;

use crate::model;
use crate::model::SelectableListExt;
use crate::utils;
use crate::view;

const ACTION_DELETE_SELECTION: &str = "volumes-panel.delete-selection";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::VolumesPanel)]
    #[template(file = "volumes_panel.ui")]
    pub(crate) struct VolumesPanel {
        #[property(get, set = Self::set_volume_list, explicit_notify, nullable)]
        pub(super) volume_list: glib::WeakRef<model::VolumeList>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) volumes_group: TemplateChild<view::VolumesGroup>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for VolumesPanel {
        const NAME: &'static str = "PdsVolumesPanel";
        type Type = super::VolumesPanel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                Self::Type::action_create_volume(),
                None,
            );
            klass.install_action(
                Self::Type::action_create_volume(),
                None,
                move |widget, _, _| {
                    widget.create_volume();
                },
            );

            klass.install_action(ACTION_DELETE_SELECTION, None, |widget, _, _| {
                widget.delete_selection();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for VolumesPanel {
        fn signals() -> &'static [Signal] {
            static SIGNALS: SyncLazy<Vec<Signal>> =
                SyncLazy::new(|| vec![Signal::builder("exit-selection-mode").build()]);
            SIGNALS.as_ref()
        }

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

            gtk::ClosureExpression::new::<Option<String>>(
                [
                    &volume_list_len_expr,
                    &volume_list_expr.chain_property::<model::VolumeList>("listing"),
                    &volume_list_expr.chain_property::<model::VolumeList>("initialized"),
                ],
                closure!(
                    |_: Self::Type, len: u32, listing: bool, initialized: bool| {
                        if len == 0 {
                            if initialized {
                                Some("empty")
                            } else if listing {
                                Some("spinner")
                            } else {
                                None
                            }
                        } else {
                            Some("volumes")
                        }
                    }
                ),
            )
            .bind(&*self.main_stack, "visible-child-name", Some(obj));
        }

        fn dispose(&self) {
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for VolumesPanel {}

    impl VolumesPanel {
        pub(super) fn set_volume_list(&self, value: &model::VolumeList) {
            let obj = &*self.obj();
            if obj.volume_list().as_ref() == Some(value) {
                return;
            }

            obj.action_set_enabled(ACTION_DELETE_SELECTION, false);
            value.connect_notify_local(
                Some("num-selected"),
                clone!(@weak obj => move |list, _| {
                    obj.action_set_enabled(ACTION_DELETE_SELECTION, list.num_selected() > 0);
                }),
            );

            self.volume_list.set(Some(value));
            obj.notify("volume-list");
        }
    }
}

glib::wrapper! {
    pub(crate) struct VolumesPanel(ObjectSubclass<imp::VolumesPanel>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for VolumesPanel {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}

impl VolumesPanel {
    pub(crate) fn action_create_volume() -> &'static str {
        view::VolumesGroup::action_create_volume()
    }

    pub(crate) fn create_volume(&self) {
        if let Some(client) = self
            .volume_list()
            .as_ref()
            .and_then(model::VolumeList::client)
        {
            utils::show_dialog(
                self.upcast_ref(),
                view::VolumeCreationPage::from(&client).upcast_ref(),
            );
        }
    }

    fn delete_selection(&self) {
        if self
            .volume_list()
            .map(|list| list.num_selected())
            .unwrap_or(0)
            == 0
        {
            return;
        }

        let dialog = adw::MessageDialog::builder()
            .heading(gettext("Confirm Forced Deletion of Multiple Volumes"))
            .body(gettext(
                "There may be containers associated with some of the volumes, which will also be removed!",
            ))
            .modal(true)
            .transient_for(&utils::root(self.upcast_ref()))
            .build();

        dialog.add_responses(&[
            ("cancel", &gettext("_Cancel")),
            ("delete", &gettext("_Delete")),
        ]);
        dialog.set_default_response(Some("cancel"));
        dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

        dialog.connect_response(
            None,
            clone!(@weak self as obj => move |_, response| if response == "delete" {
                if let Some(list) = obj.volume_list() {
                    list
                        .selected_items()
                        .iter()
                        .map(|obj| obj.downcast_ref::<model::Volume>().unwrap())
                        .for_each(|volume|
                    {
                        volume.delete(true, clone!(@weak obj => move |volume, result| {
                            if let Err(e) = result {
                                utils::show_error_toast(
                                    obj.upcast_ref(),
                                    // Translators: The first "{}" is a placeholder for the image id, the second is for an error message.
                                    &gettext!(
                                        "Error on deleting volume '{}'",
                                        volume.inner().name
                                    ),
                                    &e.to_string()
                                );
                            }
                        }));
                    });
                    list.set_selection_mode(false);
                    obj.emit_by_name::<()>("exit-selection-mode", &[]);
                }
            }),
        );

        dialog.present();
    }

    pub(crate) fn connect_exit_selection_mode<F: Fn(&Self) + 'static>(
        &self,
        f: F,
    ) -> glib::SignalHandlerId {
        self.connect_local("exit-selection-mode", true, move |values| {
            let obj = values[0].get::<Self>().unwrap();
            f(&obj);

            None
        })
    }
}
