use std::collections::HashMap;
use std::env::consts::ARCH;
use std::fmt;
use std::fs::{self, remove_file, symlink_metadata};
use std::os::unix::fs::symlink;
use std::path::{Display, Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use log::{debug, error, info, trace, warn};
use os_release::OsRelease;
// use cap_std::fs::Dir;
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
        write!(f, "{}", s)
    }
}

pub struct Manager {
    system: System,
    configs: HashMap<String, Config>,
    images: HashMap<String, Vec<Image>>,
    path: PathBuf,
}

struct System {
    arch: Architecture,
    version_id: String,
}

const CONFIGURATION_DIRECTORIES: &'static [&'static str] = &[
    "run/sysext-manager",
    "etc/sysext-manager",
    "usr/lib/sysext-manager",
];

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
            return Err(anyhow!("Architecture is not supported: {}", a));
        }
    };

    let release = OsRelease::new()?;
    let version_id = release.version_id;

    debug!("Found arch: {} | version_id: {}", arch, version_id);

    Ok(Manager {
        system: System { arch, version_id },
        configs: HashMap::new(),
        images: HashMap::new(),
        path: path.into(),
    })
}

impl Manager {
    pub fn load_config(&mut self) -> Result<()> {
        for dir in CONFIGURATION_DIRECTORIES {
            let configdir = self.path.join(dir);
            debug!("Looking for configuration in: {}", configdir.display());
            let Ok(files) = fs::read_dir(&configdir) else {
                debug!("No configuration found in: {}", configdir.display());
                continue;
            };
            for file in files {
                let Ok(filename) = file else {
                    error!("Could not get filename from direntry");
                    continue;
                };
                let Ok(config) = Config::new(filename.path().as_path()) else {
                    error!("Error reading configuration file: {}", filename.path().display());
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
        let sysexts: Vec<String> = self
            .configs
            .iter()
            .map(|(name, _config)| name.clone())
            .collect();

        if sysexts.is_empty() {
            return Err(anyhow!("No configuration found!"));
        }
        info!("Loaded configuration for sysexts: {}", sysexts.join(", "));
        Ok(())
    }

    pub fn load_images(&mut self) -> Result<()> {
        let sysext_store = self.path.join("var/lib/extensions.d");
        debug!("Looking for sysext images in: {}", sysext_store.display());
        let files = fs::read_dir(&sysext_store)?;
        for file in files {
            let Ok(filename) = file else {
                error!("Could not get filename from direntry");
                continue;
            };
            debug!("Looking at sysext image: {}", filename.path().display());
            let mut found = false;
            for (name, _configs) in &self.configs {
                let filename_osstr = filename.file_name();
                let filename_str = filename_osstr.to_str().unwrap();
                if filename_str.starts_with(name) {
                    found = true;
                    let Ok(image) = Image::new(name, filename.file_name()) else {
                        error!("Invalid sysext name: {}", filename.path().display());
                        break;
                    };
                    if image.architecture != self.system.arch {
                        error!(
                            "Incompatible architecture for sysext: {}. Ignoring",
                            filename.path().display()
                        );
                        break;
                    }
                    if image.version_id != self.system.version_id {
                        error!(
                            "Incompatible release for sysext: {}. Ignoring",
                            filename.path().display()
                        );
                        break;
                    }
                    match self.images.get_mut(&image.name) {
                        None => {
                            debug!("Adding sysext image to new list: {:?}", image);
                            let name = image.name.clone();
                            let vec = vec![image];
                            self.images.insert(name, vec);
                        }
                        Some(v) => {
                            debug!("Adding sysext image to existing list: {:?}", image);
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
        info!("Loaded versions for sysexts: {}", sysexts.join(", "));
        Ok(())
    }

    pub fn enable(&self) -> Result<()> {
        let run_extensions = self.path.join("run/extensions");

        let mut images = Vec::new();

        for (name, config) in &self.configs {
            let Some(sysexts) = self.images.get(name) else {
                error!(
                    "Config found for '{}' but no sysexts found. Not setting up. Do an update?",
                    name
                );
                continue;
            };
            if config.Kind != "latest" {
                unimplemented!();
            }

            let mut latest = None;
            for image in sysexts {
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
                        _ => panic!("Invalid version number"),
                    },
                };
            }
            match latest {
                None => {
                    error!("No image to enable for sysext: {}", name);
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
        info!("Enabling sysexts: {}", enable);

        for image in images {
            // Setup symlinks in /run/extensions.version for the sysexts found
            let original = format!("../../var/lib/extensions.d/{}", image.path());
            let link = format!("{}/{}.raw", run_extensions.display(), image.name);
            debug!("{} -> {}", link, original);
            match symlink_metadata(&link) {
                Ok(_) => remove_file(&link)?,
                _ => {}
            }
            symlink(original, link)?;
        }

        Ok(())
    }
}
