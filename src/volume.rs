use std::ffi::CStr;
use std::mem::transmute;
use std::mem::forget;
use std::os::raw::{c_void};
use std::ptr;
use std::error;
use std::fmt;
use std::time::Duration;
use std::thread::sleep;

use pulse::context::Context;
use pulse::context::ContextInternal;
use pulse::context::introspect::ServerInfoInternal;
use pulse::context::introspect::SinkInfoInternal;
use pulse::context::flags;
use pulse::context::State::Failed;
use pulse::context::State::Ready;
use pulse::context::State::Terminated;
use pulse::context::subscribe::subscription_masks;
use pulse::context::subscribe::EventType;
use pulse::context::subscribe::Facility;
use pulse::context::subscribe::get_facility;
use pulse::mainloop::standard::Mainloop;
use pulse::mainloop::standard::InterateResult;
use pulse::volume::CVolume;

use VOLUME_TX;

pub struct VolumeMonitor {
    context: Context,
    mainloop: Mainloop 
}

#[derive(Debug, Clone)]
pub struct VolumeMonitorError{
    details: String
}

impl VolumeMonitorError {
    fn new(msg: &str) -> VolumeMonitorError {
        VolumeMonitorError{details: msg.to_string()}
    }
}

impl fmt::Display for VolumeMonitorError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl error::Error for VolumeMonitorError {
    fn description(&self) -> &str {
        &self.details
    }
}

impl VolumeMonitor {
    pub fn new() -> Result<VolumeMonitor, VolumeMonitorError> {
        let m = Mainloop::new().unwrap();
        let delay = Duration::from_millis(200);
        for i in 0..10 {
            let c = Context::new(m.get_api(), "sgstatusContext").unwrap();
            match c.connect(None, flags::NOAUTOSPAWN, None) {
                Ok(_) => {
                        return Ok(VolumeMonitor {
                            context: c,
                            mainloop: m
                        })
                },
                Err(_) => error!("Could not connect to pulse on attempt: {:?}", i+1)
            }
            sleep(delay);
        }
        return Err(VolumeMonitorError::new("pulse connection failed, gave up"))
    }

    pub fn update_status(&self) {
        let data_ptr: *mut c_void = ptr::null_mut();
        self.context.introspect().get_server_info((server_info_cb, data_ptr));
    }

    pub fn run(&self) {   
        loop {
            match self.mainloop.iterate(false) {
                InterateResult::Quit(_) |
                InterateResult::Err(_) => {
                    error!("Pulse mainloop iterate error");
                    return;
                },
                InterateResult::Success(_) => {},
            }
            match self.context.get_state() {
                Ready => { break; },
                Failed | Terminated => {
                    error!("Pulse context state is failed/terminated");
                    return;
                },
                _ => {},
            }
        }
        let interest = subscription_masks::SINK;
        // I basically throw away these pointers (for now)
        let success_ptr: *mut c_void = ptr::null_mut();
        let subscribe_op = self.context.subscribe(
            interest,
            (success_cb, success_ptr)
        );
        if subscribe_op.is_none() {
            error!("Pulse audio subscribe failed");
            return;
        };
        let data_ptr: *mut c_void = ptr::null_mut();
        self.context.set_subscribe_callback(Some((subscribe_cb, data_ptr)));
        // Pre-populate the icon on startup
        self.update_status();
        loop {    
            match self.mainloop.run() {
                Some(ret_val) => {
                    info!("Stopping pulse mainloop, return value: {:?}", 
                        ret_val)
                },
                None => error!(
                    "Unspecified error while stopping pulse mainloop")
            }
        }
    }

    
}

extern "C"
fn success_cb(
    _: *mut ContextInternal,
    _: i32,
    _: *mut c_void) {}

extern "C"
fn subscribe_cb(
    c: *mut ContextInternal, 
    t: EventType,
    _: u32,
    _: *mut c_void)
{
    if get_facility(t).unwrap() == Facility::Sink {
        let data_ptr: *mut c_void = ptr::null_mut();
        let context = Context::from_raw_weak(c);
        context.introspect().get_server_info((server_info_cb, data_ptr));
    }
    return;
}

#[allow(unused)]
extern "C"
fn server_info_cb(c: *mut ContextInternal, 
    info: *const ServerInfoInternal, 
    _: *mut c_void) {
        assert!(!info.is_null());
        if !info.is_null() {
            let mut sink = "";
            unsafe {
                sink = CStr::from_ptr((*info).default_sink_name).to_str().unwrap();
            }
            let data_ptr: *mut c_void = ptr::null_mut();
            let context = Context::from_raw_weak(c);
            context.introspect().get_sink_info_by_name(
                sink,
                (sink_info_cb, data_ptr)
            );
            forget(context);
        }
    return;
}

extern "C"
fn sink_info_cb(_: *mut ContextInternal,
    info: *const SinkInfoInternal,
    _: i32,
    _: *mut c_void) {
        if !info.is_null() {
            info!("Gathering volume data");
            let muted: i32 = unsafe { transmute((*info).mute) };
            if muted == 0 {
                let volume: CVolume = unsafe { transmute((*info).volume)};
                if let Some(level) = volume.print() {
                    let level_icon = get_level_icon(level);
                    send_icon(level_icon);
                } 
            } else if muted == 1 {
                send_icon(String::from("audio-volume-muted-symbolic"));  
            }
            
        }
        return;
}

pub fn get_level_icon(level: String) -> String {
    let mut values = level.split_whitespace();
    if let Some(value) = values.nth(1) {
        let trimmed = value.trim_matches('%');
        if let Ok(percentage) = trimmed.parse::<i32>() {
            match percentage {
                p if p > 75 => return String::from("audio-volume-high-symbolic"),
                26 ... 75 => return String::from("audio-volume-medium-symbolic"),
                1 ... 25 => return String::from("audio-volume-low-symbolic"),
                0 | _ => return String::from("audio-volume-muted-symbolic")
            };
        } else {
            error!("Could not parse volume level into a percentage");
        }
    }
    return String::from("audio-volume-muted-symbolic")
}

pub fn send_icon(icon: String) {
    info!("Sent icon: {}", icon);
    unsafe {
        let sender = VOLUME_TX.as_ref().unwrap().lock().unwrap();
        match sender.send(icon.clone()) {
            Ok(_) => info!("Sent icon: {}", icon),
            Err(err) => error!("Could not send icon: {} - {}", icon, err)
        }        
    }
}

pub fn monitor_volume<'a>() {
    match VolumeMonitor::new() {
        Ok(monitor) => {
            info!("Starting volume monitor");
            monitor.run();
        },
        Err(err) => {
            error!("Could not start volume monitor: {:?}", err);
        }
    }
}
