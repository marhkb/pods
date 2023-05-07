use adw::subclass::prelude::AdwApplicationWindowImpl;
use adw::traits::BinExt;
use gettextrs::gettext;
use gettextrs::ngettext;
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::glib::clone;
use gtk::glib::closure;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::application::Application;
use crate::config;
use crate::model;
use crate::model::SelectableListExt;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/window.ui")]
    pub(crate) struct Window {
        pub(super) settings: utils::PodsSettings,
        pub(super) connection_manager: model::ConnectionManager,
        #[template_child]
        pub(super) toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub(super) main_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) connections_flap: TemplateChild<adw::Flap>,
        #[template_child]
        pub(super) actions_flap: TemplateChild<adw::Flap>,
        #[template_child]
        pub(super) header_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) leaflet: TemplateChild<adw::Leaflet>,
        #[template_child]
        pub(super) search_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub(super) title_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) title: TemplateChild<adw::ViewSwitcherTitle>,
        #[template_child]
        pub(super) search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub(super) menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) selection_mode_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub(super) selected_items_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) selected_images_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) selected_containers_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) selected_pods_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub(super) search_stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) panel_stack: TemplateChild<adw::ViewStack>,
        #[template_child]
        pub(super) containers_view_stack_page: TemplateChild<adw::ViewStackPage>,
        #[template_child]
        pub(super) containers_panel: TemplateChild<view::ContainersPanel>,
        #[template_child]
        pub(super) pods_view_stack_page: TemplateChild<adw::ViewStackPage>,
        #[template_child]
        pub(super) pods_panel: TemplateChild<view::PodsPanel>,
        #[template_child]
        pub(super) images_view_stack_page: TemplateChild<adw::ViewStackPage>,
        #[template_child]
        pub(super) images_panel: TemplateChild<view::ImagesPanel>,
        #[template_child]
        pub(super) switcher_bar: TemplateChild<adw::ViewSwitcherBar>,
        #[template_child]
        pub(super) search_panel: TemplateChild<view::SearchPanel>,
        #[template_child]
        pub(super) leaflet_overlay: TemplateChild<view::LeafletOverlay>,
        #[template_child]
        pub(super) statusbar: TemplateChild<view::Statusbar>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "PdsWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            // Initialize all classes here
            view::ActionPage::static_type();
            view::ActionRow::static_type();
            view::ActionsSidebar::static_type();
            view::BackNavigationControls::static_type();
            view::CircularProgressBar::static_type();
            view::ConnectionChooserPage::static_type();
            view::ConnectionCustomInfoDialog::static_type();
            view::ConnectionRow::static_type();
            view::ConnectionSwitcher::static_type();
            view::ConnectionsSidebar::static_type();
            view::ContainerCommitPage::static_type();
            view::ContainerFilesGetPage::static_type();
            view::ContainerFilesPutPage::static_type();
            view::ContainerHealthCheckPage::static_type();
            view::ContainerLogPage::static_type();
            view::ContainerMenuButton::static_type();
            view::ContainerPropertiesGroup::static_type();
            view::ContainerResourcesQuickReferenceGroup::static_type();
            view::ContainerTty::static_type();
            view::ContainerTtyPage::static_type();
            view::ContainersCountBar::static_type();
            view::ContainersGroup::static_type();
            view::ContainersPanel::static_type();
            view::HealthCheckLogRow::static_type();
            view::ImageBuildPage::static_type();
            view::ImageHistoryPage::static_type();
            view::ImageLocalComboRow::static_type();
            view::ImageMenuButton::static_type();
            view::ImageSearchResponseRow::static_type();
            view::ImagesPanel::static_type();
            view::PodMenuButton::static_type();
            view::PodRow::static_type();
            view::PodsPanel::static_type();
            view::PodsPrunePage::static_type();
            view::PropertyRow::static_type();
            view::PropertyWidgetRow::static_type();
            view::RandomNameEntryRow::static_type();
            view::RepoTagAddDialog::static_type();
            view::RepoTagRow::static_type();
            view::RepoTagSimpleRow::static_type();
            view::ScalableTextView::static_type();
            view::ScalableTextViewPage::static_type();
            view::SourceViewSearchWidget::static_type();
            view::Spinner::static_type();
            view::Statusbar::static_type();
            view::TextSearchEntry::static_type();
            view::WelcomePage::static_type();
            view::ZoomControl::static_type();

            klass.install_action("win.close", None, |widget, _, _| {
                widget.close();
            });
            klass.add_binding_action(
                gdk::Key::W,
                gdk::ModifierType::CONTROL_MASK,
                "win.close",
                None,
            );

            klass.add_binding_action(
                gdk::Key::Home,
                gdk::ModifierType::ALT_MASK,
                "win.navigate-home",
                None,
            );
            klass.install_action("win.navigate-home", None, |widget, _, _| {
                widget.navigate_home();
            });

            klass.install_action("win.enter-selection-mode", None, |widget, _, _| {
                widget.enter_selection_mode();
            });
            klass.install_action("win.exit-selection-mode", None, |widget, _, _| {
                widget.exit_selection_mode();
            });
            klass.install_action("win.select-all", None, |widget, _, _| {
                widget.select_all();
            });
            klass.install_action("win.select-none", None, |widget, _, _| {
                widget.select_none();
            });

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK | gdk::ModifierType::SHIFT_MASK,
                "win.new-connection",
                None,
            );
            klass.install_action("win.new-connection", None, |widget, _, _| {
                widget.add_connection();
            });

            klass.install_action(
                "win.cancel-or-delete-action",
                Some("u"),
                |widget, _, data| {
                    widget.cancel_or_delete_action(data);
                },
            );

            klass.add_binding_action(
                gdk::Key::N,
                gdk::ModifierType::CONTROL_MASK,
                "entity.create",
                None,
            );
            klass.install_action("entity.create", None, |widget, _, _| {
                widget.create_entity();
            });

            klass.install_action("win.remove-connection", Some("s"), |widget, _, data| {
                let uuid: String = data.unwrap().get().unwrap();
                widget.remove_connection(&uuid);
            });

            klass.install_action("win.show-podman-info", None, |widget, _, _| {
                widget.show_podman_info_dialog();
            });

            klass.add_binding_action(
                gdk::Key::F,
                gdk::ModifierType::CONTROL_MASK,
                "win.enter-search",
                None,
            );

            klass.install_action("win.enter-search", None, |widget, _, _| {
                widget.enter_search();
            });

            klass.add_binding_action(
                gdk::Key::Escape,
                gdk::ModifierType::empty(),
                "win.exit-search",
                None,
            );

            klass.install_action("win.exit-search", None, |widget, _, _| {
                widget.exit_search();
            });

            klass.install_action("win.exit-connections-sidebar", None, |widget, _, _| {
                widget.exit_connections_sidebar();
            });

            klass.install_action("win.exit-actions-sidebar", None, |widget, _, _| {
                widget.exit_actions_sidebar();
            });
        }

        // You must call `Widget`'s `init_template()` within `instance_init()`.
        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Window {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::ConnectionManager>(
                        "connection-manager",
                    )
                    .read_only()
                    .build(),
                    glib::ParamSpecObject::builder::<gtk::Stack>("title-stack")
                        .read_only()
                        .build(),
                    glib::ParamSpecObject::builder::<adw::ViewStack>("panel-stack")
                        .read_only()
                        .build(),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "connection-manager" => self.obj().connection_manager().to_value(),
                "title-stack" => self.title_stack.to_value(),
                "panel-stack" => self.panel_stack.to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            if config::PROFILE == "Devel" {
                obj.add_css_class("devel");
            }

            obj.load_settings();
            obj.setup_theme_selector();
            obj.setup_search();
            obj.setup_panels();

            let client_expr = Self::Type::this_expression("connection-manager")
                .chain_property::<model::ConnectionManager>("client");
            let title_visible_expr = Self::Type::this_expression("title-stack")
                .chain_property::<gtk::Stack>("visible-child-name")
                .chain_closure::<bool>(closure!(|_: Self::Type, visible_child_name: &str| {
                    visible_child_name == "title"
                }));

            title_visible_expr.bind(&*self.menu_button, "visible", Some(obj));

            let panel_stack_visible_child_name_expr =
                Self::Type::this_expression("panel-stack")
                    .chain_property::<adw::ViewStack>("visible-child-name");

            panel_stack_visible_child_name_expr.watch(
                Some(obj),
                clone!(@weak obj => move || {
                    let imp = obj.imp();
                    match imp.panel_stack.visible_child_name().as_deref() {
                        Some("containers") => imp.containers_view_stack_page.set_needs_attention(false),
                        Some("pods") => imp.pods_view_stack_page.set_needs_attention(false),
                        Some("images") => imp.images_view_stack_page.set_needs_attention(false),
                        _ => {}
                    }
                }),
            );

            gtk::ClosureExpression::new::<bool>(
                [
                    title_visible_expr.upcast_ref(),
                    &panel_stack_visible_child_name_expr.upcast(),
                    &client_expr
                        .chain_property::<model::Client>("image-list")
                        .chain_property::<model::ImageList>("len")
                        .upcast(),
                    &client_expr
                        .chain_property::<model::Client>("container-list")
                        .chain_property::<model::ContainerList>("len")
                        .upcast(),
                    &client_expr
                        .chain_property::<model::Client>("pod-list")
                        .chain_property::<model::PodList>("len")
                        .upcast(),
                ],
                closure!(|_: Self::Type,
                          title_visible: bool,
                          visible_panel: &str,
                          images: u32,
                          containers: u32,
                          pods: u32| {
                    title_visible
                        && match visible_panel {
                            "images" => images > 0,
                            "containers" => containers > 0,
                            "pods" => pods > 0,
                            _ => unreachable!(),
                        }
                }),
            )
            .bind(&*self.selection_mode_button, "visible", Some(obj));

            self.connection_manager.connect_notify_local(
                Some("client"),
                clone!(@weak obj => move |manager, _| {
                    let imp = obj.imp();
                    imp.leaflet_overlay.hide_details();

                    match manager.client() {
                        Some(client) => client.check_service(
                            clone!(@weak obj, @weak client => move || {
                                let imp = obj.imp();

                                imp.statusbar.set_background(client.connection().rgb());

                                imp.search_button.set_active(false);
                                imp.main_stack.set_visible_child_full("client", gtk::StackTransitionType::None);

                                obj.exit_connections_sidebar();
                                obj.exit_selection_mode();

                                imp.containers_view_stack_page.set_needs_attention(false);
                                client.container_list().connect_notify_local(
                                    Some("len"),
                                    clone!(@weak obj => move |list, _|
                                {
                                    let imp = obj.imp();
                                    if imp.panel_stack.visible_child_name().as_deref() != Some("containers")
                                        && list.initialized()
                                    {
                                        imp.containers_view_stack_page.set_needs_attention(true);
                                    }
                                }));

                                imp.pods_view_stack_page.set_needs_attention(false);
                                client.pod_list().connect_notify_local(
                                    Some("len"),
                                    clone!(@weak obj => move |list, _|
                                {
                                    let imp = obj.imp();
                                    if imp.panel_stack.visible_child_name().as_deref() != Some("pods")
                                        && list.initialized()
                                    {
                                        imp.pods_view_stack_page.set_needs_attention(true);
                                    }
                                }));

                                imp.images_view_stack_page.set_needs_attention(false);
                                client.image_list().connect_notify_local(
                                    Some("len"),
                                    clone!(@weak obj => move |list, _|
                                {
                                    let imp = obj.imp();
                                    if imp.panel_stack.visible_child_name().as_deref() != Some("images")
                                        && list.initialized()
                                    {
                                        imp.images_view_stack_page.set_needs_attention(true);
                                    }
                                }));
                            }),
                            clone!(@weak obj => move |e| obj.client_err_op(e)),
                            clone!(@weak obj, @weak manager => move |e| {
                                utils::show_error_toast(
                                    obj.imp().toast_overlay.upcast_ref(),
                                    "Connection lost",
                                    &e.to_string()
                                );
                                manager.unset_client();
                            }),
                        ),
                        None => {
                            imp.statusbar.set_background(None);
                            imp.main_stack.set_visible_child_full(
                                if manager.n_items() > 0 {
                                    "connection-chooser"
                                } else {
                                    "welcome"
                                },
                                gtk::StackTransitionType::Crossfade
                            );
                        }
                    }
                }),
            );

            self.connection_manager
                .setup(clone!(@weak obj => move |result| match result {
                    Ok(_) => if obj.connection_manager().n_items() == 0 {
                        obj.imp().main_stack.set_visible_child_name("welcome");
                    }
                    Err(e) => obj.on_connection_manager_setup_error(e),
                }));
        }
    }

    impl WidgetImpl for Window {
        fn realize(&self) {
            let dialog = self.obj().help_overlay().unwrap();

            let controller = gtk::EventControllerKey::new();
            controller.connect_key_pressed(clone!(
                @weak dialog => @default-return glib::signal::Inhibit(true), move |_, key, _, modifier| {
                    if key == gdk::Key::w && modifier == gdk::ModifierType::CONTROL_MASK {
                        dialog.close();
                    }
                    glib::signal::Inhibit(false)
                }
            ));
            dialog.add_controller(controller);

            self.parent_realize();
        }
    }

    impl WindowImpl for Window {
        // Save window state on delete event
        fn close_request(&self) -> gtk::Inhibit {
            let window = &*self.obj();

            if let Err(err) = window.save_window_size() {
                log::warn!("Failed to save window state, {}", &err);
            }

            if view::show_ongoing_actions_warning_dialog(
                window.upcast_ref(),
                &self.connection_manager,
                &gettext("Confirm Exiting The Application"),
            ) {
                self.parent_close_request()
            } else {
                gtk::Inhibit(true)
            }
        }
    }

    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

glib::wrapper! {
    pub(crate) struct Window(ObjectSubclass<imp::Window>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionMap, gio::ActionGroup, gtk::Root;
}

impl Window {
    pub(crate) fn new(app: &Application) -> Self {
        glib::Object::builder().property("application", app).build()
    }

    fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let (width, height) = self.default_size();

        let imp = self.imp();
        imp.settings.set_int("window-width", width)?;
        imp.settings.set_int("window-height", height)?;
        imp.settings
            .set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    fn load_settings(&self) {
        let settings = &*self.imp().settings;

        let width = settings.int("window-width");
        let height = settings.int("window-height");
        let is_maximized = settings.boolean("is-maximized");

        self.set_default_size(width, height);

        if is_maximized {
            self.maximize();
        }

        settings
            .bind(
                "last-used-view",
                &*self.imp().panel_stack,
                "visible-child-name",
            )
            .build();
    }

    fn setup_theme_selector(&self) {
        let imp = self.imp();

        let style_manager = adw::StyleManager::default();

        imp.settings
            .bind("color-scheme", &style_manager, "color-scheme")
            .get()
            .set_mapping(|value, _| Some(color_scheme_to_str(value.get().unwrap()).to_variant()))
            .set()
            .mapping(|variant, _| Some(str_to_color_scheme(variant.str().unwrap()).to_value()))
            .build();

        let action = gio::SimpleAction::new_stateful(
            "theme",
            Some(glib::VariantTy::STRING),
            color_scheme_to_str(style_manager.color_scheme()).to_variant(),
        );
        action.connect_activate(clone!(@weak self as obj => move |_, param| {
            adw::StyleManager::default()
                .set_color_scheme(str_to_color_scheme(param.unwrap().str().unwrap()));
        }));
        self.add_action(&action);

        adw::StyleManager::default().connect_color_scheme_notify(
            clone!(@weak action => move |style_manager| {
                action.set_state(color_scheme_to_str(style_manager.color_scheme()).to_variant());
            }),
        );

        let popover_menu = imp
            .menu_button
            .popover()
            .unwrap()
            .downcast::<gtk::PopoverMenu>()
            .unwrap();

        popover_menu.add_child(
            &panel::ThemeSelector::builder()
                .action_name("win.theme")
                .build(),
            "theme",
        );
    }

    fn setup_search(&self) {
        let imp = self.imp();

        imp.search_button
            .connect_active_notify(clone!(@weak self as obj => move |button| {
                let imp = obj.imp();

                if button.is_active() {
                    imp.search_entry.set_text("");
                    imp.title_stack.set_visible_child(&*imp.search_entry);
                    imp.search_entry.grab_focus();
                    imp.search_stack.set_visible_child(&*imp.search_panel);
                } else {
                    imp.title_stack.set_visible_child(&*imp.title);
                    imp.search_stack.set_visible_child_name("main");
                }
            }));

        imp.search_entry
            .connect_text_notify(clone!(@weak self as obj => move |entry| {
                if obj.is_search_activatable() {
                    let imp = obj.imp();
                    imp.search_panel.set_term(entry.text());
                    if !entry.text().is_empty() {
                        imp.search_button.set_active(true);
                    }
                }
            }));

        let key_controller = gtk::EventControllerKey::new();
        key_controller.connect_key_pressed(clone!(
            @weak self as obj => @default-return glib::signal::Inhibit(false), move |_, key, _, _| {
                glib::signal::Inhibit(if key == gdk::Key::Escape {
                    obj.exit_search();
                    true
                } else {
                    false
                })
            }
        ));
        imp.search_entry.add_controller(key_controller);
    }

    fn setup_panels(&self) {
        let imp = self.imp();

        gtk::ClosureExpression::new::<bool>(
            [
                imp.title.property_expression("title-visible"),
                imp.header_stack.property_expression("visible-child-name"),
            ],
            closure!(|_: Option<glib::Object>,
                      title_visible: bool,
                      visible_child: Option<&str>| {
                title_visible && visible_child == Some("main")
            }),
        )
        .bind(&*imp.switcher_bar, "reveal", glib::Object::NONE);

        view::ContainersPanel::this_expression("container-list")
            .chain_property::<model::ContainerList>("num-selected")
            .chain_closure::<String>(closure!(
                |_: view::ContainersPanel, selected: u32| ngettext!(
                    "{} Selected Container",
                    "{} Selected Containers",
                    selected,
                    selected
                )
            ))
            .bind(
                &*imp.selected_containers_button,
                "label",
                Some(&*imp.containers_panel),
            );

        view::PodsPanel::this_expression("pod-list")
            .chain_property::<model::PodList>("num-selected")
            .chain_closure::<String>(closure!(|_: view::PodsPanel, selected: u32| ngettext!(
                "{} Selected Pod",
                "{} Selected Pods",
                selected,
                selected
            )))
            .bind(&*imp.selected_pods_button, "label", Some(&*imp.pods_panel));

        view::ImagesPanel::this_expression("image-list")
            .chain_property::<model::ImageList>("num-selected")
            .chain_closure::<String>(closure!(|_: view::ImagesPanel, selected: u32| ngettext!(
                "{} Selected Image",
                "{} Selected Images",
                selected,
                selected
            )))
            .bind(
                &*imp.selected_images_button,
                "label",
                Some(&*imp.images_panel),
            );

        imp.containers_panel
            .connect_exit_selection_mode(clone!(@weak self as obj => move |_| {
                obj.imp().header_stack.set_visible_child_name("main");
            }));
        imp.pods_panel
            .connect_exit_selection_mode(clone!(@weak self as obj => move |_| {
                obj.imp().header_stack.set_visible_child_name("main");
            }));
        imp.images_panel
            .connect_exit_selection_mode(clone!(@weak self as obj => move |_| {
                obj.imp().header_stack.set_visible_child_name("main");
            }));
    }

    pub(crate) fn connection_manager(&self) -> model::ConnectionManager {
        self.imp().connection_manager.clone()
    }

    fn on_connection_manager_setup_error(&self, e: impl ToString) {
        let imp = self.imp();
        imp.main_stack
            .set_visible_child_name(if imp.connection_manager.n_items() > 0 {
                "connection-chooser"
            } else {
                "welcome"
            });

        utils::show_error_toast(
            imp.toast_overlay.upcast_ref(),
            "Connection lost",
            &e.to_string(),
        );
    }

    fn is_showing_overlay(&self) -> bool {
        matches!(
            self.imp().leaflet.visible_child_name().as_deref(),
            Some("overlay")
        )
    }

    fn enter_selection_mode(&self) {
        let imp = self.imp();

        if let Some(name) = imp.panel_stack.visible_child_name() {
            match name.as_str() {
                "containers" => {
                    let list = imp.containers_panel.container_list().unwrap();
                    if list.len() > 0 {
                        imp.header_stack.set_visible_child_name("selection");
                        list.set_selection_mode(true);
                    }
                }
                "pods" => {
                    let list = imp.pods_panel.pod_list().unwrap();
                    if list.len() > 0 {
                        imp.header_stack.set_visible_child_name("selection");
                        list.set_selection_mode(true);
                    }
                }
                "images" => {
                    let list = imp.images_panel.image_list().unwrap();
                    if list.len() > 0 {
                        imp.header_stack.set_visible_child_name("selection");
                        list.set_selection_mode(true);
                    }
                }
                _ => unreachable!(),
            }
        }
    }

    fn exit_selection_mode(&self) {
        let imp = self.imp();

        imp.header_stack.set_visible_child_name("main");

        if let Some(list) = imp.containers_panel.container_list() {
            list.set_selection_mode(false);
        }
        if let Some(list) = imp.pods_panel.pod_list() {
            list.set_selection_mode(false);
        }
        if let Some(list) = imp.images_panel.image_list() {
            list.set_selection_mode(false);
        }
    }

    fn select_all(&self) {
        let imp = self.imp();

        if let Some(list) = imp
            .containers_panel
            .container_list()
            .filter(|list| list.is_selection_mode())
        {
            list.select_all()
        } else if let Some(list) = imp
            .pods_panel
            .pod_list()
            .filter(|list| list.is_selection_mode())
        {
            list.select_all()
        } else if let Some(list) = imp
            .images_panel
            .image_list()
            .filter(|list| list.is_selection_mode())
        {
            list.select_all()
        }
    }

    fn select_none(&self) {
        let imp = self.imp();

        if let Some(list) = imp
            .containers_panel
            .container_list()
            .filter(|list| list.is_selection_mode())
        {
            list.select_none()
        } else if let Some(list) = imp
            .pods_panel
            .pod_list()
            .filter(|list| list.is_selection_mode())
        {
            list.select_none()
        } else if let Some(list) = imp
            .images_panel
            .image_list()
            .filter(|list| list.is_selection_mode())
        {
            list.select_none()
        }
    }

    fn is_search_activatable(&self) -> bool {
        !self.is_showing_overlay()
            && self.imp().header_stack.visible_child_name().as_deref() == Some("main")
    }

    fn navigate_home(&self) {
        self.leaflet_overlay().hide_details();
    }

    fn add_connection(&self) {
        utils::show_dialog(
            self.upcast_ref(),
            view::ConnectionCreationPage::from(&self.connection_manager()).upcast_ref(),
        );
    }

    fn cancel_or_delete_action(&self, data: Option<&glib::Variant>) {
        if let Some(action_list) = self
            .connection_manager()
            .client()
            .as_ref()
            .map(model::Client::action_list)
        {
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

    fn create_entity(&self) {
        let imp = self.imp();
        let leaflet_overlay = &*imp.leaflet_overlay;

        if self.connection_manager().client().is_some() && leaflet_overlay.child().is_none() {
            imp.panel_stack
                .visible_child_name()
                .map(|name| match name.as_str() {
                    "containers" => imp
                        .containers_panel
                        .activate_action(view::ContainersPanel::action_create_container(), None),
                    "pods" => imp
                        .pods_panel
                        .activate_action(view::PodsPanel::action_create_pod(), None),
                    "images" => imp
                        .images_panel
                        .activate_action(view::ImagesPanel::action_pull_image(), None),
                    _ => unreachable!(),
                });
        }
    }

    fn remove_connection(&self, uuid: &str) {
        self.connection_manager().remove_connection(uuid);
    }

    fn show_podman_info_dialog(&self) {
        if let Some(client) = self.connection_manager().client() {
            let dialog = view::InfoDialog::from(&client);
            dialog.set_transient_for(Some(self));
            dialog.present();
        }
    }

    fn enter_search(&self) {
        let imp = self.imp();
        if self.is_search_activatable() {
            imp.search_button.set_active(true);
            imp.search_entry.delete_text(0, -1);
            imp.search_entry.grab_focus();
        }
    }

    fn exit_search(&self) {
        if self.is_search_activatable() {
            self.imp().search_button.set_active(false);
        }
    }

    pub(crate) fn exit_connections_sidebar(&self) {
        self.imp().connections_flap.set_reveal_flap(false);
    }

    pub(crate) fn exit_actions_sidebar(&self) {
        self.imp().actions_flap.set_reveal_flap(false);
    }

    fn client_err_op(&self, e: model::ClientError) {
        self.show_toast(
            adw::Toast::builder()
                .title(gettext!(
                    "Error on loading {}",
                    match e {
                        model::ClientError::Images => gettext("images"),
                        model::ClientError::Containers => gettext("containers"),
                        model::ClientError::Pods => gettext("pods"),
                    }
                ))
                .timeout(3)
                .priority(adw::ToastPriority::High)
                .build(),
        );
    }

    pub(crate) fn show_toast(&self, toast: adw::Toast) {
        self.imp().toast_overlay.add_toast(toast);
    }

    pub(crate) fn leaflet_overlay(&self) -> &view::LeafletOverlay {
        &self.imp().leaflet_overlay
    }
}

fn str_to_color_scheme(scheme: &str) -> adw::ColorScheme {
    match scheme {
        "light" => adw::ColorScheme::ForceLight,
        "dark" => adw::ColorScheme::ForceDark,
        _ => adw::ColorScheme::Default,
    }
}

fn color_scheme_to_str(scheme: adw::ColorScheme) -> &'static str {
    match scheme {
        adw::ColorScheme::ForceDark | adw::ColorScheme::PreferDark => "dark",
        adw::ColorScheme::ForceLight | adw::ColorScheme::PreferLight => "light",
        _ => "default",
    }
}
