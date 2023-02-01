use std::{net::SocketAddr, time::Duration};

use clap::Parser;
use color_eyre::Result;
use dbus::{blocking::LocalConnection, message::SignalArgs};
use mac_address::MacAddress;

use cli::Command;
use presence::{generated::SessionManagerPresenceStatusChanged, PresenceStatus};

use crate::power::PowerManager;

mod cli;
mod power;
mod presence;

fn main() -> Result<()> {
    color_eyre::install()?;

    let cmd = Command::parse();
    match cmd {
        Command::On { mac } => power::turn_on(mac),
        Command::Off { addr } => power::turn_off(addr),
        Command::Service { mac, addr } => service(mac, addr),
    }
}

fn service(mac: MacAddress, addr: SocketAddr) -> Result<()> {
    let dbus = LocalConnection::new_session()?;
    let power_manager = PowerManager::new(mac, addr, &dbus)?;
    let match_rule = SessionManagerPresenceStatusChanged::match_rule(None, None);

    dbus.add_match(
        match_rule,
        move |signal: SessionManagerPresenceStatusChanged, _dbus, _msg| {
            match PresenceStatus::try_from(signal.status) {
                Ok(status) => {
                    eprintln!("Got presence status {status:?}");
                    power_manager.set_power(status.is_active());
                }
                Err(e) => {
                    eprintln!("Failed to parse presence status: {e}");
                }
            }
            true // Returning true keeps the match active.
        },
    )?;

    eprintln!("Listening to DBUS messages");
    loop {
        dbus.process(Duration::MAX)?;
    }
}
