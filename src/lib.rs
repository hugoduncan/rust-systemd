// Copyright 2015 Hugo Duncan
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// #![deny(missing_docs)] can not do this yet due to macro issues
#![feature(core, libc)]

//! Rust API for Systemd commands via DBus.
//!
//! See:
//!   http://www.freedesktop.org/wiki/Software/systemd/dbus/
//!   http://dbus.freedesktop.org/doc/dbus-specification.html

extern crate libc;
extern crate "rustc-serialize" as rustc_serialize;
extern crate "dbus-rs" as dbus;

use std::{error,fmt};
use std::slice::SliceConcatExt;
use serialize::{decode};

pub mod serialize;


/// Define an ObjectPath as a String
pub type ObjectPath = String;

/// Errors that can arise on systemd DBus operations.
#[derive(Debug)]
pub enum SystemdError{
    /// An error from the underlying DBus library
    BusError(dbus::Error),
    /// An error from decoding a DBus message
    DecoderError(serialize::DecoderError),
    /// An error from encoding a DBus message
    EncoderError(serialize::EncoderError),
    /// Invalid arg passed to a DBus operation.
    InvalidArg(String),
    /// Catch all error type
    UnspecifiedError(String)
}

impl error::FromError<dbus::Error> for SystemdError {
    fn from_error(err: dbus::Error) -> SystemdError {
        SystemdError::BusError(err)
    }
}

impl error::FromError<serialize::DecoderError> for SystemdError {
    fn from_error(err: serialize::DecoderError) -> SystemdError {
        SystemdError::DecoderError(err)
    }
}

impl error::FromError<serialize::EncoderError> for SystemdError {
    fn from_error(err: serialize::EncoderError) -> SystemdError {
        SystemdError::EncoderError(err)
    }
}

impl error::FromError<()> for SystemdError {
    fn from_error(_: ()) -> SystemdError {
        SystemdError::UnspecifiedError(
            "while converting from () error type".to_string())
    }
}

/// Result type for systemd DBus errors.
pub type SystemdResult<T> = Result<T,SystemdError>;

/// Remote object, on which methods can be called.
#[derive(Debug)]
struct Object {
    service: &'static str,
    path: &'static str,
    interface: &'static str,
}

static DBUS : &'static Object = &Object{
    service:"org.freedesktop",
    path: "/org/freedesktop",
    interface:"org.freedesktop.DBus"};

// static DBUS_PROPERTIES : &'static Object = &Object{
//     service:"org.freedesktop/DBus",
//     path: "/org/freedesktop/DBus",
//     interface:"org.freedesktop.DBus.Properties"};

static SYSTEMD : &'static Object = &Object{
    service:"org.freedesktop.systemd1",
    path: "/org/freedesktop/systemd1",
    interface:"org.freedesktop.systemd1.Manager"};

impl Object {
    fn method(&self, method: &str) -> Option<dbus::Message> {
        let m = dbus::Message::new_method_call(self.service, self.path,
                                               self.interface, method).unwrap();
        Some(m)
    }
}

/// Unit status returned from systemd.
#[derive(RustcDecodable,RustcEncodable,Debug)]
pub struct UnitStatus {
    name: String,
    description: String,
    load_state: String,
    active_state: String,
    sub_state: String,
    followed: String,
    path: ObjectPath,
    job_id: u32,
    job_type: String,
    job_path: ObjectPath
}


/// Systemd Job information
#[derive(RustcDecodable,RustcEncodable,Debug)]
pub struct Job {
    job_id: u32, // The numeric job id
    name: String, // The primary unit name for this job
    job_type: String, // The job type as string
    job_state: String, // The job state as string
    job_path: ObjectPath, // The job object path
    path: ObjectPath, // The unit object path
}

/// Systemd unit file information
#[derive(RustcDecodable,RustcEncodable,Debug)]
pub struct UnitFile {
    name: String,
    state: String
}

/// Systemd unit file change information
#[derive(RustcDecodable,RustcEncodable,Debug)]
pub struct UnitFileChange {
    action: String,
    link: String,
    destination: String
}

/// Systemd unit file changes
#[derive(RustcDecodable,RustcEncodable,Debug)]
pub struct UnitFileChanges {
    carries_install_info: bool,
    changes: Vec<UnitFileChange>
}

/// A Unit property
#[derive(RustcDecodable,RustcEncodable,Debug)]
pub struct UnitProperty {
    name: String,
    value: String
}

/// Unit properties
#[derive(RustcDecodable,RustcEncodable,Debug)]
pub struct UnitAux {
    name: String,
    properties: Vec<UnitProperty>
}

/// Match rules for notifications
#[derive(Debug)]
pub enum Match{
    /// match on type, type='…'
    Type(String),
    /// match on sender, sender='…'
    Sender(String),
    /// match on interface, interface='…'
    Interface(String),
    /// match on member, member='…'
    Member(String),
    /// match on path, path='…'
    Path(String),
    /// match on destination, destination='…'
    Destination(String),
    /// match on arg n, arg0='…'
    Arg(usize, String)
}

impl fmt::Display for Match {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self {
            &Match::Type(ref s) => write!(fmt,"type='{}'",s),
            &Match::Sender(ref s) => write!(fmt,"sender='{}'",s),
            &Match::Interface(ref s) => write!(fmt,"interface='{}'",s),
            &Match::Member(ref s) => write!(fmt,"member='{}'",s),
            &Match::Path(ref s) => write!(fmt,"path='{}'",s),
            &Match::Destination(ref s) => write!(fmt,"destination='{}'",s),
            &Match::Arg(n, ref s) => write!(fmt,"arg{}='{}'",n,s),
        }}
}

/// Return a rule string for a sequence of matches
pub fn rule_string(matches: &[Match]) -> String {
    let s : Vec<String> = matches.iter()
        .map(|m| m.to_string())
        .collect();
    s.connect(",")
}

/// Add some sugar for calling methods on a DBus connection
trait DBusCallable {
    /// Call a dbus method, and return the reply
    fn call(&self, mut method: dbus::Message, args: &[dbus::MessageItem])
            -> Result<Vec<dbus::MessageItem>,SystemdError>;
    /// Like call, but when no reply is expected
    fn send(&self, mut method: dbus::Message, args: &[dbus::MessageItem])
            -> Result<(),SystemdError>;
}

impl DBusCallable for dbus::Connection {
    fn call(&self, mut method: dbus::Message, args: &[dbus::MessageItem])
            -> Result<Vec<dbus::MessageItem>,SystemdError>
    {
        if !args.is_empty() {
            method.append_items(args);
        }
        let mut r = try!(self.send_with_reply_and_block(method, 2000));
        Ok(r.get_items())
    }

    fn send(&self, mut method: dbus::Message, args: &[dbus::MessageItem])
            -> Result<(),SystemdError>
    {
        if !args.is_empty() {
            method.append_items(args);
        }
        Ok(try!(self.send(method)))
    }
}

/// Macro to create a wrapper method to invoke a Systemd DBus API call.
macro_rules! systemd_dbus {
    // A match with return type
    ($bus:ident, $m:expr, $n:ident ( $($a:ident : $at:ty),* ) -> $t:ty ) => (
        impl <'a> Connection<'a> {
            pub fn $n (&self, $($a: $at),* ) -> Result<$t, SystemdError> {
                let m = self.object.method($m).unwrap();
                let res =
                    try!(self.$bus.call(m,
                                        &[$(try!(serialize::encode($a))),*]));
                let v=decode::<$t>(res);
                match &v {
                    &Ok(_) => (),
                    &Err(ref x) => println!("Error {:?}", x)
                }
                assert!(v.is_ok(), "decode failed");
                Ok(try!(v))
            }
        });
    // A match without return type
    ($bus:ident, $m:expr, $n:ident ( $($a:ident : $at:ty),* ) ) => (
        impl <'a> Connection<'a> {
            pub fn $n (&self, $($a: $at),* ) -> Result<(), SystemdError> {
                let m = self.object.method($m).unwrap();
                try!(self.$bus.call(m, &[$(try!(serialize::encode($a))),*]));
                Ok(())
            }
        })
}


/// Macro to create a wrapper method to invoke a Systemd DBus API call.
/// This doesn't work due to issue
macro_rules! systemd_dbus1 {
    // A match with return type
    ($bus:ident, $m:expr, $n:ident ( $($a:ident : $at:ty),* ) -> $t:ty ) => (
        fn $n (&self, $($a: $at),* ) -> Result<$t, SystemdError> {
                let m = self.object.method($m).unwrap();
                let res =
                    try!(self.$bus.call(m,
                                        &[$(try!(serialize::encode($a))),*]));
                let v=decode::<$t>(res);
                match &v {
                    &Ok(_) => (),
                    &Err(ref x) => println!("Error {:?}", x)
                }
                assert!(v.is_ok(), "decode failed");
                Ok(try!(v))
        });
    // A match without return type
    ($bus:ident, $m:expr, $n:ident ( $($a:ident : $at:ty),* ) ) => (
            fn $n (&self, $($a: $at),* ) -> Result<(), SystemdError> {
                let m = self.object.method($m).unwrap();
                try!(self.$bus.call(m, &[$(try!(serialize::encode($a))),*]));
                Ok(())
        })
}

/// Main type representing a connection to systemd via Dbus.
///
/// ```
/// use system-dbus::Connection;
///
/// let conn=Connection::new();
/// println!("{:?}", conn.list_units());
/// ```
#[derive(Debug)]
pub struct Connection<'a> {
    bus: dbus::Connection,
    signal_bus: dbus::Connection,
    object: &'a Object
}

impl<'a> Connection<'a> {
    /// Create a new connection to systemd
    pub fn new() -> Result<Connection<'a>,SystemdError> {
        Ok(Connection{
            bus: try!(dbus::Connection::get_private(dbus::BusType::System)),
            signal_bus: try!(
                dbus::Connection::get_private(dbus::BusType::System)),
            object: SYSTEMD
        })
    }

    /// Add a match rule for signals
    pub fn add_match(&self, rule_string: &str) -> Result<(),SystemdError> {
        let mut method=DBUS.method("AddMatch").unwrap();
        method.append_items(&[dbus::MessageItem::Str(rule_string.to_string())]);
        Ok(try!(self.signal_bus.send(method)))
    }

    /// Start subscription to systemd signals
    pub fn systemd_signals(&self) -> Result<(),SystemdError> {
        try!(self.add_match(
            rule_string(
                &[Match::Type("Signal".to_string()),
                  Match::Interface(
                      "org.freedesktop.systemd1.Manager".to_string()),
                  Match::Member("UnitNew".to_string())]).as_slice()));
        try!(self.add_match(
            rule_string(
                &[Match::Type("Signal".to_string()),
                  Match::Interface(
                      "org.freedesktop.DBus.Properties".to_string()),
                  Match::Member("PropertiesChanged".to_string())]).as_slice()));
        try!(self.subscribe());
        Ok(())
    }
}

systemd_dbus!(bus, "GetUnit",get_unit(name: &str) -> ObjectPath);
systemd_dbus!(bus, "GetUnitByPID",get_unit_by_pid(pid: u32) -> Vec<Job>);
systemd_dbus!(bus, "LoadUnit",load_unit(name: String) -> ObjectPath);
systemd_dbus!(bus, "StartUnit",
              start_unit(name: String, mode: String) -> ObjectPath);
systemd_dbus!(bus, "StartUnitRecplace",
              start_unit_replace(old_unit: String,
                                 new_unit: String,
                                 mode: String) -> ObjectPath);
systemd_dbus!(bus, "StopUnit",
              stop_unit(name: String, mode: String) -> ObjectPath);
systemd_dbus!(bus, "ReloadUnit",
              reload_unit(name: String, mode: String) -> ObjectPath);
systemd_dbus!(bus, "RestartUnit",
              restart_unit(name: String, mode: String) -> ObjectPath);
systemd_dbus!(bus, "TryRestartUnit",
              try_restart_unit(name: String, mode: String) -> ObjectPath);
systemd_dbus!(bus, "ReloadOrRestartUnit",
              reload_or_restart_unit(name: String, mode: String) -> ObjectPath);
systemd_dbus!(bus, "ReloadOrTryRestartUnit",
              reload_or_try_restart_unit(name: String,
                                         mode: String) -> ObjectPath);
systemd_dbus!(bus, "KillUnit",
              kill_unit(name: String, who: String, signal: u32) -> ObjectPath);
systemd_dbus!(bus, "ResetFailedUnit",
              reset_failed_unit(name: String) -> ObjectPath);

systemd_dbus!(bus, "GetJob",get_job(id: u32) -> ObjectPath);
systemd_dbus!(bus, "CancelJob", cancel_job(id: u32));
systemd_dbus!(bus, "ClearJobs", clear_jobs());
systemd_dbus!(bus, "ResetFailed", reset_failed());

systemd_dbus!(bus, "ListUnits",list_units() -> Vec<UnitStatus>);
systemd_dbus!(bus, "ListJobs",list_jobs() -> Vec<Job>);

systemd_dbus!(signal_bus, "Subscribe", subscribe());
systemd_dbus!(signal_bus, "Unsubscribe", unsubscribe());

systemd_dbus!(bus, "CreateSnapshot",
              create_snapshot(name: String, cleanup: bool) -> ObjectPath);
systemd_dbus!(bus, "RemoveSnapshot", remove_snapshot(name: String));

systemd_dbus!(bus, "Reload", reload());
systemd_dbus!(bus, "Reexecute", reexecute());
systemd_dbus!(bus, "Reboot", reboot());
systemd_dbus!(bus, "PowerOff", power_off());
systemd_dbus!(bus, "Halt", halt());
systemd_dbus!(bus, "KExec", k_exec());
systemd_dbus!(bus, "SwitchRoot", switch_root(new_root: String, init: String));

systemd_dbus!(bus, "SetEnvironment", set_environment(names: String));
systemd_dbus!(bus, "UnsetEnvironment", unset_environment(names: String));
systemd_dbus!(bus, "UnsetAndSetEnvironment",
              unset_and_set_environment(unset: String, set: String));

systemd_dbus!(bus, "ListUnitFiles",list_unit_files() -> Vec<UnitFile>);
systemd_dbus!(bus, "GetUnitFileState",get_unit_file_state(file: String) -> String);
systemd_dbus!(bus, "EnableUnitFiles",
              enable_unit_files(files: String, runtime: bool, force: bool)
                                -> UnitFileChanges);
systemd_dbus!(bus, "DisableUnitFiles",
              disable_unit_files(files: String, runtime: bool)
                                 -> Vec<UnitFileChange>);
systemd_dbus!(bus, "ReenableUnitFiles",
              reenable_unit_files(files: String, runtime: bool, force: bool)
                                  -> UnitFileChanges);
systemd_dbus!(bus, "LinkUnitFiles",
              link_unit_files(files: String, runtime: bool, force: bool)
                              -> Vec<UnitFileChange>);
systemd_dbus!(bus, "PresetUnitFiles",
              preset_unit_files(files: String, runtime: bool, force: bool)
                                -> UnitFileChanges);
systemd_dbus!(bus, "MaskUnitFiles",
              mask_unit_files(files: String, runtime: bool, force: bool)
                              -> Vec<UnitFileChange>);
systemd_dbus!(bus, "UnmaskUnitFiles",
              unmask_unit_files(files: String, runtime: bool)
                                -> Vec<UnitFileChange>);
systemd_dbus!(bus, "SetDefaultTarget",
              set_default_target(files: String) -> Vec<UnitFileChange>);
systemd_dbus!(bus, "GetDefaultTarget", get_default_target() -> String);
systemd_dbus!(bus, "SetUnitProperties",
              set_unit_properties(name: String,
                                  runtime: bool,
                                  properties: Vec<UnitProperty>));
systemd_dbus!(bus, "SetTransientUnit",
              set_transient_unit(name: String,
                                 mode: String,
                                 properties: Vec<UnitProperty>,
                                 aux: Vec<UnitAux>) -> ObjectPath);


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_test() {
        Connection::new().as_ref().unwrap();
    }

    #[test]
    fn list_units_test() {
        match Connection::new().unwrap().list_units() {
            Ok(v) => println!("list_units succeeded {:?}", v),
            Err(e) => {
                println!("list_units failed {:?}", e);
                assert!(false, "list_units failed");
            }}
    }

    #[test]
    fn list_jobs_test() {
        match Connection::new().unwrap().list_jobs() {
            Ok(v) => println!("list_jobs succeeded {:?}", v),
            Err(e) => {
                println!("list_jobs failed {:?}", e);
                assert!(false, "list_jobs failed");
            }}
    }

    #[test]
    fn get_unit_test() {
        match Connection::new().unwrap().get_unit("syslog.socket") {
            Ok(v) => println!("get_unit succeeded {:?}", v),
            Err(e) => {
                println!("get_unit failed {:?}", e);
                assert!(false, "get_unit failed");
            }}
    }

    #[test]
    fn get_unit_by_pid_test() {
        match Connection::new().unwrap().get_unit_by_pid(1) {
            Ok(v) => println!("get_unit succeeded {:?}", v),
            Err(e) => {
                println!("get_unit failed {:?}", e);
                assert!(false, "get_unit failed");
            }}
    }

    #[test]
    fn clear_jobs_test() {
        match Connection::new().unwrap().clear_jobs() {
            Ok(v) => println!("clear_jobs succeeded {:?}", v),
            Err(e) => {
                println!("get_unit failed {:?}", e);
                assert!(false, "get_unit failed");
            }}
    }


    #[test]
    fn rule_string_test() {
        assert_eq!("type='a',interface='i'",
                   rule_string(&[Match::Type("a".to_string()),
                                 Match::Interface("i".to_string())]))
    }
}
