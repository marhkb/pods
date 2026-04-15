use adw::prelude::*;
use adw::subclass::prelude::*;
use glib::Properties;
use gtk::CompositeTemplate;
use gtk::glib;
use gtk::glib::closure;

use crate::model;
use crate::utils;
use crate::view;

const ACTION_SHOW_DETAILS: &str = "info-panel.show-details";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::InfoPanel)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/info_panel.ui")]
    pub(crate) struct InfoPanel {
        #[property(get, set, nullable)]
        pub(super) client: glib::WeakRef<model::Client>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) memory_row: TemplateChild<adw::ActionRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for InfoPanel {
        const NAME: &'static str = "PdsInfoPanel";
        type Type = super::InfoPanel;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();

            klass.install_action(ACTION_SHOW_DETAILS, None, |widget, _, _| {
                widget.show_details();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for InfoPanel {
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

            let client_expr = Self::Type::this_expression("client");
            let client_info_expr = client_expr.chain_property::<model::Client>("info");
            let client_info_memory_expr = client_info_expr.chain_property::<model::Info>("memory");
            let client_info_memory_formatted_expr = client_info_memory_expr
                .chain_closure::<String>(closure!(|_: Self::Type, storage_size: i64| {
                    glib::format_size(storage_size as u64)
                }));

            client_info_expr
                .chain_closure::<String>(closure!(|_: Self::Type, info: Option<&model::Info>| info
                    .map(|_| "loaded")
                    .unwrap_or("loading")))
                .bind(&*self.stack, "visible-child-name", Some(obj));
            client_info_memory_formatted_expr.bind(&*self.memory_row, "subtitle", Some(obj));
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for InfoPanel {}
}

glib::wrapper! {
    pub(crate) struct InfoPanel(ObjectSubclass<imp::InfoPanel>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl InfoPanel {
    pub(crate) fn show_details(&self) {
        let Some(info) = self.client().and_then(|client| client.info()) else {
            return;
        };

        let weak_ref = glib::WeakRef::new();
        weak_ref.set(Some(&info));

        utils::navigation_view(self).push(
            &adw::NavigationPage::builder()
                .child(&view::ScalableTextViewPage::from(view::Entity::Info(
                    weak_ref,
                )))
                .build(),
        );
    }
}
