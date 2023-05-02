use glib::subclass::InitializingObject;
use glib::Properties;
use gtk::glib;
use gtk::glib::closure;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::CompositeTemplate;

use crate::model;
use crate::utils;
use crate::view;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::Overview)]
    #[template(resource = "/com/github/marhkb/Pods/ui/actions/overview.ui")]
    pub(crate) struct Overview {
        #[property(get, set = Self::set_action_list, nullable)]
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
            klass.bind_template();
            klass.bind_template_callbacks();
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

            let obj = &*self.obj();

            utils::show_dialog(
                obj.upcast_ref(),
                view::ActionPage::from(&action).upcast_ref(),
            );

            obj.ancestor(gtk::PopoverMenu::static_type())
                .unwrap()
                .downcast::<gtk::PopoverMenu>()
                .unwrap()
                .popdown();
        }
    }

    impl ObjectImpl for Overview {
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
            utils::unparent_children(self.obj().upcast_ref());
        }
    }

    impl WidgetImpl for Overview {}

    impl Overview {
        pub(super) fn set_action_list(&self, value: Option<&model::ActionList>) {
            let obj = self.obj();
            if obj.action_list().as_ref() == value {
                return;
            }

            if let Some(action_list) = value {
                let sorter = gtk::NumericSorter::builder()
                    .expression(model::Action::this_expression("start-timestamp"))
                    .sort_order(gtk::SortType::Descending)
                    .build();
                let model = gtk::SortListModel::new(Some(action_list.to_owned()), Some(sorter));

                self.action_list_view
                    .set_model(Some(&gtk::NoSelection::new(Some(model))));
            }

            self.action_list.set(value);
            obj.notify("action-list");
        }
    }
}

glib::wrapper! {
    pub(crate) struct Overview(ObjectSubclass<imp::Overview>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for Overview {
    fn default() -> Self {
        glib::Object::builder().build()
    }
}
