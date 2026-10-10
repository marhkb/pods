use std::cmp::Ordering;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::str::FromStr;

use glib::prelude::*;
use gtk::glib;

use crate::model;

pub(super) fn full_sort_list_model(ports: model::PortBindingList) -> gtk::SortListModel {
    let sorter = gtk::CustomSorter::new(|item_1, item_2| {
        let port_binding_1 = item_1.downcast_ref::<model::PortBinding>().unwrap();
        let port_binding_2 = item_2.downcast_ref::<model::PortBinding>().unwrap();

        match port_binding_1.protocol().cmp(&port_binding_2.protocol()) {
            Ordering::Equal => {
                match IpAddr::from_str(&port_binding_1.ip_address())
                    .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST))
                    .cmp(
                        &IpAddr::from_str(&port_binding_2.ip_address())
                            .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST)),
                    ) {
                    Ordering::Equal => port_binding_1.host_port().cmp(&port_binding_2.host_port()),
                    other => other,
                }
            }
            other => other,
        }
        .into()
    });

    gtk::SortListModel::new(Some(ports), Some(sorter))
}

pub(super) fn host_port_sorter(ports: model::PortBindingList) -> gtk::SortListModel {
    let sorter = gtk::CustomSorter::new(|item_1, item_2| {
        let port_binding_1 = item_1.downcast_ref::<model::PortBinding>().unwrap();
        let port_binding_2 = item_2.downcast_ref::<model::PortBinding>().unwrap();

        match port_binding_1.protocol().cmp(&port_binding_2.protocol()) {
            Ordering::Equal => port_binding_1.host_port().cmp(&port_binding_2.host_port()),
            other => other,
        }
        .into()
    });

    gtk::SortListModel::new(Some(ports), Some(sorter))
}
