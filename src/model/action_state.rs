use gtk::glib;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "PdsActionState")]
pub(crate) enum ActionState {
    Cancelled,
    Failed,
    Finished,
    #[default]
    Ongoing,
}
