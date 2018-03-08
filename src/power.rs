use dbus::Connection;
use dbus::BusType;
use dbus::arg::RefArg;
use std::rc::Rc;
use tokio_core::reactor::Core;
use futures::{Stream};
use dbus_tokio::AConnection;
use upower::OrgFreedesktopDBusProperties;
use upower::OrgFreedesktopUPowerDevice;
use std::sync::mpsc::{Sender};

// State constants
// https://upower.freedesktop.org/docs/Device.html#Device:State
const UNKNOWN: i64 = 0;
const CHARGING: i64 = 1;
const DISCHARGING: i64 = 2;
const EMPTY: i64 = 3;
const FULLY_CHARGED: i64 = 4;
const PENDING_CHARGE: i64 = 5;
const PENDING_DISCHARGE: i64 = 6;

pub struct PowerMonitor {
    conn: Rc<Connection>
}

impl PowerMonitor {
    pub fn new() -> PowerMonitor {
        let c = Rc::new(Connection::get_private(BusType::System).unwrap());
        PowerMonitor {
            conn: c
        }
    }
    
    fn get_state(&self) -> Option<i64> {
        let c_path = self.conn.with_path(
            "org.freedesktop.UPower", 
            "/org/freedesktop/UPower/devices/DisplayDevice", 
            5000);
        if let Ok(variant) = c_path.get("org.freedesktop.UPower.Device", "State") {
            return variant.as_i64();
        } else {
            return None
        }
    }

    fn get_percentage(&self) -> i64 {
        let c_path = self.conn.with_path(
        "org.freedesktop.UPower", "/org/freedesktop/UPower/devices/DisplayDevice", 5000);
        let percentage = c_path.get_percentage().unwrap();
        return percentage.round() as i64
    }

    pub fn update_status(&self) -> String {
        if let Some(state) = self.get_state() {
            let percentage = self.get_percentage();
            match state {
                FULLY_CHARGED => {
                    return String::from("battery-full-charged-symbolic");
                },
                CHARGING => { 
                    match percentage {
                        98 ... 100 => return String::from("battery-full-charging-symbolic"),
                        40 ... 97 => return String::from("battery-good-charging-symbolic"),
                        21 ... 39 => return String::from("battery-medium-charging-symbolic"),
                        5 ... 20 => return String::from("battery-low-charging-symbolic"),
                        0 ... 4 => return String::from("battery-caution-charging-symbolic"),
                        _ => return String::from("battery-symbolic")
                    }
                },
                DISCHARGING => {
                    match percentage {
                        98 ... 100 => return String::from("battery-full-charged-symbolic"),
                        40 ... 97 => return String::from("battery-good-symbolic"),
                        21 ... 39 => return String::from("battery-mediun-symbolic"),
                        5 ... 20 => return String::from("battery-low-symbolic"),
                        0 ... 4 => return String::from("battery-caution-symbolic"),
                        _ => return String::from("battery-symbolic")
                    }
                },
                EMPTY => {
                    return String::from("battery-empty-symbolic")
                }
                UNKNOWN | PENDING_DISCHARGE | PENDING_CHARGE | _ => { 
                    return String::from("battery-missing-symbolic")
                }
            }
        } else {
            return String::from("battery-symbolic")
        }
        
    }

    pub fn run(&self, out_chan: Sender<String>) {
        self.conn.add_match(
            "type=signal,sender=org.freedesktop.UPower,path=/org/freedesktop/UPower/devices/DisplayDevice,member=PropertiesChanged").unwrap();
        let mut core = Core::new().unwrap();
        let aconn = AConnection::new(self.conn.clone(), core.handle()).unwrap();
        let messages = aconn.messages().unwrap();
        let signals = messages.for_each(|m| {
            let headers = m.headers();
            let member = headers.3.unwrap();
            if member == "PropertiesChanged" {
                let icon = self.update_status();
                match out_chan.send(icon.clone()) {
                    Ok(_) => info!("Sent icon: {}", icon),
                    Err(err) => error!("Could not send icon: {} - {}", icon, err)
                }
            }
            Ok(())
        });
        core.run(signals).unwrap();   
    }
}

pub fn monitor_power<'a>(out_chan: Sender<String>) {
    let monitor = PowerMonitor::new();
    info!("Starting power monitor");
    let icon = monitor.update_status();
    match out_chan.send(icon.clone()) {
        Ok(_) => info!("Sent initial icon: {}", icon.clone()),
        Err(err) => error!("Could not send initial icon: {} - {}", icon, err)
    }
    monitor.run(out_chan);
}