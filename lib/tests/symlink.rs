// SPDX-FileCopyrightText: Timothée Ravier <tim@siosm.fr>
// SPDX-License-Identifier: MIT

use std::fs::{read_link, remove_file};
use std::path::{Path, PathBuf};

fn cleanup(f: &Path, sysexts: &Vec<&'static str>) {
    for s in sysexts {
        let run_sysexts = f.join("run/extensions").join(format!("{s}.raw"));
        let _ = remove_file(&run_sysexts);
        assert!(!run_sysexts.exists());
    }
}

#[test]
fn basic_latest_version() {
    let f = Path::new("./test-data/basic_latest_version");

    let sysexts = vec!["foo", "test"];

    cleanup(f, &sysexts);

    let mut manager = sysexts_manager_lib::manager::new_with_root(f).unwrap();
    manager.load_config().unwrap();
    manager.load_images().unwrap();
    manager.enable_all().unwrap();

    for s in &sysexts {
        let run_sysexts = f.join("run/extensions").join(format!("{s}.raw"));
        assert!(run_sysexts.exists());
    }
    assert_eq!(
        read_link(f.join("run/extensions/foo.raw")).unwrap(),
        PathBuf::from("../../var/lib/extensions.d/foo-1.4-42-x86-64.raw")
    );
    assert_eq!(
        read_link(f.join("run/extensions/test.raw")).unwrap(),
        PathBuf::from("../../var/lib/extensions.d/test-20250330-42-x86-64.raw")
    );

    cleanup(f, &sysexts);
}

#[test]
fn basic_multiple_release() {
    let f = Path::new("./test-data/basic_multiple_release");

    let sysexts = vec!["foo", "test"];

    cleanup(f, &sysexts);

    let mut manager = sysexts_manager_lib::manager::new_with_root(f).unwrap();
    manager.load_config().unwrap();
    manager.load_images().unwrap();
    manager.enable_all().unwrap();

    for s in &sysexts {
        let run_sysexts = f.join("run/extensions").join(format!("{s}.raw"));
        assert!(run_sysexts.exists());
    }
    assert_eq!(
        read_link(f.join("run/extensions/foo.raw")).unwrap(),
        PathBuf::from("../../var/lib/extensions.d/foo-20250205-42-x86-64.raw")
    );
    assert_eq!(
        read_link(f.join("run/extensions/test.raw")).unwrap(),
        PathBuf::from("../../var/lib/extensions.d/test-20250330-42-x86-64.raw")
    );

    cleanup(f, &sysexts);
}

#[test]
fn no_valid_sysext() {
    let f = Path::new("./test-data/no_valid_sysext");

    let sysexts = vec!["foo", "test"];

    cleanup(f, &sysexts);

    let mut manager = sysexts_manager_lib::manager::new_with_root(f).unwrap();
    manager.load_config().unwrap();
    manager.load_images().unwrap();
    assert!(manager.enable_all().is_err());

    for s in &sysexts {
        let _run_sysexts = f.join("run/extensions").join(format!("{s}.raw"));
        // FIXME
        // assert!(!run_sysexts.exists());
    }

    cleanup(f, &sysexts);
}
