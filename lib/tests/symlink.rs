use std::fs;
use std::path::Path;

#[test]
fn test_symlink_test1() {
    let f = Path::new("../test-data/test1");

    let _ = fs::remove_file(f.join("run/extensions/foo.raw"));
    let _ = fs::remove_file(f.join("run/extensions/test.raw"));
    assert!(!f.join("run/extensions/foo.raw").exists());
    assert!(!f.join("run/extensions/test.raw").exists());

    let mut manager = sysexts_manager_lib::manager::new_with_root(f).unwrap();
    manager.load_config().unwrap();
    manager.load_images().unwrap();
    manager.enable().unwrap();

    assert!(f.join("run/extensions/foo.raw").exists());
    assert!(f.join("run/extensions/test.raw").exists());
}

#[test]
fn test_symlink_test2() {
    let f = Path::new("../test-data/test2");

    let _ = fs::remove_file(f.join("run/extensions/foo.raw"));
    let _ = fs::remove_file(f.join("run/extensions/test.raw"));
    assert!(!f.join("run/extensions/foo.raw").exists());
    assert!(!f.join("run/extensions/test.raw").exists());

    let mut manager = sysexts_manager_lib::manager::new_with_root(f).unwrap();
    manager.load_config().unwrap();
    manager.load_images().unwrap();
    manager.enable().unwrap();

    assert!(f.join("run/extensions/test.raw").exists());
}
