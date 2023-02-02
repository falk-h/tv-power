use std::{
    fmt::{self, Display, Formatter},
    fs,
    path::PathBuf,
};

use color_eyre::{
    eyre::Context,
    owo_colors::{OwoColorize, Stream},
    Result,
};
use owo_colors::colored::Color;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub name: String,
    pub status: Status,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Connected,
    Disconnected,
    Unknown(String),
}

impl Display for Status {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let s = match self {
            Status::Connected => "connected",
            Status::Disconnected => "disconnected",
            Status::Unknown(s) => s,
        };
        write!(f, "{s}",)
    }
}

pub fn all() -> Result<Vec<Output>> {
    find_dirs()?
        .map(|path| -> Result<Output> {
            let name = path.iter().nth(4).unwrap().to_str().unwrap().to_owned();

            let status = fs::read_to_string(path.join("status"))?;
            let status = match status.trim() {
                "connected" => Status::Connected,
                "disconnected" => Status::Disconnected,
                status => {
                    log::warn!("Unknown output status {status:?} for {name}");
                    Status::Unknown(status.to_owned())
                }
            };

            Ok(Output { name, status })
        })
        .collect()
}

pub fn connected() -> Result<impl Iterator<Item = Output>> {
    Ok(all()?.into_iter().filter(|o| o.status == Status::Connected))
}

pub fn exists(name: &str) -> Result<bool> {
    Ok(all()?.into_iter().any(|o| o.name == name))
}

pub fn is_connected(name: &str) -> Result<bool> {
    Ok(connected()?.any(|o| o.name == name))
}

pub fn list() -> Result<()> {
    for Output { name, status } in all()? {
        let color = match status {
            Status::Connected => Color::Green,
            Status::Disconnected => Color::Red,
            Status::Unknown(_) => Color::Yellow,
        };
        let status = status.to_string();
        let status = status.if_supports_color(Stream::Stdout, |s| s.color(color));
        println!("{name} {status}");
    }

    Ok(())
}

fn find_dirs() -> Result<impl Iterator<Item = PathBuf>> {
    let dir = "/sys/class/drm";
    let entries: Result<Vec<_>, _> = fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory {dir}"))?
        .collect();

    Ok(entries?.into_iter().filter_map(|entry| {
        if entry.path().join("status").is_file() {
            Some(entry.path())
        } else {
            None
        }
    }))
}
