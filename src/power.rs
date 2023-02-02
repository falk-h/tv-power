use std::{
    net::SocketAddr,
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

                while receiver.is_empty() {
                    let result = if power_on {
                        turn_on(mac)
                    } else {
                        turn_off(addr)
                    };

                    if let Err(e) = result {
                        log::error!("Failed to turn TV {onoff}: {e}");
                        daemon::notify(
                            false,
                            &[NotifyState::Status(format!("Retrying TV power-{onoff}"))],
                        )
                        .ok();
                    }

                    if let Ok(on) = outputs::is_connected(&output) {
                        if on == power_on {
                            log::info!("Turned TV {onoff}");
                            break;
                        }
                        let delay = 200;
                        log::warn!("TV is not yet {onoff}, retrying in {delay}ms");
                        thread::sleep(Duration::from_millis(delay));
                    }
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

pub fn turn_on(mac: MacAddress) -> Result<()> {
    let mac = wol::MacAddr(mac.bytes());

    // Send a magic wake-on-LAN packet to the TV to wake it. Actually, a
    // wake-on-WLAN packet, as it's on WiFi.
    wol::send_wol(mac, None, None)?;

    Ok(())
}

pub fn turn_off(addr: SocketAddr) -> Result<()> {
    adb::send_keycode(addr, 26)
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
