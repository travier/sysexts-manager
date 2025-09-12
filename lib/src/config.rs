// SPDX-FileCopyrightText: Timothée Ravier <tim@siosm.fr>
// SPDX-License-Identifier: MIT

use std::fs;
use std::path::Path;

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use toml;

#[derive(Deserialize, Serialize, Debug, Clone)]
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
