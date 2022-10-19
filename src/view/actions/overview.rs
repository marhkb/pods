use glib::subclass::InitializingObject;
use gtk::glib::closure;
use gtk::glib::{self};
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;
use once_cell::sync::Lazy;

use crate::model;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/github/marhkb/Pods/ui/actions/overview.ui")]
    pub(crate) struct Overview {
        pub(super) action_list: glib::WeakRef<model::ActionList>,
        #[template_child]
        pub(super) stack: TemplateChild<gtk::Stack>,
        #[template_child]
        pub(super) action_list_view: TemplateChild<gtk::ListView>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Overview {
        const NAME: &'static str = "PdsActionsOverview";
        type Type = super::Overview;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            Self::bind_template(klass);
            Self::bind_template_callbacks(klass);
            klass.set_css_name("actionsoverview");
        }

        fn instance_init(obj: &InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl Overview {
        #[template_callback]
        fn activated(&self, pos: u32) {
            let action = self
                .action_list_view
                .model()
                .unwrap()
                .item(pos)
                .unwrap()
                .downcast::<model::Action>()
                .unwrap();

            let instance = self.instance();

            utils::root(&*instance)
                .leaflet_overlay()
                .show_details(&view::ActionPage::from(&action));

            instance
                .ancestor(gtk::PopoverMenu::static_type())
                .unwrap()
                .downcast::<gtk::PopoverMenu>()
                .unwrap()
                .popdown();
        }
    }

    impl ObjectImpl for Overview {
        fn properties() -> &'static [glib::ParamSpec] {
            static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
                vec![
                    glib::ParamSpecObject::builder::<model::ActionList>("action-list")
                        .flags(glib::ParamFlags::READWRITE | glib::ParamFlags::EXPLICIT_NOTIFY)
                        .build(),
                ]
            });

            PROPERTIES.as_ref()
        }

        fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            match pspec.name() {
                "action-list" => self.instance().set_action_list(value.get().unwrap()),
                _ => unimplemented!(),
            }
        }

        fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            match pspec.name() {
                "action-list" => self.instance().action_list().to_value(),
                _ => unimplemented!(),
            }
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.instance();

            Self::Type::this_expression("action-list")
                .chain_property::<model::ActionList>("len")
                .chain_closure::<String>(closure!(|_: Self::Type, len: u32| {
                    if len > 0 {
                        "actions"
                    } else {
                        "empty"
                    }
                }))
                .bind(&*self.stack, "visible-child-name", Some(obj));
        }

        fn dispose(&self) {
            self.stack.unparent();
        }
    }

    impl WidgetImpl for Overview {}
}

glib::wrapper! {
    pub(crate) struct Overview(ObjectSubclass<imp::Overview>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for Overview {
    fn default() -> Self {
        glib::Object::new::<Self>(&[])
    }
}

impl Overview {
    pub(crate) fn action_list(&self) -> Option<model::ActionList> {
        self.imp().action_list.upgrade()
    }

    pub(crate) fn set_action_list(&self, value: Option<&model::ActionList>) {
        if self.action_list().as_ref() == value {
            return;
        }

        let imp = self.imp();

        if let Some(action_list) = value {
            let sorter = gtk::NumericSorter::builder()
                .expression(model::Action::this_expression("start-timestamp"))
                .sort_order(gtk::SortType::Descending)
                .build();
            let model = gtk::SortListModel::new(Some(action_list), Some(&sorter));

            imp.action_list_view
                .set_model(Some(&gtk::NoSelection::new(Some(&model))));
        }

        imp.action_list.set(value);
        self.notify("action-list");
    }
}
