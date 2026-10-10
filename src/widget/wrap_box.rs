use std::cell::RefCell;

use adw::subclass::prelude::*;
use gio::prelude::*;
use glib::Properties;
use glib::clone;
use gtk::CompositeTemplate;
use gtk::gio;
use gtk::glib;

use crate::utils;

mod imp {
    use std::marker::PhantomData;

    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::WrapBox)]
    #[template(resource = "/com/github/marhkb/Pods/ui/widget/wrap_box.ui")]
    pub(crate) struct WrapBox {
        pub(super) model_binding: RefCell<Option<(gio::ListModel, glib::SignalHandlerId)>>,

        #[property(get = Self::align, set = Self::set_align)]
        _align: PhantomData<f32>,
        #[property(get = Self::child_spacing, set = Self::set_child_spacing)]
        _child_spacing: PhantomData<i32>,
        #[property(get = Self::line_spacing, set = Self::set_line_spacing)]
        _line_spacing: PhantomData<i32>,

        #[template_child]
        pub(super) inner: TemplateChild<adw::WrapBox>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WrapBox {
        const NAME: &'static str = "PdsWrapBox";
        type Type = super::WrapBox;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for WrapBox {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn dispose(&self) {
            utils::unparent_children(&*self.obj());
        }
    }

    impl WidgetImpl for WrapBox {}

    impl WrapBox {
        pub(super) fn align(&self) -> f32 {
            self.inner.align()
        }

        pub(super) fn set_align(&self, value: f32) {
            self.inner.set_align(value);
        }

        pub(super) fn child_spacing(&self) -> i32 {
            self.inner.child_spacing()
        }

        pub(super) fn set_child_spacing(&self, value: i32) {
            self.inner.set_child_spacing(value);
        }

        pub(super) fn line_spacing(&self) -> i32 {
            self.inner.line_spacing()
        }

        pub(super) fn set_line_spacing(&self, value: i32) {
            self.inner.set_line_spacing(value);
        }
    }
}

glib::wrapper! {
    pub(crate) struct WrapBox(ObjectSubclass<imp::WrapBox>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl WrapBox {
    pub(crate) fn bind_model<P: Fn(&glib::Object) -> gtk::Widget + 'static>(
        &self,
        model: Option<&impl IsA<gio::ListModel>>,
        create_widget_func: P,
    ) {
        let imp = self.imp();
        let inner = &*imp.inner;

        if let Some((model, handler_id)) = imp.model_binding.take() {
            model.disconnect(handler_id);
        }
        inner.remove_all();

        let Some(model) = model else {
            return;
        };

        model
            .iter::<glib::Object>()
            .map(Result::unwrap)
            .map(|item| create_widget_func(&item))
            .for_each(|child| inner.append(&child));

        let handler_id = model.connect_items_changed(clone!(
            #[weak]
            inner,
            move |model, position, removed, added| {
                utils::ChildIter::from(&inner)
                    .skip(position as usize)
                    .take(removed as usize)
                    .for_each(|child| inner.remove(&child));

                let sibling = utils::ChildIter::from(&inner).nth(position as usize);

                model
                    .iter::<glib::Object>()
                    .skip(position as usize)
                    .take(added as usize)
                    .map(Result::unwrap)
                    .rev()
                    .map(|item| create_widget_func(&item))
                    .for_each(|child| {
                        inner.insert_child_after(&child, sibling.as_ref());
                    });
            }
        ));

        imp.model_binding
            .replace(Some((model.to_owned().upcast(), handler_id)));
    }
}
