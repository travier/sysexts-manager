use std::path::Path;

use anyhow::Result;
use clap::{Parser, Subcommand};
use log::{LevelFilter, debug, info, trace};

mod manager;
mod ostree;
mod sysext;

// sub commands:
// status
// update / sync?
// track / add
// untrack // remove

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
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let level = match cli.verbose {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    env_logger::Builder::new()
        .filter(None, level)
        .format_timestamp(None)
        .init();

    let mut manager = manager::new_with_root(Path::new("test"))?;
    manager.load_config()?;
    manager.load_images()?;

    // for deployment
    // read os-release
    // find current release (version_id) & architecture & variant (?) to filter sysexts
    // ostree::rpm_ostree_status()?;

    match &cli.command {
        Command::Symlinks {} => manager.enable(),
    }
}
