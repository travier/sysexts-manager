// SPDX-FileCopyrightText: Timothée Ravier <tim@siosm.fr>
// SPDX-License-Identifier: MIT

use std::num::NonZero;
use std::path::PathBuf;
use std::result::Result::Ok;
use std::thread::available_parallelism;
use std::time::Duration;
use std::{process, thread::sleep};

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use log::{LevelFilter, debug};

#[derive(Parser, Debug)]
#[command(version, about, long_about = "systemd system extension manager")]
struct Cli {
    /// Log verbosity. Defaults to Warn, -v for Info, -vv for Debug, -vvv for Trace.
    #[arg(short = 'v', long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// How many threads will be used for parallel operations such as image downloads. Defaults to two times the number of CPU threads.
    #[arg(short = 'j', long, global = true, default_value_t = 0)]
    jobs: u8,

    /// Takes a directory path as an argument. All paths will be prefixed with the given alternate root path, including config search paths.
    #[arg(long, global = true)]
    root: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Enable all sysexts, or only one if specified
    Enable {
        /// The sysext to enable instead of operating on all sysexts
        name: Option<String>,
    },
    /// Disable all sysexts, or only one if specified
    Disable {
        /// The sysext to disable instead of operating on all sysexts
        name: Option<String>,
    },
    /// Add configuration for a sysext
    Add {
        /// Name of the sysext
        name: String,
        /// Base URL where the sysext and its SHAS256SUMS file are hosted
        url: String,
        /// Override any existing configuration file
        #[arg(short, long, default_value_t = false)]
        force: bool,
    },
    /// Remove configuration and images for a sysext
    Remove {
        /// Name of the sysext
        name: String,
    },
    /// Update all configured sysexts
    Update {},
    /// Refresh enabled sysexts
    Refresh {},
    /// Status of sysexts
    Status {},
}
// Download {
//     /// Name of the sysext
//     name: String,
//     /// Override operating system VERSION_ID
//     version_id: Option<String>,
// }
// Clean

fn refresh() -> Result<()> {
    debug!("Asking systemd-sysusers to refresh enabled sysexts");
    let mut cmd = process::Command::new("systemctl");
    cmd.args(["restart", "systemd-sysext.service"]);
    let res = match cmd.status() {
        Ok(s) => {
            if !s.success() {
                Err(anyhow!("Failed to refresh sysexts"))
            } else {
                debug!("sysexts successfully refreshed");
                Ok(())
            }
        }
        Err(_e) => Err(anyhow!("Failed to refresh sysexts")),
    };
    // FIXME
    sleep(Duration::new(1, 0));
    res
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let level = match cli.verbose {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    env_logger::Builder::new()
        .filter(None, level)
        .format_timestamp(None)
        .init();

    let mut manager = match cli.root {
        None => sysexts_manager_lib::manager::new()?,
        Some(p) => {
            let p = PathBuf::from(p);
            sysexts_manager_lib::manager::new_with_root(&p)?
        }
    };
    manager.load_config()?;
    manager.load_images()?;

    // for deployment
    // read os-release
    // find current release (version_id) & architecture & variant (?) to filter sysexts
    // ostree::rpm_ostree_status()?;

    let jobs = if cli.jobs == 0 {
        available_parallelism()
            .unwrap_or(NonZero::new(1).unwrap())
            .get()
            * 2
    } else {
        cli.jobs.into()
    };
    rayon::ThreadPoolBuilder::new()
        .num_threads(jobs)
        .build_global()
        .unwrap();

    match &cli.command {
        Command::Enable { name } => match name {
            None => manager.enable_all(),
            Some(n) => manager.enable(n),
        },
        Command::Disable { name } => match name {
            None => manager.disable_all(),
            Some(n) => manager.disable(n),
        },
        Command::Add { name, url, force } => manager.add_sysext(name, "latest", url, force),
        Command::Remove { name } => manager.remove_sysext(name),
        Command::Update {} => manager.update(),
        // Command::Download { name, version_id } => manager.download(name, version_id),
        Command::Refresh {} => refresh(),
        Command::Status {} => manager.status(),
    }
}
