use std::cell::RefCell;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::str::FromStr;

use adw::prelude::*;
use adw::subclass::prelude::*;
use ashpd::WindowIdentifier;
use ashpd::desktop::open_uri::OpenURIProxy;
use gettextrs::gettext;
use glib::Properties;
use glib::clone;
use glib::closure;
use gtk::CompositeTemplate;
use gtk::glib;

use crate::model;
use crate::rt;
use crate::utils;

const ACTION_OPEN: &str = "port-binding-row.open";

mod imp {
    use super::*;

    #[derive(Debug, Default, Properties, CompositeTemplate)]
    #[properties(wrapper_type = super::PortBindingRow)]
    #[template(resource = "/com/github/marhkb/Pods/ui/view/port_binding_row.ui")]
    pub(crate) struct PortBindingRow {
        #[property(get, set, construct_only)]
        pub(super) port_binding: RefCell<Option<model::PortBinding>>,

        #[template_child]
        pub(super) host_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) container_port_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub(super) open_link_button: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PortBindingRow {
        const NAME: &'static str = "PdsPortBindingRow";
        type Type = super::PortBindingRow;
        type ParentType = gtk::ListBoxRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.install_action_async(ACTION_OPEN, None, async |widget, _, _| {
                widget.open().await;
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for PortBindingRow {
        fn properties() -> &'static [glib::ParamSpec] {
            Self::derived_properties()
        }

        fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
            self.derived_set_property(id, value, pspec);
        }

        fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
            self.derived_property(id, pspec)
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = &*self.obj();

            let port_binding_expr = Self::Type::this_expression("port-binding");
            let is_tcp_expr = port_binding_expr
                .chain_property::<model::PortBinding>("protocol")
                .chain_closure::<bool>(closure!(
                    |_: Self::Type, protocol: model::PortMappingProtocol| protocol
                        == model::PortMappingProtocol::Tcp
                ));

            port_binding_expr
                .chain_closure::<String>(closure!(
                    |_: Self::Type, port_binding: model::PortBinding| {
                        SocketAddr::new(
                            to_std_ip_addr(&port_binding.ip_address()),
                            port_binding.host_port() as u16,
                        )
                        .to_string()
                    }
                ))
                .bind(&*self.host_label, "label", Some(obj));

            port_binding_expr
                .chain_closure::<String>(closure!(
                    |_: Self::Type, port_binding: model::PortBinding| format!(
                        "{}/{}",
                        port_binding.target_port(),
                        port_binding.protocol()
                    )
                ))
                .bind(&*self.container_port_label, "label", Some(obj));

            is_tcp_expr.watch(
                Some(obj),
                clone!(
                    #[weak]
                    obj,
                    #[strong]
                    is_tcp_expr,
                    move || {
                        obj.action_set_enabled(
                            ACTION_OPEN,
                            is_tcp_expr
                                .evaluate(Some(&obj))
                                .map(|v| v.get().unwrap())
                                .unwrap_or(false),
                        );
                    }
                ),
            );
        }
    }

    impl WidgetImpl for PortBindingRow {}
    impl ListBoxRowImpl for PortBindingRow {}
}

glib::wrapper! {
    pub(crate) struct PortBindingRow(ObjectSubclass<imp::PortBindingRow>)
        @extends gtk::Widget, gtk::ListBoxRow,
        @implements gtk::Accessible, gtk::Actionable, gtk::Buildable, gtk::ConstraintTarget;
}

impl From<&model::PortBinding> for PortBindingRow {
    fn from(value: &model::PortBinding) -> Self {
        glib::Object::builder()
            .property("port-binding", value)
            .build()
    }
}

impl PortBindingRow {
    pub(crate) async fn open(&self) {
        let Some(port_binding) = self.port_binding() else {
            return;
        };

        if port_binding.protocol() != model::PortMappingProtocol::Tcp {
            return;
        }

        let Some(connection) = port_binding
            .port_binding_list()
            .and_then(|container_port_binding_list| container_port_binding_list.container())
            .and_then(|container| container.container_list())
            .and_then(|container_list| container_list.client())
            .map(|client| client.connection())
        else {
            return;
        };

        let ip_address = port_binding.ip_address();
        let std_ip_address = to_std_ip_addr(&ip_address);

        let host = if std_ip_address.is_unspecified() && connection.is_remote() {
            let url = connection.url();
            let host = url.split_once("://").unwrap().1;

            host.split_once(':')
                .map(|(host, _)| host)
                .unwrap_or(host)
                .to_string()
        } else {
            std_ip_address.to_string()
        };

        let protocol = match port_binding.target_port() {
            443 | 8443 | 9443 => "https",
            _ => "http",
        };

        let url = format!("{protocol}://{host}:{}", port_binding.host_port());
        let Ok(uri) = ashpd::Uri::parse(&url) else {
            utils::show_error_toast(self, &gettext("Invalid URL"), &url);
            return;
        };

        let identifier = WindowIdentifier::from_native(&self.native().unwrap()).await;

        let open_result = rt::Promise::new(async move {
            let proxy = OpenURIProxy::new().await?;
            proxy
                .open_uri(identifier.as_ref(), &uri, Default::default())
                .await
                .map(|_| ())
        })
        .exec()
        .await;

        if let Err(e) = open_result {
            utils::show_error_toast(self, &gettext("Error opening link"), &e.to_string());
        }
    }
}

fn to_std_ip_addr(ip_address: &str) -> IpAddr {
    if ip_address.is_empty() {
        IpAddr::V4(Ipv4Addr::LOCALHOST)
    } else {
        IpAddr::from_str(ip_address).unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST))
    }
}
