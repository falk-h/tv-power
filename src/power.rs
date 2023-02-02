use std::{
    net::{IpAddr, SocketAddr},
    os::unix::process::ExitStatusExt,
    process::{Command, Stdio},
    thread,
    time::Duration,
};

use color_eyre::{
    eyre::{self, Context},
    Result,
};
use crossbeam::channel::Sender;
use dbus::blocking::LocalConnection;
use libsystemd::daemon::{self, NotifyState};
use mac_address::MacAddress;
use nix::sys::signal::Signal;

use crate::{
    adb, outputs,
    presence::{generated::SessionManagerPresence, PresenceStatus},
};

pub struct PowerManager {
    sender: Sender<bool>,
}

impl PowerManager {
    pub fn new(
        mac: MacAddress,
        addr: SocketAddr,
        dbus: &LocalConnection,
        output: Option<String>,
    ) -> Result<Self> {
        let proxy = dbus.with_proxy(
            "org.gnome.SessionManager",
            "/org/gnome/SessionManager/Presence",
            Duration::from_secs(1),
        );

        let last_status = proxy
            .status()
            .context("Failed to get initial presence status over DBUS")?;
        let last_status = PresenceStatus::try_from(last_status)
            .context("Failed to parse initial presence status")?;
        log::debug!("Got initial presence status {last_status:?}");

        let output = find_output(output).context("Failed to find graphical output")?;
        let last_active = last_status.is_active();
        let (sender, receiver) = crossbeam::channel::unbounded();
        thread::spawn(move || {
            let mut last_active = last_active;

            while let Ok(power_on) = receiver.recv() {
                if power_on == last_active {
                    continue;
                }
                last_active = power_on;

                let onoff = if power_on { "on" } else { "off" };
                let status = format!("Turning TV {onoff}");
                log::info!("{}", status);
                daemon::notify(false, &[NotifyState::Status(status)]).ok();

                while let Err(e) = turn_on_or_off_wait(power_on, addr, mac, &output) {
                    log::error!("Failed to turn TV {onoff}: {e}");
                    daemon::notify(
                        false,
                        &[NotifyState::Status(format!("Retrying TV power-{onoff}"))],
                    )
                    .ok();
                }

                daemon::notify(false, &[NotifyState::Status("Idle".to_owned())]).ok();
            }
        });
        Ok(Self { sender })
    }

    pub fn set_power(&self, power_on: bool) {
        self.sender
            .send(power_on)
            .expect("Failed to send message to worker thread. Did it die?")
    }
}

pub fn turn_on(addr: SocketAddr, mac: MacAddress) -> Result<()> {
    if ping_tv(addr.ip())? {
        log::debug!("TV responds to ping. Trying to turn it on via adb");
        match send_power_key(addr) {
            Ok(()) => {
                log::debug!("Turning on via adb succeeded");
                return Ok(());
            }
            Err(e) => {
                log::warn!("Failed to turn TV back on via adb: {e}");
                log::info!("Trying Wake-on-LAN instead");
                // If we failed to turn the TV back on via adb, it probably
                // turned off WiFi just after we pinged it. Let's try WoL
                // instead.
            }
        }
    }

    let mac = wol::MacAddr(mac.bytes());

    // Send a magic wake-on-LAN packet to the TV to wake it. Actually, a
    // wake-on-WLAN packet, as it's on WiFi.
    wol::send_wol(mac, None, None)?;

    Ok(())
}

pub fn turn_off(addr: SocketAddr) -> Result<()> {
    send_power_key(addr)
}

fn turn_on_wait(addr: SocketAddr, mac: MacAddress, output: &str) -> Result<()> {
    log::info!("Turning on the TV");
    loop {
        log::debug!("Sending WoL packet");
        turn_on(addr, mac)?;

        thread::sleep(Duration::from_millis(100));

        if tv_is_on(addr.ip(), output)? {
            log::info!("Turned on the TV");
            return Ok(());
        }

        log::debug!("TV is not yet on, retrying...")
    }
}

fn turn_off_wait(addr: SocketAddr, output: &str) -> Result<()> {
    log::info!("Turning off the TV");
    turn_off(addr)?;
    log::debug!("Waiting for TV to turn off...");

    loop {
        thread::sleep(Duration::from_millis(100));

        if !tv_is_on(addr.ip(), output)? {
            log::info!("Turned off the TV");
            return Ok(());
        }

        log::debug!("TV is not yet off...");
    }
}

fn turn_on_or_off_wait(on: bool, addr: SocketAddr, mac: MacAddress, output: &str) -> Result<()> {
    if on {
        turn_on_wait(addr, mac, output)
    } else {
        turn_off_wait(addr, output)
    }
}

pub fn send_power_key(addr: SocketAddr) -> Result<()> {
    adb::send_keycode(addr, 26, Some(Duration::from_secs(1)))
}

fn ping_tv(ip: IpAddr) -> Result<bool> {
    // Send one ping with a 200ms timeout.
    let status = Command::new("ping")
        .args(["-q", "-c", "1", "-W", "0.1"])
        .arg(ip.to_string())
        .stdout(Stdio::null())
        .status()
        .context("Failed to invoke ping")?;

    match status.code().expect("ping didn't return a status code?") {
        0 => Ok(true),
        1 => Ok(false),
        n => match status.signal() {
            Some(sig) => match Signal::try_from(sig) {
                Ok(sig) => eyre::bail!("ping died from signal {sig}"),
                Err(_) => eyre::bail!("ping died from unknown signal {sig}"),
            },
            None => eyre::bail!("ping exited with status {n}"),
        },
    }
}

fn tv_is_on(ip: IpAddr, output: &str) -> Result<bool> {
    Ok(ping_tv(ip)? && outputs::is_connected(output)?)
}

fn find_output(output: Option<String>) -> Result<String> {
    if let Some(output) = output {
        eyre::ensure!(outputs::exists(&output)?, "Output {output} doesn't exist");
        return Ok(output);
    }

    match &*outputs::connected()?.collect::<Vec<_>>() {
        [] => eyre::bail!("You haven't specified an output and no outputs are connected"),
        [output] => {
            log::info!("Using output {}", output.name);
            Ok(output.name.clone())
        }
        _ => {
            eyre::bail!("You haven't specified an output but there are multiple connected outputs")
        }
    }
}
