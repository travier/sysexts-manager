// SPDX-FileCopyrightText: Timothée Ravier <tim@siosm.fr>
// SPDX-License-Identifier: MIT

use std::ffi::OsString;

use anyhow::{Result, anyhow};
use version_compare::Version;

use super::arch::Architecture;

#[derive(Debug, Clone)]
pub struct Image {
    pub name: String,
    pub architecture: Architecture,
    pub version_id: String,
    pub version: String,
    pub hash: Option<String>,
}

impl Image {
    pub fn new(name: &str, f: OsString, h: Option<String>) -> Result<Image> {
        let Some(filename) = f.to_str() else {
            return Err(anyhow!("Failed to parse sysext image name: {:?}", name));
        };
        let mut filename = filename.to_string();

        if !filename.starts_with(name) {
            return Err(anyhow!(
                "sysext image name must start with its own name: {}",
                filename
            ));
        }

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
        } else if filename.ends_with("-arm64") {
            for _ in 0.."-arm64".len() {
                filename.pop();
            }
            arch = Architecture::aarch64;
        } else {
            return Err(anyhow!(
                "sysext image name must be either for x86-64 or arm64: {}",
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
            hash: h,
        })
    }

    pub fn path(&self) -> String {
        let arch = match self.architecture {
            Architecture::x86_64 => "x86-64".to_string(),
            Architecture::aarch64 => "arm64".to_string(),
        };
        format!(
            "{}-{}-{}-{}.raw",
            self.name, self.version, self.version_id, arch
        )
    }
}
