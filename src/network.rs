use dbus::Connection;
use dbus::BusType;
use dbus::arg::RefArg;
use dbus::Error;

use network_manager::OrgFreedesktopDBusProperties;

use std::rc::Rc;
use tokio_core::reactor::Core;
use futures::{Stream};
use dbus_tokio::AConnection;

use std::sync::mpsc::{Sender};

// NMState:
// https://developer.gnome.org/NetworkManager/stable/nm-dbus-types.html#NMState
const NM_STATE_UNKNOWN: i64 = 0;
const NM_STATE_STATE_ASLEEP: i64 = 10;
const NM_STATE_DISCONNECTED: i64 = 20;
const NM_STATE_DISCONNECTING: i64 = 30;
const NM_STATE_CONNECTING: i64 = 40;
const NM_STATE_CONNECTED_LOCAL: i64 = 50;
const NM_STATE_CONNECTED_SITE: i64 = 60;
const NM_STATE_CONNECTED_GLOBAL: i64 = 70;

pub struct NetworkMonitor {
    conn: Rc<Connection>
}


impl NetworkMonitor {
    pub fn new() -> Result<NetworkMonitor, Error> {
        match Connection::get_private(BusType::System) {
            Ok(c) => {
                Ok(NetworkMonitor {
                    conn: Rc::new(c)
                })
            },
            Err(err) => {
                Err(err)
            }
        }
    }

    fn get_state(&self) -> Option<i64> {
        info!("Getting connection state");
        let c_path = self.conn.with_path(
            "org.freedesktop.NetworkManager", 
            "/org/freedesktop/NetworkManager", 
            5000);
        if let Ok(variant) = c_path.get("org.freedesktop.NetworkManager",
        "State") {
            return variant.as_i64();
        } else {
            return None
        }
    }

    fn get_primary_connection_type(&self) -> Option<String> {
        info!("Getting primary connection type");
        let c_path = self.conn.with_path(
            "org.freedesktop.NetworkManager",
            "/org/freedesktop/NetworkManager",
            5000);
        if let Ok(variant) = c_path.get("org.freedesktop.NetworkManager", 
            "PrimaryConnectionType") {
            if let Some(variant_str) = variant.as_str() {
                return Some(String::from(variant_str));
            } else {
                return None
            }
        } else {
            error!("Could not get network connection type");
            return None
        }
    }

    fn update_wifi_status(&self, state: i64) -> String {
        match state {
            NM_STATE_CONNECTED_GLOBAL => {
                return String::from("network-wireless-signal-excellent-symbolic");
            },
            NM_STATE_CONNECTED_SITE => {
                return String::from("network-wireless-no-route-symbolic");
            },
            NM_STATE_CONNECTED_LOCAL => {
                return String::from("network-wireless-disconnected-symbolic");
            },
            NM_STATE_CONNECTING => {
                return String::from("network-wireless-acquiring-symbolic");
            },
            NM_STATE_DISCONNECTING | NM_STATE_DISCONNECTED => {
                return String::from("network-wireless-disconnected-symbolic");
            },
            NM_STATE_STATE_ASLEEP | NM_STATE_UNKNOWN | _ => {
                return String::from("network-wireless-disconnected-symbolic");
            }
        }
    }

    fn update_wired_status(&self, state: i64) -> String {
        match state {
            NM_STATE_CONNECTED_GLOBAL => {
                return String::from("network-transmit-receive-symbolic");
            },
            NM_STATE_CONNECTED_SITE => {
                return String::from("network-wired-no-route-symbolic");
            },
            NM_STATE_CONNECTED_LOCAL => {
                return String::from("network-wired-disconnected-symbolic");
            },
            NM_STATE_CONNECTING => {
                return String::from("network-wired-acquiring-symbolic");
            },
            NM_STATE_DISCONNECTING | NM_STATE_DISCONNECTED => {
                return String::from("network-wired-disconnected-symbolic");
            },
            NM_STATE_STATE_ASLEEP | NM_STATE_UNKNOWN | _ => {
                return String::from("network-wired-offline-symbolic");
            }
        }
    }

    fn update_status(&self) -> String {        
        if let Some(state) = self.get_state() {
            if let Some(connection_type) = self.get_primary_connection_type() {
                info!("Connection type: {:?}", connection_type);
                match connection_type.as_ref() {
                    "802-11-wireless" => return self.update_wifi_status(state),
                    "802-3-ethernet" => return self.update_wired_status(state),
                    _  => return String::from("network-wired-no-route-symbolic")
                }
            }
        }
        return String::from("network-wired-acquiring-symbolic")
    }

    pub fn run(&self, out_chan: Sender<String>) {
        if let Err(err) = self.conn.add_match(
            "type=signal,sender=org.freedesktop.NetworkManager,path=/org/freedesktop/NetworkManager,member=StateChanged")
        {
                error!("Network monitor could not add dbus match: {:?}", err);
                return
        }
        if let Err(err) = self.conn.add_match(
            "type=signal,sender=org.freedesktop.NetworkManager,path=/org/freedesktop/NetworkManager,member=PropertiesChanged")
        {
                error!("Network monitor could not add dbus match: {:?}", err);
                return
        }
        let mut core = Core::new().unwrap();
        let aconn = AConnection::new(self.conn.clone(), core.handle()).unwrap();
        let messages = aconn.messages().unwrap();
        let signals = messages.for_each(|m| {
            info!("Incoming signal: {:?}", m);
            let headers = m.headers();
            if let Some(member) = headers.3 {
                if member == "StateChanged" || member == "PropertiesChanged" {
                    let icon = self.update_status();
                    match out_chan.send(icon.clone()) {
                        Ok(_) => info!("Sent network icon: {}", icon),
                        Err(err) => error!("Could not send network icon: {} - {}", icon, err)
                    }
                }
            }
            Ok(())
        });
        core.run(signals).unwrap();   
    }
}

pub fn monitor_network<'a>(out_chan: Sender<String>) {
    match NetworkMonitor::new() {
        Ok(monitor) => {
            info!("Starting network monitor");
            let icon = monitor.update_status();
            match out_chan.send(icon.clone()) {
                Ok(_) => info!("Sent initial network icon: {}", icon.clone()),
                Err(err) => error!("Could not send network initial icon: {} - {}", icon, err)
            }
            monitor.run(out_chan);
        },
        Err(err) => {
            error!("Could not start network monitor: {:?}", err);
        }
    }
}