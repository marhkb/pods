use gtk::glib;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "ActionState2")]
pub(crate) enum ActionState2 {
    Cancelled,
    Failed,
    Finished,
    #[default]
    Ongoing,
}
