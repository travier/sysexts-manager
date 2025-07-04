use std::process;
use std::result::Result::Ok;

use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};
use log::{LevelFilter, debug};

#[derive(Parser, Debug)]
#[command(version, about, long_about = "systemd system extension manager")]
struct Cli {
    /// Log verbosity. Defaults to Info, -v for Debug, -vv for Trace
    #[arg(short = 'v', long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Setup sysexts symlinks in /run/extensions
    Symlinks {},
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
    /// Refresh loaded sysexts
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
    match cmd.status() {
        Ok(s) => {
            if !s.success() {
                Err(anyhow!("Failed to refresh sysexts"))
            } else {
                debug!("sysexts successfully refreshed");
                Ok(())
            }
        }
        Err(_e) => Err(anyhow!("Failed to refresh sysexts")),
    }
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

    let mut manager = sysexts_manager_lib::manager::new()?;
    manager.load_config()?;
    manager.load_images()?;

    // for deployment
    // read os-release
    // find current release (version_id) & architecture & variant (?) to filter sysexts
    // ostree::rpm_ostree_status()?;

    match &cli.command {
        Command::Symlinks {} => manager.enable(),
        Command::Add { name, url, force } => manager.add_sysext(name, "latest", url, force),
        Command::Remove { name } => manager.remove_sysext(name),
        Command::Update {} => manager.update(),
        // Command::Download { name, version_id } => manager.download(name, version_id),
        Command::Refresh {} => refresh(),
        Command::Status {} => manager.status(),
    }
}
