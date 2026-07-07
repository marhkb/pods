use std::cell::Cell;
use std::cell::OnceCell;
use std::cell::RefCell;
use std::marker::PhantomData;

use futures::future;
use glib::Properties;
use glib::prelude::*;
use glib::subclass::prelude::*;
use gtk::glib;

use crate::model;

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::Action)]
    pub(crate) struct Action {
        pub(super) abort_handle: RefCell<Option<future::AbortHandle>>,

        #[property(get, set, construct_only, nullable)]
        pub(super) action_list: glib::WeakRef<model::ActionList>,

        #[property(get = Self::end_timestamp, set = Self::set_end_timestamp, explicit_notify)]
        pub(super) end_timestamp: OnceCell<i64>,
        #[property(get, set = Self::set_state, default)]
        pub(super) state: Cell<model::ActionState>,
        #[property(get, set, construct_only)]
        pub(super) start_timestamp: OnceCell<i64>,
        #[property(get, set = Self::set_error, nullable, explicit_notify)]
        pub(super) error: RefCell<Option<String>>,
        #[property(get = Self::has_error)]
        _has_error: PhantomData<bool>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Action {
        const ABSTRACT: bool = true;
        const NAME: &'static str = "Action";
        type Type = super::Action;
    }

    impl ObjectImpl for Action {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }
    }

    impl Action {
        fn end_timestamp(&self) -> i64 {
            self.end_timestamp.get().copied().unwrap_or(-1)
        }

        fn set_end_timestamp(&self, value: i64) {
            let obj = &*self.obj();
            if obj.end_timestamp() >= 0 {
                return;
            }

            self.end_timestamp.set(value).unwrap();
            obj.notify_end_timestamp();
        }

        fn set_state(&self, value: model::ActionState) {
            if self.state.get() == value || self.state.get() != model::ActionState::Ongoing {
                return;
            }

            let obj = &*self.obj();

            if value != model::ActionState::Ongoing {
                self.set_end_timestamp(glib::DateTime::now_local().unwrap().to_unix());
            }

            self.state.set(value);
            obj.notify_state();
        }

        fn set_error(&self, error: Option<String>) {
            let obj = &*self.obj();
            if obj.error() == error {
                return;
            }

            self.error.replace(error);
            obj.notify_error();
            obj.notify_has_error();
        }

        fn has_error(&self) -> bool {
            self.obj().error().is_some()
        }
    }
}

glib::wrapper! {
    pub(crate) struct Action(ObjectSubclass<imp::Action>);
}

pub(crate) trait ActionExt: IsA<Action> {
    fn builder<O>(action_list: &model::ActionList) -> glib::object::ObjectBuilder<'static, O>
    where
        O: glib::object::IsClass + IsA<glib::Object> + IsA<Action>,
    {
        glib::Object::builder()
            .property("action-list", action_list)
            .property(
                "start-timestamp",
                glib::DateTime::now_local().unwrap().to_unix(),
            )
    }

    fn action_list(&self) -> Option<model::ActionList> {
        self.upcast_ref::<Action>().action_list()
    }

    fn cancel(&self) {
        let imp = self.upcast_ref::<Action>().imp();

        let Some(handle) = imp.abort_handle.borrow_mut().take() else {
            return;
        };

        handle.abort();

        self.set_state(model::ActionState::Cancelled);
    }

    fn set_failed(&self, error: &str) {
        self.set_state(model::ActionState::Failed);
        self.upcast_ref::<Action>().set_error(Some(error));
    }

    fn state(&self) -> model::ActionState {
        self.upcast_ref::<Action>().state()
    }

    fn set_state(&self, state: model::ActionState) {
        self.upcast_ref::<Action>().set_state(state);
    }

    fn setup_abort_handle(&self) -> future::AbortRegistration {
        let (abort_handle, abort_registration) = future::AbortHandle::new_pair();
        self.upcast_ref::<Action>()
            .imp()
            .abort_handle
            .replace(Some(abort_handle));

        abort_registration
    }

    fn connect_state_notify<C>(&self, callback: C)
    where
        C: Fn(&Action) + 'static,
    {
        self.upcast_ref::<Action>().connect_state_notify(callback);
    }
}

impl<O: IsA<Action>> ActionExt for O {}

unsafe impl<T: ObjectSubclass + ObjectImpl> IsSubclassable<T> for Action {}
