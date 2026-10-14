use glib::closure;
use gtk::glib;

pub(crate) trait BoolExpression {
    fn not(&self) -> gtk::ClosureExpression;
    fn and(&self, other: impl AsRef<gtk::Expression>) -> gtk::ClosureExpression;
    fn or(&self, other: impl AsRef<gtk::Expression>) -> gtk::ClosureExpression;
}

impl<T: AsRef<gtk::Expression>> BoolExpression for T {
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
