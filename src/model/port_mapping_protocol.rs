use std::fmt;

use gtk::glib;

use crate::engine;

#[derive(Debug, Default, Copy, Clone, PartialEq, Eq, glib::Enum)]
#[enum_type(name = "PortMappingProtocol")]
pub(crate) enum PortMappingProtocol {
    #[default]
    Tcp,
    Udp,
    Sctp,
}

impl From<engine::dto::PortMappingProtocol> for PortMappingProtocol {
    fn from(value: engine::dto::PortMappingProtocol) -> Self {
        match value {
            engine::dto::PortMappingProtocol::Tcp => Self::Tcp,
            engine::dto::PortMappingProtocol::Udp => Self::Udp,
            engine::dto::PortMappingProtocol::Sctp => Self::Sctp,
        }
    }
}

impl From<PortMappingProtocol> for engine::dto::PortMappingProtocol {
    fn from(value: PortMappingProtocol) -> Self {
        match value {
            PortMappingProtocol::Tcp => Self::Tcp,
            PortMappingProtocol::Udp => Self::Udp,
            PortMappingProtocol::Sctp => Self::Sctp,
        }
    }
}

impl fmt::Display for PortMappingProtocol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Tcp => "tcp",
                Self::Udp => "udp",
                Self::Sctp => "sctp",
            }
        )
    }
}
