use adw::prelude::*;
use adw::subclass::prelude::*;
use gettextrs::gettext;
use glib::Properties;
use gtk::gdk;
use gtk::glib;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_GLOBAL_SEARCH: &str = "client-view.global-search";
const ACTION_PANEL_SEARCH: &str = "client-view.panel-search";
const ACTION_SHOW_CONNECTIONS: &str = "client-view.show-connections";
const ACTION_SHOW_ACTIONS: &str = "client-view.show-actions";
const ACTION_CANCEL_OR_DELETE_ACTION: &str = "client-view.cancel-or-delete-action";
const ACTION_CREATE_ENTITY: &str = "client-view.create-entity";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::ClientView)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/client_view.ui")]
    pub(crate) struct ClientView {
        pub(super) settings: utils::PodsSettings,
        pub(super) css_provider: gtk::CssProvider,
        #[property(get, set)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[template_child]
        pub(super) navigation_split_view: TemplateChild<adw::NavigationSplitView>,
        #[template_child]
        pub(super) sidebar_navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub(super) sidebar_list_box: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub(super) content_navigation_view: TemplateChild<adw::NavigationView>,
        #[template_child]
        pub(super) content_navigation_page: TemplateChild<adw::NavigationPage>,
        #[template_child]
        pub(super) panel_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) containers_panel: TemplateChild<view::ContainersPanel>,
        #[template_child]
        pub(super) pods_panel: TemplateChild<view::PodsPanel>,
        #[template_child]
        pub(super) images_panel: TemplateChild<view::ImagesPanel>,
        #[template_child]
        pub(super) volumes_panel: TemplateChild<view::VolumesPanel>,
        #[template_child]
        pub(super) color_bin: TemplateChild<adw::Bin>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ClientView {
        const NAME: &'static str = "PdsClientView";
        type Type = super::ClientView;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();

            klass.add_binding_action(
                gdk::Key::F,
                gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK,
                ACTION_GLOBAL_SEARCH,
            );
            klass.install_action(ACTION_GLOBAL_SEARCH, None, |widget, _, _| {
                widget.global_search();
            });

            klass.add_binding_action(
                gdk::Key::F,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_PANEL_SEARCH,
            );
            klass.install_action(ACTION_PANEL_SEARCH, None, |widget, _, _| {
                widget.toggle_panel_search();
            });

            klass.install_action(ACTION_SHOW_CONNECTIONS, None, |widget, _, _| {
                widget.show_connections();
            });

            klass.install_action(ACTION_SHOW_ACTIONS, None, |widget, _, _| {
                widget.show_actions();
            });

            klass.install_action(
                ACTION_CANCEL_OR_DELETE_ACTION,
                Some(glib::VariantTy::UINT32),
                |widget, _, data| {
                    widget.cancel_or_delete_action(data);
                },
            );

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                ACTION_CREATE_ENTITY,
            );
            klass.install_action(ACTION_CREATE_ENTITY, None, |widget, _, _| {
                widget.create_entity();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ClientView {
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

            self.settings
                .bind(
                    "last-used-view",
                    &self.panel_stack.get(),
                    "visible-child-name",
                )
                .build();

            self.sidebar_list_box.set_header_func(|row, _| {
                row.set_header(
                    row.child()
                        .filter(gtk::Widget::is::<view::InfoRow>)
                        .map(|_| {
                            gtk::Separator::builder()
                                .orientation(gtk::Orientation::Horizontal)
                                .hexpand(true)
                                .build()
                        })
                        .as_ref(),
                );
            });

            self.color_bin
                .style_context()
                .add_provider(&self.css_provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);

            self.on_panel_stack_notify_visible_child();
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for ClientView {}

    #[gtk::template_callbacks]
    impl ClientView {
        #[template_callback]
        fn on_notify_client(&self) {
            self.exit_panel_search_mode();
            self.sidebar_navigation_view.pop_to_tag("home");
            self.content_navigation_view.pop_to_tag("home");

            if let Some(client) = self.obj().client() {
                self.set_background(client.connection().rgb());
            }
        }

        #[template_callback]
        fn on_navigation_split_view_notify_show_content(&self) {
            if self.navigation_split_view.is_collapsed() {
                self.sidebar_list_box.select_row(gtk::ListBoxRow::NONE);
                self.exit_selection_mode();
            }
        }

        #[template_callback]
        fn on_navigation_split_view_notify_collapsed(&self) {
            if !self.navigation_split_view.is_collapsed() {
                self.navigation_split_view.set_show_content(true);

                self.sidebar_list_box.select_row(
                    self.sidebar_list_box
                        .row_at_index(
                            match self.panel_stack.visible_child_name().unwrap().as_str() {
                                "containers" => 0,
                                "pods" => 1,
                                "images" => 2,
                                "volumes" => 3,
                                "info" => 4,
                                "search" => 5,
                                _ => unreachable!(),
                            },
                        )
                        .as_ref(),
                );
            }
        }

        #[template_callback]
        fn on_sidebar_row_activated(&self, row: Option<&gtk::ListBoxRow>) {
            if let Some(row) = row {
                let child = row.child().unwrap();

                self.panel_stack
                    .set_visible_child_name(if child.is::<view::ContainersRow>() {
                        "containers"
                    } else if child.is::<view::PodsRow>() {
                        "pods"
                    } else if child.is::<view::ImagesRow>() {
                        "images"
                    } else if child.is::<view::VolumesRow>() {
                        "volumes"
                    } else if child.is::<view::InfoRow>() {
                        "info"
                    } else {
                        unreachable!()
                    });

                self.content_navigation_view.pop_to_tag("home");
                self.navigation_split_view.set_show_content(true);
            }
        }

        #[template_callback]
        fn on_actions_cleared(&self) {
            self.sidebar_navigation_view.pop_to_tag("home");
        }

        #[template_callback]
        fn on_panel_stack_notify_visible_child(&self) {
            self.content_navigation_page.set_title(&match self
                .panel_stack
                .visible_child_name()
                .unwrap()
                .as_str()
            {
                "containers" => gettext("Containers"),
                "pods" => gettext("Pods"),
                "images" => gettext("Images"),
                "volumes" => gettext("Volumes"),
                "info" => gettext("Info"),
                "search" => gettext("Search"),
                _ => unreachable!(),
            });

            self.exit_selection_mode();
        }

        fn exit_panel_search_mode(&self) {
            self.containers_panel.set_search_mode(false);
            self.pods_panel.set_search_mode(false);
            self.images_panel.set_search_mode(false);
            self.volumes_panel.set_search_mode(false);
        }

        fn exit_selection_mode(&self) {
            self.containers_panel.exit_selection_mode();
            self.pods_panel.exit_selection_mode();
            self.images_panel.exit_selection_mode();
            self.volumes_panel.exit_selection_mode();
        }

        fn set_background(&self, bg_color: Option<gdk::RGBA>) {
            match bg_color {
                Some(color) => {
                    self.css_provider
                        .load_from_data(&format!("widget {{ background: {color}; }}",));
                    self.color_bin.set_visible(true);
                }
                None => {
                    self.color_bin.set_visible(false);
                }
            }
        }
    }
}

glib::wrapper! {
    pub(crate) struct ClientView(ObjectSubclass<imp::ClientView>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl ClientView {
    pub(crate) fn navigation_view(&self) -> &adw::NavigationView {
        &self.imp().content_navigation_view
    }

    pub(crate) fn global_search(&self) {
        self.imp()
            .sidebar_list_box
            .row_at_index(7)
            .as_ref()
            .unwrap()
            .activate();
    }

    pub(crate) fn toggle_panel_search(&self) {
        let imp = self.imp();

        match imp.panel_stack.visible_child_name().unwrap().as_str() {
            "containers" => imp.containers_panel.toggle_search_mode(),
            "pods" => imp.pods_panel.toggle_search_mode(),
            "images" => imp.images_panel.toggle_search_mode(),
            "volumes" => imp.volumes_panel.toggle_search_mode(),
            _ => {}
        }
    }

    pub(crate) fn show_connections(&self) {
        self.imp()
            .sidebar_navigation_view
            .push_by_tag("connections");
    }

    pub(crate) fn show_actions(&self) {
        self.imp().sidebar_navigation_view.push_by_tag("actions");
    }

    pub(crate) fn create_entity(&self) {
        if self.client().is_some() {
            let imp = self.imp();

            if imp.containers_panel.is_mapped() {
                imp.containers_panel.create_container();
            } else if imp.pods_panel.is_mapped() {
                imp.pods_panel.create_pod();
            } else if imp.images_panel.is_mapped() {
                imp.images_panel.show_download_page();
            } else if imp.volumes_panel.is_mapped() {
                imp.volumes_panel.create_volume();
            }
        }
    }

    pub(crate) fn cancel_or_delete_action(&self, data: Option<&glib::Variant>) {
        if let Some(action_list) = self.client().as_ref().map(model::Client::action_list) {
            let action_num: u32 = data.unwrap().get().unwrap();

            if let Some(action) = action_list.get(action_num) {
                if action.state() == model::ActionState::Ongoing {
                    action.cancel();
                } else {
                    action_list.remove(action_num);
                }
            }
        }
    }
}
