use gtk::glib;
use gtk::glib::clone;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::utils;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/theme-selector.ui")]
    pub(crate) struct ThemeSelector {
        pub(super) settings: utils::PodsSettings,
        #[template_child]
        pub(super) follow: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) light: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub(super) dark: TemplateChild<gtk::CheckButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ThemeSelector {
        const NAME: &'static str = "ThemeSelector";
        type Type = super::ThemeSelector;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);

            klass.set_css_name("theme-selector");
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ThemeSelector {
        fn constructed(&self, obj: &Self::Type) {
            self.parent_constructed(obj);

            let style_manager = adw::StyleManager::default();

            self.settings
                .bind("style-variant", &style_manager, "color-scheme")
                .mapping(|variant, _| {
                    Some(
                        match variant.get::<String>().unwrap().as_str() {
                            "follow" => adw::ColorScheme::Default,
                            "dark" => adw::ColorScheme::ForceDark,
                            _ => adw::ColorScheme::ForceLight,
                        }
                        .to_value(),
                    )
                })
                .build();

            obj.setup_check_button(&*self.follow, "follow");
            obj.setup_check_button(&*self.light, "light");
            obj.setup_check_button(&*self.dark, "dark");

            obj.on_notify_system_supports_color_schemes(&style_manager);
            style_manager.connect_system_supports_color_schemes_notify(
                clone!(@weak obj => move |style_manager| {
                    obj.on_notify_system_supports_color_schemes(style_manager);
                }),
            );

            obj.on_notify_dark(&style_manager);
            style_manager.connect_dark_notify(clone!(@weak obj => move |style_manager| {
                obj.on_notify_dark(style_manager);
            }));
        }

        fn dispose(&self, _obj: &Self::Type) {
            self.follow.unparent();
            self.light.unparent();
            self.dark.unparent();
        }
    }

    impl WidgetImpl for ThemeSelector {}
}

glib::wrapper! {
    pub(crate) struct ThemeSelector(ObjectSubclass<imp::ThemeSelector>)
        @extends gtk::Widget;
}

impl Default for ThemeSelector {
    fn default() -> Self {
        glib::Object::new(&[]).expect("Failed to create ThemeSelector")
    }
}

impl ThemeSelector {
    fn setup_check_button(&self, button: &gtk::CheckButton, theme: &'static str) {
        if self.imp().settings.get::<String>("style-variant").as_str() == theme {
            button.set_active(true);
        }
        button.connect_active_notify(clone!(@weak self as obj => move |button| {
            if button.is_active() {
                obj.imp().settings.set_string(
                    "style-variant",
                    theme
                ).unwrap();
            }
        }));
    }

    fn on_notify_system_supports_color_schemes(&self, style_manager: &adw::StyleManager) {
        self.imp()
            .follow
            .set_visible(style_manager.system_supports_color_schemes());
    }

    fn on_notify_dark(&self, style_manager: &adw::StyleManager) {
        if style_manager.is_dark() {
            self.add_css_class("dark")
        } else {
            self.remove_css_class("dark")
        }
    }
}
