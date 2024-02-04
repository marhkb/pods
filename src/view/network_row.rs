use std::cell::RefCell;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::Properties;
use glib::clone;
use glib::closure;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::model;
use crate::model::SelectableExt;
use crate::model::SelectableListExt;
use crate::utils;
use crate::view;

const ACTION_DELETE: &str = "network-row.delete";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::NetworkRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/network_row.ui")]
    pub(crate) struct NetworkRow {
        pub(super) bindings: RefCell<Vec<glib::Binding>>,
        #[property(get, set = Self::set_network, construct, nullable)]
        pub(super) network: glib::WeakRef<model::Network>,
        #[template_child]
        pub(super) check_button_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub(super) check_button: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) name_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) age_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) spinner: TemplateChild<adw::Spinner>,
        #[template_child]
        pub(super) containers_count_bar: TemplateChild<view::ContainersCountBar>,
        #[template_child]
        pub(super) end_box_revealer: TemplateChild<gtk::Revealer>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for NetworkRow {
        const NAME: &'static str = "PdsNetworkRow";
        type Type = super::NetworkRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action("network-row.activate", None, |widget, _, _| {
                widget.activate();
            });

            klass.install_action_async(ACTION_DELETE, None, async |widget, _, _| {
                widget.delete().await;
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for NetworkRow {
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

            let network_expr = Self::Type::this_expression("network");
            let network_inner_expr = network_expr.chain_property::<model::Network>("inner");
            // let network_name_is_id_expr = network_inner_expr.chain_closure::<bool>(closure!(
            //     |_: Self::Type, inner: model::BoxedNetwork| utils::is_podman_id(&inner.name)
            // ));
            let network_to_be_deleted_expr =
                network_expr.chain_property::<model::Network>("to-be-deleted");
            let container_list_expr =
                network_expr.chain_property::<model::Network>("container-list");

            let selection_mode_expr = network_expr
                .chain_property::<model::Network>("network-list")
                .chain_property::<model::NetworkList>("selection-mode");

            selection_mode_expr.bind(&*self.check_button_revealer, "reveal-child", Some(obj));
            selection_mode_expr
                .chain_closure::<bool>(closure!(|_: Self::Type, is_selection_mode: bool| {
                    !is_selection_mode
                }))
                .bind(&*self.end_box_revealer, "reveal-child", Some(obj));

            gtk::ClosureExpression::new::<String>(
                [
                    network_inner_expr.upcast_ref(),
                    network_to_be_deleted_expr.upcast_ref(),
                ],
                closure!(
                    |_: Self::Type, inner: model::BoxedNetwork, to_be_deleted: bool| {
                        let name = inner.name.as_ref().unwrap();
                        if to_be_deleted {
                            format!("<s>{name}</s>")
                        } else {
                            name.to_owned()
                        }
                    }
                ),
            )
            .bind(&*self.name_label, "label", Some(obj));

            let css_classes = utils::css_classes(&*self.name_label);
            container_list_expr
                .chain_property::<model::SimpleContainerList>("len")
                .chain_closure::<Vec<String>>(closure!(|_: Self::Type, len: u32| {
                    css_classes
                        .iter()
                        .cloned()
                        .chain(if len == 0 {
                            Some(String::from("dim-label"))
                        } else {
                            None
                        })
                        .collect::<Vec<_>>()
                }))
                .bind(&*self.name_label, "css-classes", Some(obj));

            gtk::ClosureExpression::new::<String>(
                [&ticks_expr, &network_inner_expr],
                closure!(|_: Self::Type, _ticks: u64, inner: model::BoxedNetwork| {
                    // Translators: This will resolve to sth. like "{a few minutes} old" or "{15 days} old".
                    gettext!(
                        "{} old",
                        utils::human_friendly_timespan(utils::timespan_now(
                            glib::DateTime::from_iso8601(
                                "2022-10-12T00:00:00Z",
                                // inner.created.as_deref().unwrap(),
                                None
                            )
                            .unwrap()
                            .to_unix(),
                        ))
                    )
                }),
            )
            .bind(&*self.age_label, "label", Some(obj));

            network_expr
                .chain_property::<model::Network>("searching-containers")
                .bind(&self.spinner.get(), "visible", Some(obj));

            container_list_expr.bind(&*self.containers_count_bar, "container-list", Some(obj));

            network_to_be_deleted_expr.watch(
                Some(obj),
                clone!(
                    #[weak]
                    obj,
                    #[strong]
                    network_to_be_deleted_expr,
                    move || {
                        obj.action_set_enabled(
                            ACTION_DELETE,
                            !network_to_be_deleted_expr
                                .evaluate_as::<bool, _>(Some(&obj))
                                .unwrap(),
                        );
                    }
                ),
            );

            if let Some(network) = obj.network() {
                obj.action_set_enabled("network.show-details", !network.to_be_deleted());
                network.connect_notify_local(
                    Some("to-be-deleted"),
                    clone!(
                        #[weak]
                        obj,
                        move |network, _| {
                            obj.action_set_enabled(
                                "network.show-details",
                                !network.to_be_deleted(),
                            );
                        }
                    ),
                );
            }
        }
    }

    impl WidgetImpl for NetworkRow {}
    impl ListBoxRowImpl for NetworkRow {}

    impl NetworkRow {
        pub(super) fn set_network(&self, value: Option<&model::Network>) {
            let obj = &*self.obj();
            if obj.network().as_ref() == value {
                return;
            }

            let mut bindings = self.bindings.borrow_mut();
            while let Some(binding) = bindings.pop() {
                binding.unbind();
            }

            if let Some(network) = value {
                let binding = network
                    .bind_property("selected", &*self.check_button, "active")
                    .flags(glib::BindingFlags::SYNC_CREATE | glib::BindingFlags::BIDIRECTIONAL)
                    .build();

                bindings.push(binding);
            }

            self.network.set(value);
            obj.notify("network")
        }
    }
}

glib::wrapper! {
    pub(crate) struct NetworkRow(ObjectSubclass<imp::NetworkRow>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Buildable, gtk::Actionable, gtk::ConstraintTarget;
}

impl From<&model::Network> for NetworkRow {
    fn from(network: &model::Network) -> Self {
        glib::Object::builder().property("network", network).build()
    }
}

impl NetworkRow {
    pub(crate) fn activate(&self) {
        if let Some(network) = self.network().as_ref() {
            if network
                .network_list()
                .map(|list| list.is_selection_mode())
                .unwrap_or(false)
            {
                network.select();
            } else {
                // utils::navigation_view(self).push(
                //     &adw::NavigationPage::builder()
                //         .title(gettext!(
                //             "Network {}",
                //             utils::format_volume_name(&network.inner().name)
                //         ))
                //         .child(&view::VolumeDetailsPage::from(network))
                //         .build(),
                // );
            }
        }
    }

    pub(crate) async fn delete(&self) {
        view::network::delete_show_confirmation(self, self.network().as_ref()).await;
    }
}
