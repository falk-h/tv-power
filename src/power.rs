use std::{
    net::{IpAddr, SocketAddr},
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
use mac_address::MacAddress;

use crate::presence::{generated::SessionManagerPresence, PresenceStatus};

pub struct PowerManager {
    sender: Sender<bool>,
}

impl PowerManager {
    pub fn new(
        mac: MacAddress,
        addr: SocketAddr,
        dbus: &LocalConnection,
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
        eprintln!("Got initial presence status {last_status:?}");

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
                eprintln!("Turning TV {onoff}");

                while receiver.is_empty() {
                    let result = if power_on {
                        turn_on(mac)
                    } else {
                        turn_off(addr)
                    };

                    if let Err(e) = result {
                        eprintln!("Failed to turn TV {onoff}: {e}");
                    }

                    if let Ok(on) = ping_tv(addr.ip()) {
                        if on == power_on {
                            eprintln!("Turned TV {onoff}");
                            break;
                        }
                        let delay = 200;
                        eprintln!("TV is not yet on, retrying in {delay}ms");
                        thread::sleep(Duration::from_millis(delay));
                    }
                }
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

pub fn ping_tv(addr: IpAddr) -> Result<bool> {
    // Send one ping with a 200ms timeout.
    let status = Command::new("ping")
        .args(["-q", "-c", "1", "-W", "0.2"])
        .arg(addr.to_string())
        .stdout(Stdio::null())
        .status()
        .context("Failed to invoke ping")?;
    Ok(status.success())
}

pub fn turn_on(mac: MacAddress) -> Result<()> {
    let mac = wol::MacAddr(mac.bytes());

    // Send a magic wake-on-LAN packet to the TV to wake it. Actually, a
    // wake-on-WLAN packet, as it's on WiFi.
    wol::send_wol(mac, None, None)?;

    Ok(())
}

pub fn turn_off(addr: SocketAddr) -> Result<()> {
    // Make sure we're connected to the TV.
    let status = Command::new("adb")
        .arg("connect")
        .arg(addr.to_string())
        .stdout(Stdio::null())
        .status()
        .context("Failed to invoke ADB")?;
    eyre::ensure!(status.success(), "Connecting to {addr} over ADB failed");

    // Then send a keyevent to the TV to turn it off. 26 corresponds to the
    // power button.
    let status = Command::new("adb")
        .arg("-s")
        .arg(addr.to_string())
        .args(["shell", "input", "keyevent", "26"])
        .status()
        .context("Failed to invoke ADB")?;
    eyre::ensure!(
        status.success(),
        "Sending keyevent to {addr} over ADB failed",
    );

    Ok(())
}
