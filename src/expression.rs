use glib::closure;
use gtk::glib;
use gtk::glib::value::FromValue;

pub(crate) trait ExpressionExt {
    fn map<U, O>(&self, f: impl Fn(O) -> U + 'static) -> gtk::ClosureExpression
    where
        U: Clone + glib::value::ValueType,
        for<'r> O: FromValue<'r>;
}

impl<T: AsRef<gtk::Expression>> ExpressionExt for T {
    fn map<U, O>(&self, f: impl Fn(O) -> U + 'static) -> gtk::ClosureExpression
    where
        U: Clone + glib::value::ValueType,
        for<'r> O: FromValue<'r>,
    {
        self.as_ref()
            .chain_closure_with_callback(move |values| f(values[1].get::<O>().unwrap()))
    }
}

pub(crate) trait BoolExpressionExt {
    fn not(&self) -> gtk::ClosureExpression;
    fn and(&self, other: impl AsRef<gtk::Expression>) -> gtk::ClosureExpression;
    fn or(&self, other: impl AsRef<gtk::Expression>) -> gtk::ClosureExpression;
}

impl<T: AsRef<gtk::Expression>> BoolExpressionExt for T {
    fn not(&self) -> gtk::ClosureExpression {
        self.as_ref()
            .chain_closure::<bool>(closure!(|_: Option<glib::Object>, value: bool| !value))
    }

    fn and(&self, other: impl AsRef<gtk::Expression>) -> gtk::ClosureExpression {
        gtk::ClosureExpression::new::<bool>(
            [self.as_ref(), other.as_ref()],
            closure!(|_: Option<glib::Object>, value_1: bool, value_2: bool| value_1 && value_2),
        )
    }

    fn or(&self, other: impl AsRef<gtk::Expression>) -> gtk::ClosureExpression {
        gtk::ClosureExpression::new::<bool>(
            [self.as_ref(), other.as_ref()],
            closure!(|_: Option<glib::Object>, value_1: bool, value_2: bool| value_1 || value_2),
        )
    }
}

pub(crate) trait NullableExpressionExt {
    fn is_some<O>(&self) -> gtk::ClosureExpression
    where
        for<'r> Option<O>: FromValue<'r>;

    fn map_or<U, O>(&self, default: U, f: impl Fn(O) -> U + 'static) -> gtk::ClosureExpression
    where
        U: Clone + glib::value::ValueType,
        for<'r> Option<O>: FromValue<'r>;

    fn map_or_else<U, O>(
        &self,
        default: impl Fn() -> U + 'static,
        f: impl Fn(O) -> U + 'static,
    ) -> gtk::ClosureExpression
    where
        U: glib::value::ValueType,
        for<'r> Option<O>: FromValue<'r>;

    fn map_or_default<U, O>(&self, f: impl Fn(O) -> U + 'static) -> gtk::ClosureExpression
    where
        U: Default + glib::value::ValueType,
        for<'r> Option<O>: FromValue<'r>;
}

impl<T: AsRef<gtk::Expression>> NullableExpressionExt for T {
    fn is_some<O>(&self) -> gtk::ClosureExpression
    where
        for<'r> Option<O>: FromValue<'r>,
    {
        self.as_ref()
            .chain_closure_with_callback(|values| values[1].get::<Option<O>>().unwrap().is_some())
    }

    fn map_or<U, O>(&self, default: U, f: impl Fn(O) -> U + 'static) -> gtk::ClosureExpression
    where
        U: Clone + glib::value::ValueType,
        for<'r> Option<O>: FromValue<'r>,
    {
        self.as_ref().chain_closure_with_callback(move |values| {
            values[1]
                .get::<Option<O>>()
                .unwrap()
                .map_or(default.clone(), &f)
        })
    }

    fn map_or_else<U, O>(
        &self,
        default: impl Fn() -> U + 'static,
        f: impl Fn(O) -> U + 'static,
    ) -> gtk::ClosureExpression
    where
        U: glib::value::ValueType,
        for<'r> Option<O>: FromValue<'r>,
    {
        self.as_ref().chain_closure_with_callback(move |values| {
            values[1]
                .get::<Option<O>>()
                .unwrap()
                .map_or_else(&default, &f)
        })
    }

    fn map_or_default<U, O>(&self, f: impl Fn(O) -> U + 'static) -> gtk::ClosureExpression
    where
        U: Default + glib::value::ValueType,
        for<'r> Option<O>: FromValue<'r>,
    {
        self.as_ref().chain_closure_with_callback(move |values| {
            values[1]
                .get::<Option<O>>()
                .unwrap()
                .map(&f)
                .unwrap_or_default()
        })
    }
}
