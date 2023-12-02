use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Event {
    pub(crate) typ: String,
    pub(crate) action: String,
    pub(crate) actor: Actor,
}

impl From<docker_api::models::EventMessage> for Event {
    fn from(value: docker_api::models::EventMessage) -> Self {
        Self {
            typ: value.type_.unwrap(),
            action: value.action.unwrap(),
            actor: value.actor.unwrap().into(),
        }
    }
}

impl From<podman_api::models::Event> for Event {
    fn from(value: podman_api::models::Event) -> Self {
        Self {
            typ: value.typ,
            action: value.action,
            actor: value.actor.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Actor {
    pub(crate) id: String,
    pub(crate) attributes: HashMap<String, String>,
}

impl From<docker_api::models::EventActor> for Actor {
    fn from(value: docker_api::models::EventActor) -> Self {
        Self {
            id: value.id.unwrap(),
            attributes: value.attributes.unwrap(),
        }
    }
}

impl From<podman_api::models::Actor> for Actor {
    fn from(value: podman_api::models::Actor) -> Self {
        Self {
            id: value.id,
            attributes: value.attributes,
        }
    }
}
