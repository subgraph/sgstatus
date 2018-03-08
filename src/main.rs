#[macro_use] extern crate log;
extern crate dbus;
extern crate dbus_tokio;
extern crate tokio_core;
extern crate tokio_timer;
extern crate futures;
extern crate env_logger;

mod network;
mod network_manager;
mod power;
mod upower;
mod sni_item;
mod sni_watcher;
mod status;

use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::thread;

fn main() {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    let (network_tx, network_rx): (Sender<String>, Receiver<String>) = mpsc::channel();

    let (power_tx, power_rx): (Sender<String>, Receiver<String>) = mpsc::channel();

    thread::spawn(move || {
        network::monitor_network(network_tx);
    });

    
    thread::spawn(move || {
        power::monitor_power(power_tx);
    });

    thread::spawn(move || {
        if let Ok(mut network_notifier) = status::StatusNotifier::new() {
            info!("Starting network notifier");
            if let Err(err) = network_notifier.run(network_rx) {
                error!("Could not start network notifier: {:?}", err)
            }
        }    
    });

    thread::spawn(move || {
        if let Ok(mut power_notifier) = status::StatusNotifier::new() {
            info!("Starting power notifier");
            if let Err(err) = power_notifier.run(power_rx) {
                error!("Could not start power notifier: {:?}", err)
            }
        }
        
    });

    loop {
        std::thread::sleep(std::time::Duration::new(1, 0));
    }

}
