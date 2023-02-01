use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

#[allow(dead_code)]
pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/generated.rs"));
}

// See https://lira.no-ip.org:8443/doc/gnome-session/dbus/gnome-session.html
#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u32)]
pub enum PresenceStatus {
    Available = 0,
    Invisible = 1,
    Busy = 2,
    Idle = 3,
}

impl PresenceStatus {
    pub fn is_active(self) -> bool {
        self != Self::Idle
    }
}

impl TryFrom<u32> for PresenceStatus {
    type Error = UnknownPresenceStatus;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Available),
            1 => Ok(Self::Invisible),
            2 => Ok(Self::Busy),
            3 => Ok(Self::Idle),
            n => Err(UnknownPresenceStatus(n)),
        }
    }
}

#[derive(Debug)]
pub struct UnknownPresenceStatus(u32);

impl Display for UnknownPresenceStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Unknown presence status: {}", self.0)
    }
}

impl Error for UnknownPresenceStatus {}
