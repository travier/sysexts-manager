// SPDX-FileCopyrightText: Timothée Ravier <tim@siosm.fr>
// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::env::consts::ARCH;
use std::fs::{self, File, remove_file, rename, symlink_metadata};
use std::io::Write;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use log::{debug, error, info, warn};
use os_release::OsRelease;
// use cap_std::fs::Dir;
use rayon::prelude::*;
use version_compare::{Cmp, compare};

use super::arch::Architecture;
use super::config::Config;
use super::image::Image;
use super::sha256writer::Sha256Writer;

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

const DEFAULT_CONFIG_DIR: &str = "etc/sysexts-manager";
const MUTABLE_CONFIG_DIRS: &[&str] = &["run/sysexts-manager", "etc/sysexts-manager"];
const ALL_CONFIG_DIRS: &[&str] = &[
    "run/sysexts-manager",
    "etc/sysexts-manager",
    "usr/lib/sysexts-manager",
];

const RUNTIME_EXTENSIONS_DIR: &str = "run/extensions";
// const PERMANENT_EXTENSIONS_DIR: &str = "var/lib/extensions";
const ALL_EXTENSIONS_DIRS: &[&str] = &["run/extensions", "etc/extensions", "var/lib/extensions"];

const DEFAULT_STORE: &str = "var/lib/extensions.d";

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
        for dir in ALL_CONFIG_DIRS {
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
                if filename_str.ends_with(".tmp") {
                    debug!("Cleaning up temporary file: {filename_str}");
                    remove_file(self.rootdir.join(DEFAULT_STORE).join(filename_osstr))?;
                    continue;
                }
                if filename_str.starts_with(name) {
                    found = true;
                    let Ok(image) = Image::new(name, filename.file_name(), None) else {
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
        if self.images.is_empty() {
            info!("No sysext loaded");
        } else {
            info!(
                "Loaded versions for sysexts: {}",
                self.images
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
                    .collect::<Vec<String>>()
                    .join(", ")
            )
        }
        Ok(())
    }

    pub fn enable_all(&self) -> Result<()> {
        let run_extensions = self.rootdir.join(RUNTIME_EXTENSIONS_DIR);
        if !run_extensions.exists() {
            debug!("Creating {}", &run_extensions.display());
            fs::create_dir(&run_extensions)?;
        }
        if !fs::metadata(&run_extensions)?.is_dir() {
            return Err(anyhow!("{} is not a directory", &run_extensions.display()));
        }
        for name in self.configs.keys() {
            self.enable_one(name, &run_extensions)?;
        }
        Ok(())
    }

    pub fn enable(&self, name: &String) -> Result<()> {
        let run_extensions = self.rootdir.join(RUNTIME_EXTENSIONS_DIR);
        if !run_extensions.exists() {
            debug!("Creating {}", &run_extensions.display());
            fs::create_dir(&run_extensions)?;
        }
        if !fs::metadata(&run_extensions)?.is_dir() {
            return Err(anyhow!("{} is not a directory", &run_extensions.display()));
        }
        self.enable_one(name, &run_extensions)
    }

    /// Enable a sysext: create a symlink in /run/extensions that points to the
    /// latest image matching the Kind policy (only latest implemented for now)
    fn enable_one(&self, name: &String, dir: &Path) -> Result<()> {
        let config = self
            .configs
            .get(name)
            .context(format!("No config found for: {name}"))?;

        if config.Kind != "latest" {
            unimplemented!();
        }

        let images = self.images.get(name).context(format!(
            "Found config but no images for: {name}. Not setting up. Update first."
        ))?;

        let image = self
            .find_latest_image(name, images)?
            .context(format!("No image to enable for sysext: {name}"))?;

        info!("Enabling sysext: {} ({})", image.name, image.version);

        let original = format!("../../var/lib/extensions.d/{}", image.path());
        let link = dir.join(format!("{name}.raw"));
        debug!("{} -> {original}", link.display());
        if symlink_metadata(&link).is_ok() {
            remove_file(&link)?
        };
        if link.exists() {
            return Err(anyhow!(
                "Not overriding an existing file for: {}",
                link.display()
            ));
        };
        symlink(original, link)?;
        println!("Enabled sysext: {name}");
        Ok(())
    }

    pub fn disable_all(&self) -> Result<()> {
        let run_extensions = self.rootdir.join(RUNTIME_EXTENSIONS_DIR);
        for name in self.configs.keys() {
            self.disable_one(name, &run_extensions)?;
        }
        Ok(())
    }

    pub fn disable(&self, name: &String) -> Result<()> {
        let run_extensions = self.rootdir.join(RUNTIME_EXTENSIONS_DIR);
        self.disable_one(name, &run_extensions)
    }

    pub fn disable_one(&self, name: &String, dir: &Path) -> Result<()> {
        let sysext = dir.join(format!("{name}.raw"));
        if !sysext.exists() {
            debug!("sysext already disabled: {}", &sysext.display());
            return Ok(());
        }
        remove_file(&sysext)?;
        println!("Disabled sysext: {name}");
        Ok(())
    }

    fn find_latest_image(
        &self,
        name: &String,
        sysext_images: &Vec<Image>,
    ) -> Result<Option<Image>> {
        let mut latest = None;
        for image in sysext_images {
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
        Ok(latest)
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

        println!("Added configuration for sysext: {name} ({url})");

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

        for dir in ALL_EXTENSIONS_DIRS {
            let symlink = self.rootdir.join(dir).join(format!("{name}.raw"));
            if symlink.exists() {
                info!("Found symlink: {}", symlink.display());
                return Err(anyhow!("Not removing currently enabled sysext: {name}"));
            }
        }

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
                self.images.remove(name);
            }
        };

        // Try removing the config from /run and /etc, ignore /usr
        for dir in MUTABLE_CONFIG_DIRS {
            let conf = self.rootdir.join(dir).join(format!("{name}.conf"));
            if conf.exists() {
                info!("Removing sysext config: {}", conf.display());
                remove_file(&conf)?;
            }
        }
        self.configs.remove(name);

        println!("Removed configuration and images for sysext: {name}");

        Ok(())
    }

    fn update_sysext(&self, config: &Config, images: &Vec<Image>) -> Result<()> {
        debug!(
            "Downloading SHA256SUMS for: {} (version_id: {}, arch: {})",
            config.Name, self.system.version_id, self.system.arch
        );
        let sha256sum_url = format!("{}/{}/SHA256SUMS", config.Url, config.Name);
        debug!("Downloading: {sha256sum_url}");
        let client = reqwest::blocking::Client::new();
        let sha256sums = client.get(sha256sum_url).send()?.text()?;
        debug!("{sha256sums}");

        // Parse images and hashes list from SHA256SUM file
        let mut remote_images: Vec<Image> = Vec::new();
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
            let Ok(image) = Image::new(&config.Name, filename.clone().into(), Some(hash)) else {
                warn!("Ignoring invalid sysext: {filename}");
                continue;
            };
            remote_images.push(image);
        }
        let parsed_sha256sums = remote_images
            .iter()
            .map(|i| format!("{} ({})", i.path(), i.hash.clone().unwrap_or("?".into())))
            .collect::<Vec<String>>()
            .join("\n");
        if parsed_sha256sums.is_empty() {
            warn!("Empty SHA256SUMS file for: {}", config.Name);
            return Ok(());
        }
        debug!("Found potential sysexts:\n{parsed_sha256sums}");

        // Search latest image from SHA256SUM list that matches arch & version_id
        let remote_image = match self.find_latest_image(&config.Name, &remote_images)? {
            None => {
                error!("No remote valid image found for sysext: {}", config.Name);
                return Ok(());
            }
            Some(i) => {
                debug!(
                    "Found latest remote image for sysext: {} {}",
                    i.name, i.version
                );
                i
            }
        };

        // Compare it with the latest image installed locally, matching arch & version_id
        let download_image = match self.find_latest_image(&config.Name, images)? {
            None => {
                info!(
                    "No local image for sysext: {}. Downloading: {}",
                    config.Name, remote_image.version
                );
                remote_image
            }
            Some(img) => {
                debug!(
                    "Comparing latest local & remote image for sysext '{}': local: '{}' remote: '{}'",
                    img.name, img.version, remote_image.version
                );
                match compare(&img.version, &remote_image.version) {
                    Ok(Cmp::Lt) => {
                        info!("Found update to download: {}", remote_image.version);
                        remote_image
                    }
                    Ok(Cmp::Eq) => {
                        println!("No update found for '{}'", img.name);
                        // TODO: Compute local image SHA256SUM, compare it and warn if different
                        return Ok(());
                    }
                    Ok(Cmp::Gt) => {
                        warn!("Local image is newer for '{}': {}", img.name, img.version);
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
        let image_url = format!("{}/{}/{}", config.Url, config.Name, download_image.path());
        debug!("Downloading: {image_url}");
        let mut response = client.get(image_url).send()?;

        // Setup a temporary file
        let sysext_store = self.rootdir.join(DEFAULT_STORE);
        let sysext_tmp = sysext_store.join(format!("{}.tmp", download_image.path()));
        let file = File::create(&sysext_tmp)?;

        // Create a writer to compute sha256sum hash as we download the image to the temporary file
        let mut writer = Sha256Writer::new(file);
        response.copy_to(&mut writer)?;

        let digest = writer.digest();
        if digest != download_image.hash.clone().unwrap_or("?".into()) {
            debug!("Invalid hash, removing file: {}", sysext_tmp.display());
            remove_file(sysext_tmp)?;
            return Err(anyhow!(
                "Invalid hash for {}: got {} vs expected {}",
                download_image.path(),
                digest,
                download_image.hash.unwrap_or("?".into())
            ));
        }
        debug!(
            "Valid hash for {} {}",
            download_image.path(),
            download_image.hash.clone().unwrap_or("?".into())
        );

        // Rename to final name
        let sysext_final = sysext_store.join(download_image.path());
        debug!(
            "Renaming: {} -> {}",
            sysext_tmp.display(),
            sysext_final.display()
        );
        rename(sysext_tmp, &sysext_final)?;
        println!("Successfully updated sysext: {}", config.Name);

        // TODO: Add image to manager

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
        self.configs.clone().into_par_iter().for_each(|(n, c)| {
            let images = self.images.get(&n).unwrap_or(&empty);
            self.update_sysext(&c, images).unwrap_or_else(|e| {
                error!("Failed to update sysext: {n}: {e}");
            });
        });

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
