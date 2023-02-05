use std::{env, path::PathBuf};

use color_eyre::{eyre::Context, Report, Result};

use crate::cli::Command;

const CONFIG_FILE: &str = "tv-power.conf";

fn config_dir() -> PathBuf {
    let config_dir = match env::var_os("XDG_CONFIG_DIR") {
        Some(d) => PathBuf::from(d),
        None => {
            let home_dir = env::var_os("HOME").expect(
                "Can't find the config directory as neither $XDG_CONFIG_DIR nor $HOME are set",
            );
            PathBuf::from(home_dir).join(".config")
        }
    };
    let program_name = env!("CARGO_BIN_NAME");
    config_dir.join(program_name)
}

fn config_file() -> PathBuf {
    config_dir().join(CONFIG_FILE)
}

pub fn apply_env_overrides() -> Result<()> {
    let file = config_file();
    log::debug!("Using config file path {file:?}");

    let vars = match dotenvy::from_filename_iter(&file) {
        Ok(vars) => vars,
        Err(e) if e.not_found() => {
            log::debug!("Ignoring config file {file:?} as it doesn't exist");
            return Ok(());
        }
        Err(e) => {
            return Err(Report::new(e).wrap_err(format!("Failed to read config file {file:?}")))
        }
    };

    let expected_vars = Command::env_vars();

    for v in vars {
        let (var, val) = v.with_context(|| format!("Failed to parse config file {file:?}"))?;
        let var_upper = var.to_uppercase();

        if !expected_vars.contains(&var_upper) {
            log::warn!("Unexpected configuration key {var}");

            let mut keys = Vec::from_iter(expected_vars.clone());
            keys.sort();
            let keys = keys.join(", ").to_lowercase();

            log::warn!("Valid keys are {keys} (case insensitive)");
            continue;
        }

        match env::var_os(&var_upper) {
            Some(v) => log::debug!(
                "Not using {var_upper}={val:?} from config as it's set to {v:?} in the environment",
            ),
            None => {
                log::debug!("Using {var_upper}={val:?} from config");
                env::set_var(var_upper, val);
            }
        }
    }

    Ok(())
}
