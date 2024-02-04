pub(crate) struct Network {
    pub(crate) id: String,
    pub(crate) default: bool,
    pub(crate) dns: bool,
    pub(crate) driver: Option<String>,
    pub(crate) internal: bool,
    pub(crate) ipv4: bool,
    pub(crate) ipv6: bool,
    pub(crate) name: Option<String>,
}

impl From<bollard::plugin::Network> for Network {
    fn from(value: bollard::plugin::Network) -> Self {
        Self {
            id: value.id.unwrap(),
            default: value
                .name
                .as_ref()
                .map(|name| name != "bridge")
                .unwrap_or(false),
            dns: value
                .name
                .as_ref()
                .map(|name| name != "bridge" && name != "host" && name != "none")
                .unwrap_or(false),
            driver: value.driver,
            internal: value.internal.unwrap_or(false),
            ipv4: value.enable_ipv4.unwrap_or(false),
            ipv6: value.enable_ipv6.unwrap_or(false),
            name: value.name,
        }
    }
}

impl From<podman_api::models::Network> for Network {
    fn from(value: podman_api::models::Network) -> Self {
        Self {
            id: value.id.unwrap(),
            default: value.name.as_ref().unwrap() == "podman",
            dns: value.dns_enabled.unwrap_or(false),
            driver: value.driver,
            internal: value.internal.unwrap_or(true),
            ipv4: value
                .subnets
                .as_ref()
                .map(|subnets| {
                    subnets.iter().any(|s| {
                        s.subnet
                            .as_ref()
                            .map_or(false, |cidr| cidr.to_string().contains('.'))
                    })
                })
                .unwrap_or(true),
            ipv6: value.ipv_6_enabled.unwrap_or(false),
            name: value.name,
        }
    }
}
