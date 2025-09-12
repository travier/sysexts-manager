// SPDX-FileCopyrightText: Timothée Ravier <tim@siosm.fr>
// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::process::Command;

use serde::Deserialize;

use anyhow::Context;
use anyhow::Result;

/// Representation of the rpm-ostree client-side state; this
/// can be parsed directly from the output of `rpm-ostree status --json`.
/// Currently not all fields are here, but that is a bug.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[allow(dead_code)]
pub struct Status {
    pub deployments: Vec<Deployment>,
}

/// A single deployment, i.e. a bootable ostree commit
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[allow(dead_code)]
pub struct Deployment {
    pub unlocked: Option<String>,
    pub osname: String,
    pub pinned: bool,
    pub checksum: String,
    pub base_checksum: Option<String>,
    pub base_commit_meta: HashMap<String, serde_json::Value>,
    pub staged: Option<bool>,
    pub booted: bool,
    pub serial: u32,
    pub origin: Option<String>,
    pub container_image_reference: Option<String>,
    pub version: Option<String>,
}

#[allow(dead_code)]
pub fn rpm_ostree_status() -> Result<()> {
    let mut cmd = Command::new("rpm-ostree");
    cmd.env("RPMOSTREE_CLIENT_ID", "manager");

    // Retry on temporary activation failures, see
    // https://github.com/coreos/rpm-ostree/issues/2531
    let pause = std::time::Duration::from_secs(1);
    let max_retries = 10;
    let mut retries = 0;
    let cmd_res = loop {
        retries += 1;
        let res = cmd.args(["status", "--json"]).output()?;

        if res.status.success() || retries >= max_retries {
            break res;
        }
        std::thread::sleep(pause);
    };

    if !cmd_res.status.success() {
        return Err(anyhow::Error::msg(format!(
            "running 'rpm-ostree status' failed: {}",
            String::from_utf8_lossy(&cmd_res.stderr)
        )));
    }

    // https://github.com/coreos/rpm-ostree/blob/main/rust/rpmostree-client/src/lib.rs
    let status: Status = serde_json::from_slice(&cmd_res.stdout)
        .context("failed to parse 'rpm-ostree status' output")?;

    for d in status.deployments {
        println!("Deployments: {}", d.version.clone().unwrap());
        // https://docs.rs/os-release/0.1.0/os_release/
    }

    Ok(())
}
