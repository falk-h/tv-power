use std::{
    collections::HashSet,
    net::{IpAddr, SocketAddr},
};

use clap::{Args, CommandFactory, Parser};
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

impl Command {
    pub fn env_vars() -> HashSet<String> {
        env_vars_inner(&Self::command())
    }
}

fn env_vars_inner(command: &clap::Command) -> HashSet<String> {
    let mut envs: HashSet<_> = command
        .get_arguments()
        .into_iter()
        .filter_map(|a| a.get_env())
        .map(|e| e.to_os_string().into_string().unwrap())
        .collect();

    for subcommand in command.get_subcommands() {
        envs.extend(env_vars_inner(subcommand));
    }

    envs
}
