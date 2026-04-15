use crate::engine;

pub(crate) type Event = engine::Response<bollard::plugin::EventMessage, podman_api::models::Event>;

impl Event {
    pub(crate) fn type_(&self) -> EventType {
        match self {
            engine::Response::Docker(event) => event.into(),
            engine::Response::Podman(event) => event.into(),
        }
    }
}

#[derive(Clone, Copy, Default)]
pub(crate) enum EventType {
    Container,
    Image,
    Pod,
    Volume,
    #[default]
    Other,
}

impl From<&bollard::plugin::EventMessage> for EventType {
    fn from(value: &bollard::plugin::EventMessage) -> Self {
        value
            .typ
            .as_ref()
            .map(|event_type| match event_type {
                bollard::plugin::EventMessageTypeEnum::CONTAINER => Self::Container,
                bollard::plugin::EventMessageTypeEnum::IMAGE => Self::Image,
                bollard::plugin::EventMessageTypeEnum::VOLUME => Self::Volume,
                _ => Self::default(),
            })
            .unwrap_or_default()
    }
}

impl From<&podman_api::models::Event> for EventType {
    fn from(value: &podman_api::models::Event) -> Self {
        match value.typ.as_str() {
            "container" => Self::Container,
            "image" => Self::Image,
            "pod" => Self::Pod,
            "volume" => Self::Volume,
            _ => Self::default(),
        }
    }
}
