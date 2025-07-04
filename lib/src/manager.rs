use std::collections::HashMap;
use std::env::consts::ARCH;
use std::fmt;
use std::fs::{self, File, remove_file, symlink_metadata};
// use std::io::BufReader;
use std::io::prelude::*;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use hex;
use log::{debug, error, info, warn};
use os_release::OsRelease;
// use cap_std::fs::Dir;
use sha2::{Digest, Sha256};
use version_compare::{Cmp, compare};

use super::sysext::{Config, Image};

#[derive(Debug, Clone, PartialEq)]
#[allow(non_camel_case_types)]
pub enum Architecture {
    x86_64,
    aarch64,
}

impl fmt::Display for Architecture {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Architecture::x86_64 => "x86_64",
            Architecture::aarch64 => "aarch64",
        };
        write!(f, "{s}")
    }
}

pub struct Manager {
    system: System,
    configs: HashMap<String, Config>,
    images: HashMap<String, Vec<Image>>,
    rootdir: PathBuf,
}

struct System {
    arch: Architecture,
    version_id: String,
}

const CONFIGURATION_DIRECTORIES: &[&str] = &[
    "run/sysexts-manager",
    "etc/sysexts-manager",
    "usr/lib/sysexts-manager",
];

const MUTABLE_CONFIGURATION_DIRECTORIES: &[&str] = &["run/sysexts-manager", "etc/sysexts-manager"];

const DEFAULT_CONFIG_DIR: &str = "etc/sysexts-manager";
const DEFAULT_STORE: &str = "var/lib/extensions.d";

#[allow(dead_code)]
pub fn new() -> Result<Manager> {
    // let dir = cap_std::open_ambient_dir("/")?;
    // new_with_root(Path::new(dir))
    new_with_root(Path::new("/"))
}

pub fn new_with_root(path: &Path) -> Result<Manager> {
    if path.as_os_str() != "/" {
        info!("Setting up for rootfs: {}", path.display());
    } else {
        debug!("Setting up for /");
    }

    let arch = match ARCH {
        "x86_64" => Architecture::x86_64,
        "aarch64" => Architecture::aarch64,
        a => {
            return Err(anyhow!("Architecture is not supported: {a}"));
        }
    };

    let release = OsRelease::new()?;
    let version_id = release.version_id;

    debug!("Found arch: {arch} | version_id: {version_id}");

    Ok(Manager {
        system: System { arch, version_id },
        configs: HashMap::new(),
        images: HashMap::new(),
        rootdir: path.into(),
    })
}

impl Manager {
    pub fn load_config(&mut self) -> Result<()> {
        for dir in CONFIGURATION_DIRECTORIES {
            let configdir = self.rootdir.join(dir);
            debug!("Looking for configuration in: {}", configdir.display());
            let Ok(files) = fs::read_dir(&configdir) else {
                debug!(
                    "Could not find configuration directory: {}",
                    configdir.display()
                );
                continue;
            };
            for file in files {
                let Ok(filename) = file else {
                    error!("Could not get filename from direntry");
                    continue;
                };
                let Ok(config) = Config::new(filename.path().as_path()) else {
                    error!(
                        "Error reading configuration file: {}",
                        filename.path().display()
                    );
                    continue;
                };
                debug!("Valid configuration file for sysext: {:?}", &config);
                if self.configs.contains_key(&config.Name) {
                    info!("Ignoring config file: {}", filename.path().display())
                } else {
                    self.configs.insert(config.Name.clone(), config);
                }
            }
        }
        let sysexts: Vec<String> = self.configs.keys().cloned().collect();

        if sysexts.is_empty() {
            info!("No configuration found");
        } else {
            info!("Loaded configuration for sysexts: {}", sysexts.join(", "));
        }
        Ok(())
    }

    pub fn load_images(&mut self) -> Result<()> {
        let sysext_store = self.rootdir.join(DEFAULT_STORE);
        debug!("Looking for sysext images in: {}", sysext_store.display());
        let Ok(files) = fs::read_dir(&sysext_store) else {
            debug!(
                "Could not find sysext directory: {}",
                sysext_store.display()
            );
            return Ok(());
        };
        for file in files {
            let Ok(filename) = file else {
                error!("Could not get filename from direntry");
                continue;
            };
            debug!("Looking at sysext image: {}", filename.path().display());
            let mut found = false;
            for name in self.configs.keys() {
                let filename_osstr = filename.file_name();
                let filename_str = filename_osstr.to_str().unwrap();
                if filename_str.starts_with(name) {
                    found = true;
                    let Ok(image) = Image::new(name, filename.file_name()) else {
                        error!("Invalid sysext name: {}", filename.path().display());
                        break;
                    };
                    match self.images.get_mut(&image.name) {
                        None => {
                            debug!("Adding sysext image to new list: {image:?}");
                            let name = image.name.clone();
                            let vec = vec![image];
                            self.images.insert(name, vec);
                        }
                        Some(v) => {
                            debug!("Adding sysext image to existing list: {image:?}");
                            v.push(image);
                        }
                    }
                    break;
                }
            }
            if !found {
                error!(
                    "Could not find a matching config for sysext image: {}. Ignoring",
                    filename.path().display()
                );
            }
        }
        let sysexts: Vec<String> = self
            .images
            .iter()
            .map(|(name, image)| {
                format!(
                    "{} ({})",
                    name,
                    image
                        .iter()
                        .map(|i| { i.version.clone() })
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            })
            .collect();
        if sysexts.is_empty() {
            info!("No sysext loaded");
        } else {
            info!("Loaded versions for sysexts: {}", sysexts.join(", "));
        }
        Ok(())
    }

    pub fn enable(&self) -> Result<()> {
        let run_extensions = self.rootdir.join("run/extensions");

        if !run_extensions.exists() {
            debug!("Creating {}", &run_extensions.display());
            fs::create_dir(&run_extensions)?;
        }
        if !fs::metadata(&run_extensions)?.is_dir() {
            return Err(anyhow!("{} is not a directory", &run_extensions.display()));
        }

        let mut images = Vec::new();

        for (name, config) in &self.configs {
            let Some(sysexts) = self.images.get(name) else {
                error!(
                    "Config found for '{name}' but no sysexts found. Not setting up. Do an update?"
                );
                continue;
            };
            if config.Kind != "latest" {
                unimplemented!();
            }

            let mut latest = None;
            for image in sysexts {
                // Filter images that we can not use
                if image.architecture != self.system.arch {
                    info!("Ignoring '{}' (incompatible architecture)", image.path());
                    continue;
                }
                if image.version_id != self.system.version_id {
                    info!("Ignoring '{}' (incompatible release)", image.path());
                    continue;
                }
                match &latest {
                    None => {
                        latest = Some(image.clone());
                    }
                    Some(l) => match compare(&image.version, &l.version) {
                        Ok(Cmp::Lt) => debug!("{}: Skipping {}", name, image.version),
                        Ok(Cmp::Eq) => error!(
                            "{}: Should never happen: {} = {}",
                            name, image.version, l.version
                        ),
                        Ok(Cmp::Gt) => {
                            debug!("{}: Selecting {}", name, image.version);
                            latest = Some(image.clone());
                        }
                        _ => return Err(anyhow!("Invalid version number")),
                    },
                };
            }
            match latest {
                None => {
                    error!("No image to enable for sysext: {name}");
                    continue;
                }
                Some(img) => {
                    debug!("Enabling sysext: {} {}", img.name, img.version);
                    images.push(img);
                }
            };
        }

        let enable = images
            .iter()
            .map(|image| format!("{} ({})", image.name, image.version))
            .collect::<Vec<String>>()
            .join(", ");
        if enable.is_empty() {
            info!("No sysexts to enable");
            return Ok(());
        }
        info!("Enabling sysexts: {enable}");

        for image in images {
            // Setup symlinks in /run/extensions.version for the sysexts found
            let original = format!("../../var/lib/extensions.d/{}", image.path());
            let link = format!("{}/{}.raw", run_extensions.display(), image.name);
            debug!("{link} -> {original}");
            if symlink_metadata(&link).is_ok() {
                remove_file(&link)?
            };
            symlink(original, link)?;
        }

        Ok(())
    }

    pub fn add_sysext(&self, name: &str, kind: &str, url: &str, force: &bool) -> Result<()> {
        debug!("Adding sysext config: {name}, {kind}, {url} (override: {force})");

        let conf = Config {
            Name: name.into(),
            Kind: kind.into(),
            Url: url.into(),
        };

        let configdir = self.rootdir.join(DEFAULT_CONFIG_DIR);
        if !configdir.exists() {
            debug!("Creating {}", &configdir.display());
            fs::create_dir(&configdir)?;
        }
        if !fs::metadata(&configdir)?.is_dir() {
            return Err(anyhow!("{} is not a directory", &configdir.display()));
        }

        let conffile = configdir.join(format!("{name}.conf"));
        if conffile.exists() && !*force {
            return Err(anyhow!(
                "{} already exists (use --force to override it)",
                conffile.display()
            ));
        }

        let mut file = File::create(conffile)?;
        file.write_all(&toml::to_string(&conf)?.into_bytes())?;

        // TODO: Add config to manager

        println!("Successfully added configuration for sysext: {name}");

        Ok(())
    }

    pub fn remove_sysext(&mut self, name: &str) -> Result<()> {
        debug!("Removing sysext config and images: {name}");

        match self.configs.get(name) {
            None => {
                info!("No configuration found for: {name}");
                return Ok(());
            }
            Some(_c) => {}
        };
        self.configs.remove(name);

        match self.images.get(name) {
            None => {
                debug!("No images to remove");
            }
            Some(v) => {
                let images = v
                    .iter()
                    .filter(|i| i.name == name)
                    .map(|i| i.path())
                    .collect::<Vec<String>>();
                let sysext_store = self.rootdir.join(DEFAULT_STORE);
                for image in images {
                    let path = sysext_store.join(image);
                    info!("Removing sysext image: {}", path.display());
                    remove_file(&path)?
                }
            }
        };

        // Try removing the config from /run and /etc, ignore /usr
        for dir in MUTABLE_CONFIGURATION_DIRECTORIES {
            let configdir = self.rootdir.join(dir);
            let conffile = configdir.join(format!("{name}.conf"));
            if conffile.exists() {
                info!("Removing sysext config: {}", conffile.display());
                remove_file(&conffile)?;
            }
        }

        println!("Successfully removed configuration and images for sysext: {name}");

        Ok(())
    }

    fn update_sysext(&self, config: &Config, images: &Vec<Image>) -> Result<()> {
        debug!(
            "Downloading SHA256SUMS for: {} (version_id: {}, arch: {})",
            config.Name, self.system.version_id, self.system.arch
        );

        // info!("curl --location --silent --output ... {}/SHA256SUMS", config.Url);
        let client = reqwest::blocking::Client::new();
        let sha256sums = client
            .get(format!("{}/SHA256SUMS", config.Url))
            .send()?
            .text()?;
        // println!("{}", sha256sums.to_string());

        let mut hashes: Vec<(String, Image)> = Vec::new();
        for line in sha256sums.lines() {
            let mut split = line.split("  ");
            let hash: String = match split.next() {
                Some(s) => s.into(),
                None => {
                    error!("Invalid line in SHA256SUMS file: {line}");
                    continue;
                }
            };
            let filename: String = match split.next() {
                Some(s) => s.into(),
                None => {
                    error!("Invalid line in SHA256SUMS file: {line}");
                    continue;
                }
            };
            let Ok(image) = Image::new(&config.Name, filename.clone().into()) else {
                error!("Invalid sysext name: {filename}");
                continue;
            };
            hashes.push((hash, image));
        }
        let parsed_sha256sums = hashes
            .iter()
            .map(|(h, i)| format!("{} ({})", i.path(), h))
            .collect::<Vec<String>>()
            .join("\n");
        if parsed_sha256sums.is_empty() {
            warn!("Empty SHA256SUMS file for: {}", config.Name);
            return Ok(());
        }
        debug!("Found potential sysexts:\n{parsed_sha256sums}");

        // Look for latest remote image
        let mut latest_remote = None;
        for (hash, image) in hashes {
            // Filter images that we can not use
            if image.architecture != self.system.arch {
                debug!("Ignoring '{}' (incompatible architecture)", image.path());
                continue;
            }
            if image.version_id != self.system.version_id {
                debug!("Ignoring '{}' (incompatible release)", image.path());
                continue;
            }
            match &latest_remote {
                None => {
                    latest_remote = Some((hash, image.clone()));
                }
                Some((_h, i)) => match compare(&image.version, &i.version) {
                    Ok(Cmp::Lt) => debug!("{}: Skipping {}", config.Name, image.version),
                    Ok(Cmp::Eq) => error!(
                        "{}: Should never happen: {} = {}",
                        config.Name, image.version, i.version
                    ),
                    Ok(Cmp::Gt) => {
                        debug!("{}: Selecting {}", config.Name, image.version);
                        latest_remote = Some((hash, image.clone()));
                    }
                    _ => panic!("Invalid version number"),
                },
            };
        }
        let (remote_hash, remote_image) = match latest_remote {
            None => {
                error!("No remote valid image found for sysext: {}", config.Name);
                return Ok(());
            }
            Some((h, i)) => {
                debug!(
                    "Found latest remote image for sysext: {} {}",
                    i.name, i.version
                );
                (h, i)
            }
        };

        let mut latest_local = None;
        for image in images {
            // Filter images that we can not use
            if image.architecture != self.system.arch {
                debug!("Ignoring '{}' (incompatible architecture)", image.path());
                continue;
            }
            if image.version_id != self.system.version_id {
                debug!("Ignoring '{}' (incompatible release)", image.path());
                continue;
            }
            match &latest_local {
                None => {
                    latest_local = Some(image.clone());
                }
                Some(l) => match compare(&image.version, &l.version) {
                    Ok(Cmp::Lt) => debug!("{}: Skipping {}", config.Name, image.version),
                    Ok(Cmp::Eq) => error!(
                        "{}: Should never happen: {} = {}",
                        config.Name, image.version, l.version
                    ),
                    Ok(Cmp::Gt) => {
                        debug!("{}: Selecting {}", config.Name, image.version);
                        latest_local = Some(image.clone());
                    }
                    _ => return Err(anyhow!("Invalid version number")),
                },
            };
        }
        let (download_hash, download_image) = match latest_local {
            None => {
                info!(
                    "No local image for sysext: {}. Downloading: {}",
                    config.Name, remote_image.version
                );
                (remote_hash, remote_image)
            }
            Some(img) => {
                debug!(
                    "Comparing latest local & remote image for sysext '{}': local: '{}' remote: '{}'",
                    img.name, img.version, remote_image.version
                );
                match compare(&img.version, &remote_image.version) {
                    Ok(Cmp::Lt) => {
                        info!("Downloading: {}", remote_image.version);
                        (remote_hash, remote_image)
                    }
                    Ok(Cmp::Eq) => {
                        println!("No update found for '{}'", img.name);
                        // TODO check hash
                        return Ok(());
                    }
                    Ok(Cmp::Gt) => {
                        debug!("Local image is newer for '{}': {}", img.name, img.version);
                        return Ok(());
                    }
                    _ => {
                        return Err(anyhow!(
                            "Invalid version number: {} or {}",
                            img.version,
                            remote_image.version
                        ));
                    }
                }
            }
        };

        println!("Downloading update: {}", download_image.path());
        // info!("curl --location --silent --output ... {}/{}", config.Url, download_image.path());
        // Find latest version that matches version_id
        // Check if we already have it and check its SHA256SUM again
        // Download if needed via systemd-run command call to curl
        debug!("Downloading: {}/{}", config.Url, download_image.path());
        let sysext_body = client
            .get(format!("{}/{}", config.Url, download_image.path()))
            .send()?
            .bytes()?;

        // TODO: Compute that progressively while downloading
        let mut hasher = Sha256::new();
        hasher.update(&sysext_body);
        let result = hex::encode(hasher.finalize());

        if result != download_hash {
            return Err(anyhow!(
                "Invalid hash for {}: got {} vs expected {}",
                download_image.path(),
                result,
                download_hash
            ));
        }
        debug!("Valid hash for {} {}", download_image.path(), download_hash);

        let sysext_store = self.rootdir.join(DEFAULT_STORE);
        let sysextfile = sysext_store.join(download_image.path());

        // Write to a temp file and rename
        let mut file = File::create(&sysextfile)?;
        file.write_all(&sysext_body)?;

        info!("Downloaded: {}", sysextfile.display());

        // TODO: Add image to manager

        println!("Successfully updated sysexts: {}", config.Name);

        Ok(())
    }

    pub fn update(&self) -> Result<()> {
        info!("Updating all sysexts");

        let sysext_store = self.rootdir.join(DEFAULT_STORE);
        if !sysext_store.exists() {
            debug!("Creating {}", &sysext_store.display());
            fs::create_dir(&sysext_store)?;
        }
        if !fs::metadata(&sysext_store)?.is_dir() {
            return Err(anyhow!("{} is not a directory", &sysext_store.display()));
        }

        let empty: Vec<Image> = vec![];
        for (n, c) in &self.configs {
            let images = self.images.get(n).unwrap_or(&empty);
            self.update_sysext(c, images)?;
        }

        println!("Successfully updated all sysexts");
        Ok(())
    }

    pub fn status(&self) -> Result<()> {
        println!("sysexts:");
        for (n, c) in &self.configs {
            println!("  {n} ({}, {}):", c.Kind, c.Url);
            match self.images.get(n) {
                None => println!("    No images installed for that sysext"),
                Some(images) => {
                    for i in images {
                        println!("    {}", i.path())
                    }
                }
            };
        }
        Ok(())
    }
}
