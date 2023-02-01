use std::net::SocketAddr;

use clap::Parser;
use mac_address::MacAddress;

/// TV power manager.
#[derive(Debug, Parser)]
pub enum Command {
    /// Turn the TV on.
    On {
        /// The TV's MAC address.
        #[arg(env)]
        mac: MacAddress,
    },

    /// Turn the TV off.
    Off {
        /// The TV's IP address and ADB port.
        ///
        /// The port should usually be 5555.
        #[arg(env)]
        addr: SocketAddr,
    },

    /// Runs in a service mode, turning the TV off when the computer is idle.
    Service {
        /// The TV's MAC address.
        #[arg(env)]
        mac: MacAddress,

        /// The TV's IP address and ADB port.
        ///
        /// The port should usually be 5555.
        #[arg(env)]
        addr: SocketAddr,
    },
}
