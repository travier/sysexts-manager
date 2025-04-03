use std::path::Path;
use std::{ffi::OsString, fs};

use anyhow::{Result, anyhow};
use serde::Deserialize;
use toml;
use version_compare::Version;

use super::manager::Architecture;

#[derive(Deserialize, Debug)]
#[allow(non_snake_case, dead_code)]
pub struct Config {
    pub Name: String,
    pub Kind: String,
    pub Url: String,
}

impl Config {
    pub fn new(path: &Path) -> Result<Config> {
        let Ok(f) = fs::read_to_string(path) else {
            return Err(anyhow!(
                "Could not read content from file: {}",
                path.display()
            ));
        };
        let Ok(c) = toml::from_str::<Config>(&f) else {
            return Err(anyhow!("Invalid config in file: {}", path.display()));
        };
        Ok(c)
    }
}

#[derive(Debug, Clone)]
pub struct Image {
    pub name: String,
    pub architecture: Architecture,
    pub version_id: String,
    pub version: String,
}

impl Image {
    pub fn new(name: &str, f: OsString) -> Result<Image> {
        let Some(filename) = f.to_str() else {
            return Err(anyhow!("Failed to parse sysext image name: {:?}", name));
        };
        let mut filename = filename.to_string();

        if !filename.ends_with(".raw") {
            return Err(anyhow!(
                "sysext image name must end with the `.raw` extension: {}",
                filename
            ));
        }
        for _ in 0..".raw".len() {
            filename.pop();
        }

        let mut arch = Architecture::x86_64;
        if filename.ends_with("-x86-64") {
            for _ in 0.."-x86-64".len() {
                filename.pop();
            }
        } else if filename.ends_with("-aarch64") {
            for _ in 0.."-aarch64".len() {
                filename.pop();
            }
            arch = Architecture::aarch64;
        } else {
            return Err(anyhow!(
                "sysext image name must be either for x86-64 or aarch64: {}",
                filename
            ));
        }

        let mut split: Vec<&str> = filename.split("-").collect();
        if split.len() < 3 {
            return Err(anyhow!(
                "sysext image name must have a name, version and version id: {}",
                filename
            ));
        }

        let version_id = split.pop().unwrap();

        let mut version = split.join("-");
        for _ in 0..name.len() {
            version.remove(0);
        }
        // Remove the '-'
        version.remove(0);

        if Version::from(&version).is_none() {
            return Err(anyhow!(
                "could not parse version '{}': {}",
                version,
                filename
            ));
        }

        Ok(Image {
            name: name.into(),
            architecture: arch,
            version_id: version_id.into(),
            version,
        })
    }

    pub fn path(&self) -> String {
        let arch = match self.architecture {
            Architecture::x86_64 => "x86-64".to_string(),
            Architecture::aarch64 => "aarch64".to_string(),
        };
        format!(
            "{}-{}-{}-{}.raw",
            self.name, self.version, self.version_id, arch
        )
    }
}
