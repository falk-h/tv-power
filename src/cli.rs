use std::net::{IpAddr, SocketAddr};

use clap::{Args, Parser};
use mac_address::MacAddress;

/// TV power manager.
#[derive(Debug, Parser)]
pub enum Command {
    /// Turn the TV on.
    On {
        #[command(flatten)]
        mac: MacAddr,

        #[command(flatten)]
        sockaddr: SockAddr,
    },

    /// Turn the TV off.
    Off {
        #[command(flatten)]
        sockaddr: SockAddr,
    },

    /// Runs in a service mode, turning the TV off when the computer is idle.
    Service {
        #[command(flatten)]
        mac: MacAddr,

        #[command(flatten)]
        sockaddr: SockAddr,

        /// Which graphics output to watch to see if the TV is on.
        ///
        /// You can list available outputs with the list-outputs command.
        #[arg(short, long, env)]
        output: Option<String>,
    },

    /// List video outputs.
    ListOutputs {},

    Keycodes {
        #[command(flatten)]
        sockaddr: SockAddr,

        /// Keycodes to send.
        keycodes: Vec<i32>,
    },
}

#[derive(Debug, Args, Clone, Copy)]
pub struct MacAddr {
    /// The TV's MAC address.
    #[arg(env)]
    pub mac: MacAddress,
}

#[derive(Debug, Args, Clone, Copy)]
pub struct SockAddr {
    /// The TV's IP address.
    ///
    /// This is used for connecting to it with adb.
    #[arg(env)]
    pub ip: IpAddr,

    /// The TV's adb port.
    ///
    /// This should usually be 5555.
    #[arg(short, long, env, default_value_t = 5555)]
    pub port: u16,
}

impl SockAddr {
    pub fn to_std(self) -> SocketAddr {
        SocketAddr::from((self.ip, self.port))
    }
}
