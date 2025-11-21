// SPDX-FileCopyrightText: Timothée Ravier <tim@siosm.fr>
// SPDX-License-Identifier: MIT

use std::fs::{read_link, remove_file};
use std::path::{Path, PathBuf};

fn cleanup(root: &Path, sysexts: &Vec<&'static str>) {
    for s in sysexts {
        let run_sysexts = root.join("run/extensions").join(format!("{s}.raw"));
        let _ = remove_file(&run_sysexts);
        assert!(!run_sysexts.exists());
    }
}

fn enable_all(root: &Path) {
    let mut manager = sysexts_manager_lib::manager::new_with_root(root).unwrap();
    manager.load_config().unwrap();
    manager.load_images().unwrap();
    manager.enable_all().unwrap();
}

fn enable_all_err(root: &Path) {
    let mut manager = sysexts_manager_lib::manager::new_with_root(root).unwrap();
    manager.load_config().unwrap();
    manager.load_images().unwrap();
    assert!(manager.enable_all().is_err());
}

fn validate_symlink(root: &Path, name: &str, dest: &str) {
    let run_sysexts = root.join("run/extensions").join(format!("{name}.raw"));
    assert!(run_sysexts.exists());
    assert_eq!(
        read_link(run_sysexts).unwrap(),
        PathBuf::from(format!("../../var/lib/extensions.d/{dest}.raw"))
    );
}

fn validate_no_symlink(root: &Path, name: &str) {
    let run_sysexts = root.join("run/extensions").join(format!("{name}.raw"));
    assert!(!run_sysexts.exists());
}

#[test]
fn valid_version_latest() {
    let root = Path::new("./test-data/valid_version_latest");
    let sysexts = vec!["foo", "bar", "duck"];
    cleanup(root, &sysexts);
    enable_all(root);
    validate_symlink(root, "foo", "foo-3-43-x86-64");
    validate_symlink(root, "bar", "bar-20251120-43-x86-64");
    validate_symlink(root, "duck", "duck-1.6.5-43-x86-64");
    cleanup(root, &sysexts);
}

#[test]
fn valid_current_release() {
    let root = Path::new("./test-data/valid_current_release");
    let sysexts = vec!["foo", "bar", "duck"];
    cleanup(root, &sysexts);
    enable_all(root);
    validate_symlink(root, "foo", "foo-3-43-x86-64");
    validate_symlink(root, "bar", "bar-20251120-43-x86-64");
    validate_symlink(root, "duck", "duck-1.6.5-43-x86-64");
    cleanup(root, &sysexts);
}

#[test]
fn valid_current_arch() {
    let root = Path::new("./test-data/valid_current_arch");
    let sysexts = vec!["foo", "bar", "duck"];
    cleanup(root, &sysexts);
    enable_all(root);
    validate_symlink(root, "foo", "foo-3-43-x86-64");
    validate_symlink(root, "bar", "bar-20251120-43-x86-64");
    validate_symlink(root, "duck", "duck-1.6.5-43-x86-64");
    cleanup(root, &sysexts);
}

#[test]
fn invalid_arch() {
    let root = Path::new("./test-data/invalid_arch");
    let sysexts = vec!["foo", "bar", "duck"];
    cleanup(root, &sysexts);
    enable_all_err(root);
    validate_no_symlink(root, "foo");
    validate_no_symlink(root, "bar");
    validate_no_symlink(root, "duck");
    cleanup(root, &sysexts);
}

#[test]
fn invalid_release() {
    let root = Path::new("./test-data/invalid_arch");
    let sysexts = vec!["foo", "bar", "duck"];
    cleanup(root, &sysexts);
    enable_all_err(root);
    validate_no_symlink(root, "foo");
    validate_no_symlink(root, "bar");
    validate_no_symlink(root, "duck");
    cleanup(root, &sysexts);
}
