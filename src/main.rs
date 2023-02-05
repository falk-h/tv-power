use std::{io::Write, net::SocketAddr, process, time::Duration};

use clap::Parser;
use color_eyre::{eyre, Result};
use dbus::{blocking::LocalConnection, message::SignalArgs};
use libsystemd::daemon::{self, NotifyState};
use log::Level;
use mac_address::MacAddress;

use cli::Command;
use power::PowerManager;
use presence::{generated::SessionManagerPresenceStatusChanged, PresenceStatus};

mod adb;
mod cli;
mod config;
mod outputs;
mod power;
mod presence;

fn main() -> Result<()> {
    init_logging()?;

    if let Err(e) = config::apply_env_overrides() {
        log::error!("{e:#}");
        process::exit(2);
    }

    let cmd = Command::parse();
    use Command::*;
    match cmd {
        On { mac, sockaddr } => power::turn_on(sockaddr.to_std(), mac.mac),
        Off { sockaddr } => power::turn_off(sockaddr.to_std()),
        Service {
            mac,
            sockaddr,
            output,
        } => service(mac.mac, sockaddr.to_std(), output),
        ListOutputs {} => outputs::list(),
        Keycodes { sockaddr, keycodes } => adb::send_keycodes(sockaddr.to_std(), keycodes, None),
    }
}

fn init_logging() -> Result<()> {
    let use_systemd = systemd_journal_logger::connected_to_journal();

    if use_systemd {
        color_eyre::config::HookBuilder::new()
            .display_env_section(false)
            .install()?;
    } else {
        color_eyre::install()?;
    }

    if use_systemd {
        systemd_journal_logger::init()?;
        log::set_max_level(Level::Trace.to_level_filter());
        log::trace!("Logging to systemd journal");
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
            .format(|buf, record| {
                let level = record.level();

                if level != Level::Info {
                    let style = buf.default_level_style(level);
                    write!(buf, "{} ", style.value(level.as_str().to_lowercase() + ":"))?;
                }

                writeln!(buf, "{}", record.args())
            })
            .try_init()?;
    }

    Ok(())
}

fn service(mac: MacAddress, addr: SocketAddr, output: Option<String>) -> Result<()> {
    let dbus = connect_dbus()?;
    let power_manager = PowerManager::new(mac, addr, &dbus, output)?;
    let match_rule = SessionManagerPresenceStatusChanged::match_rule(None, None);

    dbus.add_match(
        match_rule,
        move |signal: SessionManagerPresenceStatusChanged, _dbus, _msg| {
            match PresenceStatus::try_from(signal.status) {
                Ok(status) => {
                    log::debug!("Got presence status {status:?}");
                    power_manager.set_power(status.is_active());
                }
                Err(e) => {
                    log::error!("Failed to parse presence status: {e}");
                }
            }
            true // Returning true keeps the match active.
        },
    )?;

    daemon::notify(
        false,
        &[NotifyState::Ready, NotifyState::Status("Idle".to_owned())],
    )?;
    log::info!("Listening to DBUS messages");
    loop {
        dbus.process(Duration::MAX)?;
    }
}

fn connect_dbus() -> Result<LocalConnection> {
    log::trace!("Connecting to DBUS");
    daemon::notify(false, &[NotifyState::Status("Waiting for DBUS".to_owned())])?;

    let attempts = 30;
    for attempt in 1..=attempts {
        match LocalConnection::new_session() {
            Ok(dbus) => return Ok(dbus),
            Err(e) => log::warn!("Failed to connect to DBUS: {e} (attempt {attempt}/{attempts})"),
        }
    }

    eyre::bail!("Failed to connect to DBUS after {attempts} attempts")
}
