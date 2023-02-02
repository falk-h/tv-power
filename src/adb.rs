use std::{
    ffi::OsStr,
    io::ErrorKind,
    net::SocketAddr,
    os::unix::process::ExitStatusExt,
    process::{Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use color_eyre::{
    eyre::{self, Context, ContextCompat},
    Result, Section,
};
use nix::sys::signal::Signal;

pub fn shell(
    addr: SocketAddr,
    command: impl IntoIterator<Item = impl AsRef<OsStr>>,
    timeout: Option<Duration>,
) -> Result<()> {
    let start = Instant::now();
    ensure_connected(addr, timeout)?;

    // Then send a keyevent to the TV to turn it off. 26 corresponds to the
    // power button.
    let mut child = Command::new("adb")
        .args(["-s", &addr.to_string(), "shell"])
        .args(command)
        .spawn()
        .context("Failed to invoke adb")
        .suggestion("Make sure that adb is installed")?;

    let timeout = timeout.unwrap_or(Duration::MAX);
    while start.elapsed() < timeout {
        match child.try_wait().context("Failed to wait for child")? {
            Some(status) => {
                return check_status(status, "adb").context("adb shell command failed");
            }
            None => thread::sleep(Duration::from_millis(50)),
        }
    }

    let status = child
        .wait()
        .context("Failed to wait for child after killing it")?;
    check_status(status, "adb").context("adb shell command out")
}

pub fn send_keycode(addr: SocketAddr, keycode: i32, timeout: Option<Duration>) -> Result<()> {
    shell(addr, ["input", "keyevent", &keycode.to_string()], timeout)
}

pub fn send_keycodes(
    addr: SocketAddr,
    keycodes: impl IntoIterator<Item = i32>,
    timeout: Option<Duration>,
) -> Result<()> {
    let cmd = keycodes
        .into_iter()
        .map(|n| format!("input keyevent {n}"))
        .collect::<Vec<_>>()
        .join(" && ");

    shell(addr, [cmd], timeout)
}

/// Makes sure we're connected to the TV.
fn ensure_connected(addr: SocketAddr, timeout: Option<Duration>) -> Result<()> {
    let start = Instant::now();
    let mut child = Command::new("adb")
        .arg("connect")
        .arg(addr.to_string())
        .stdout(Stdio::null())
        .spawn()
        .context("Failed to invoke adb")
        .suggestion("Make sure that adb is installed")?;

    let timeout = timeout.unwrap_or(Duration::MAX);
    while start.elapsed() < timeout {
        match child.try_wait().context("Failed to wait for child")? {
            Some(status) => {
                return check_status(status, "adb")
                    .with_context(|| format!("Connecting to {addr} over adb failed"))
            }
            None => thread::sleep(Duration::from_millis(50)),
        }
    }

    log::error!("Connecting to TV timed out");

    match child.kill() {
        // Child died before we could kill it.
        Err(e) if e.kind() != ErrorKind::InvalidInput => {
            return Err(e).context("Failed to kill child")
        }
        _ => {}
    }

    let status = child
        .wait()
        .context("Failed to wait for child after killing it")?;
    check_status(status, "adb").context("Connecting to TV timed out")
}

fn check_status(status: ExitStatus, name: &str) -> Result<()> {
    match status
        .code()
        .with_context(|| format!("{name} didn't return a status code?"))?
    {
        0 => Ok(()),
        n => match status.signal() {
            Some(sig) => match Signal::try_from(sig) {
                Ok(sig) => eyre::bail!("{name} died from signal {sig}"),
                Err(_) => eyre::bail!("{name} died from unknown signal {sig}"),
            },
            None => eyre::bail!("{name} exited with status {n}"),
        },
    }
}
