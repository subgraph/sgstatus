use std::rc::Rc;
use std::cell::RefCell;
use std::sync::mpsc::{Receiver};

use dbus;
use dbus::Connection;
use dbus::BusType;
use dbus::tree::DataType;
use dbus::Interface;
use dbus::Path;
use dbus::Member;
use dbus::Message;
use dbus::tree::MethodErr;
use dbus::tree::Factory;

use sni_watcher::OrgFreedesktopStatusNotifierWatcher;
use sni_item::org_kde_status_notifier_item_server;
use sni_item::OrgKdeStatusNotifierItem;


#[derive(Clone, Default, Debug)]
struct Status {
    icon_name: RefCell<String>
}

#[derive(Copy, Clone, Default, Debug)]
struct StatusData;

#[allow(unused)]
impl OrgKdeStatusNotifierItem for Rc<Status> {  
   type Err = MethodErr;
    fn scroll(&self, delta: i32, orientation: &str) -> Result<(), Self::Err> { Ok(()) }
    fn activate(&self,  x: i32, y: i32) -> Result<(), Self::Err> { 
        Ok(()) 
    }
    fn secondary_activate(&self,  x: i32, y: i32) -> Result<(), Self::Err> { Ok(()) }
    fn get_id(&self) -> Result<String, Self::Err> { 
        Ok("".into()) }
    fn get_category(&self) -> Result<String, Self::Err> { Ok("I feel so introspected right now".into()) }
    fn get_status(&self) -> Result<String, Self::Err> { 
        Ok("".into()) }
    fn get_icon_name(&self) -> Result<String, Self::Err> { 
        Ok(self.icon_name.borrow().clone())
    }
    fn get_icon_pix_map(&self) -> Result<Vec<(i32, i32, Vec<u8>)>, Self::Err> {
        let bytes: Vec<u8> = Vec::new();
        let pix_map: Vec<(i32, i32, Vec<u8>)> = vec![(0, 0, bytes)];
        Ok(pix_map)
    }
    fn get_attention_icon_name(&self) -> Result<String, Self::Err> { Ok("I feel so introspected right now".into()) }
    fn get_title(&self) -> Result<String, Self::Err> { 
        Ok("".into()) }
    fn get_icon_theme_path(&self) -> Result<String, Self::Err> {
        Ok("".into()) }
    fn get_menu(&self) -> Result<Path<'static>, Self::Err> { Ok("I feel so introspected right now".into()) }
}

impl DataType for StatusData {
    type Tree = ();
    type ObjectPath = Rc<Status>;
    type Property = ();
    type Interface = ();
    type Method = ();
    type Signal = ();
}

pub struct StatusNotifier {
    conn: Rc<Connection>,
    status: Rc<Status>
}

impl StatusNotifier {
    pub fn new() -> Result<StatusNotifier, dbus::Error> {
        let default_icon_name = RefCell::new(
            String::from("image-loading-symbolic"));
        let s = Rc::new(Status{icon_name: default_icon_name});
        let f = Factory::new_fn::<StatusData>();
        let iface =
            org_kde_status_notifier_item_server(&f, (), |m| m.path.get_data());
        let t = f.tree(()).add(f.object_path(
            "/StatusNotifierItem", s.clone()).add(iface));
        match Connection::get_private(BusType::Session) {
            Ok(c) => {
                match t.set_registered(&c, true) {
                    Ok(_) => {
                        c.add_handler(t);
                        Ok(StatusNotifier {
                            conn: Rc::new(c),
                            status: s
                        })
                    },
                    Err(err) => {
                        error!(
                            "StatusNotifier: could not register object path: {:?}", 
                                err);
                        return Err(err)
                    },
                }
            },
            Err(err) => {
                error!(
                    "StatusNotifier: could not create dbus connection: {:?}", 
                        err);
                return Err(err)
            }
        }    
    }

    fn register_sni(&self) -> Result<(), dbus::Error> {
        let c_path = self.conn.with_path(
            "org.freedesktop.StatusNotifierWatcher", "/StatusNotifierWatcher", 5000);
        let result = c_path.register_status_notifier_item(&self.conn.unique_name());
        return result
    }

    pub fn set_icon_name(&mut self, name: String) {
        *self.status.icon_name.borrow_mut() = name;
    }

    pub fn send_new_icon_signal(&self) {
        let c_path = self.conn.with_path(
            "org.freedesktop.StatusNotifierWatcher", "/StatusNotifierWatcher", 5000);
        if let Ok(signal) = Member::new("NewIcon") {
            if let Ok(iface) = Interface::new("org.kde.StatusNotifierItem") {
                let message = Message::signal(&c_path.path, &iface, &signal);
                match self.conn.send(message) {
                    Ok(_) => info!("Sent new icon signal"),
                    Err(err) => error!("Could not send new icon signal: {:?}", 
                        err)
                }
            }
        }
    }

    pub fn update_icon(&mut self, name: String) {
        info!("Updating icon");
        self.set_icon_name(name);
        self.send_new_icon_signal();
    }

    pub fn run(&mut self, in_chan: Receiver<String>) -> Result<(), dbus::Error> {
        let reply = self.register_sni();
        match reply {
            Ok(_) => {
                info!("StatusNotifierItem registered with watcher - starting StatusNotifier");
                loop {
                    self.conn.incoming(1000).next();
                    match in_chan.recv() {
                        Ok(icon) => self.update_icon(icon),
                        Err(err) => error!("Recv error: {:?}", err)
                    }
                }
            },
            Err(err) => return Err(err)
        }
    }
}

