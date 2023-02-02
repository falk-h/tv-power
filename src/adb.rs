use std::{
    ffi::OsStr,
    net::SocketAddr,
    process::{Command, Stdio},
};

use color_eyre::{
    eyre::{self, Context},
    Result, Section,
};

pub fn shell(addr: SocketAddr, command: impl IntoIterator<Item = impl AsRef<OsStr>>) -> Result<()> {
    ensure_connected(addr)?;

    // Then send a keyevent to the TV to turn it off. 26 corresponds to the
    // power button.
    let status = Command::new("adb")
        .args(["-s", &addr.to_string(), "shell"])
        .args(command)
        .status()
        .context("Failed to invoke adb")
        .suggestion("Make sure that adb is installed")?;
    eyre::ensure!(
        status.success(),
        "Sending keyevent to {addr} over ADB failed",
    );

    Ok(())
}

pub fn send_keycode(addr: SocketAddr, keycode: i32) -> Result<()> {
    shell(addr, ["input", "keyevent", &keycode.to_string()])
}

pub fn send_keycodes(addr: SocketAddr, keycodes: impl IntoIterator<Item = i32>) -> Result<()> {
    let cmd = keycodes
        .into_iter()
        .map(|n| format!("input keyevent {n}"))
        .collect::<Vec<_>>()
        .join(" && ");

    shell(addr, [cmd])
}

/// Makes sure we're connected to the TV.
fn ensure_connected(addr: SocketAddr) -> Result<()> {
    let status = Command::new("adb")
        .arg("connect")
        .arg(addr.to_string())
        .stdout(Stdio::null())
        .status()
        .context("Failed to invoke adb")
        .suggestion("Make sure that adb is installed")?;
    eyre::ensure!(status.success(), "Connecting to {addr} over ADB failed");
    Ok(())
}
