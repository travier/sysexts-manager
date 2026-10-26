// SPDX-FileCopyrightText: 2026 sysexts-manager contributors
// SPDX-License-Identifier: MIT

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Child, Command, Output};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use sysexts_manager_lib::sha256writer::Sha256Writer;
use tempfile::TempDir;

const BODY: &[u8] = b"test sysext image\n";

fn image_name(name: &str) -> String {
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x86-64",
        "aarch64" => "arm64",
        arch => panic!("Unsupported test architecture: {arch}"),
    };
    format!("{name}-1-44-{arch}.raw")
}

struct Root(TempDir);

impl Root {
    fn new() -> Self {
        let root = Self(tempfile::tempdir().unwrap());
        fs::create_dir_all(root.0.path().join("etc/sysexts-manager")).unwrap();
        fs::create_dir_all(root.store()).unwrap();
        fs::write(
            root.0.path().join("etc/os-release"),
            "NAME=Fedora\nID=fedora\nVERSION_ID=44\n",
        )
        .unwrap();
        root
    }

    fn store(&self) -> PathBuf {
        self.0.path().join("var/lib/extensions.d")
    }

    fn configure(&self, name: &str, url: &str) {
        fs::write(
            self.0
                .path()
                .join(format!("etc/sysexts-manager/{name}.conf")),
            format!("Name = \"{name}\"\nKind = \"latest\"\nUrl = \"{url}\"\n"),
        )
        .unwrap();
    }

    fn command(&self, command: &str) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_sysexts-manager"));
        cmd.args(["--root"])
            .arg(self.0.path())
            .args(["-j", "1", command])
            .env("NO_PROXY", "*");
        cmd
    }

    fn temporary_files(&self) -> Vec<PathBuf> {
        fs::read_dir(self.store())
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|ext| ext == "tmp"))
            .collect()
    }

    fn wait_for_downloads(&self, count: usize) {
        let deadline = Instant::now() + Duration::from_secs(10);
        while self.temporary_files().len() != count {
            assert!(Instant::now() < deadline, "Downloads did not start");
            thread::sleep(Duration::from_millis(10));
        }
    }
}

// Hold image bodies until the test has inspected or interfered with the download.
struct Server {
    url: String,
    release: Arc<(Mutex<bool>, Condvar)>,
    stopped: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Server {
    fn new(names: &[&str], paused: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let release = Arc::new((Mutex::new(!paused), Condvar::new()));
        let stopped = Arc::new(AtomicBool::new(false));
        let mut writer = Sha256Writer::new(Vec::new());
        writer.write_all(BODY).unwrap();
        let hash = writer.digest();
        let mut routes = HashMap::new();
        for name in names {
            let filename = image_name(name);
            let checksum = if *name == "bad" {
                "0".repeat(64)
            } else {
                hash.clone()
            };
            routes.insert(
                format!("/{name}/SHA256SUMS"),
                format!("{checksum}  {filename}\n").into_bytes(),
            );
            routes.insert(format!("/{name}/{filename}"), BODY.to_vec());
        }
        let routes = Arc::new(routes);
        let worker_release = release.clone();
        let worker_stopped = stopped.clone();
        let thread = thread::spawn(move || {
            let mut workers = Vec::new();
            while !worker_stopped.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let routes = routes.clone();
                        let release = worker_release.clone();
                        workers.push(thread::spawn(move || serve(stream, &routes, &release)));
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("Failed to accept request: {error}"),
                }
            }
            for worker in workers {
                worker.join().unwrap();
            }
        });
        Self {
            url,
            release,
            stopped,
            thread: Some(thread),
        }
    }

    fn resume(&self) {
        *self.release.0.lock().unwrap() = true;
        self.release.1.notify_all();
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.resume();
        self.stopped.store(true, Ordering::Relaxed);
        self.thread.take().unwrap().join().unwrap();
    }
}

fn serve(
    mut stream: TcpStream,
    routes: &HashMap<String, Vec<u8>>,
    release: &(Mutex<bool>, Condvar),
) {
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    let mut reader = BufReader::new(&stream);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    let path = line.split_whitespace().nth(1).unwrap().to_owned();
    loop {
        line.clear();
        assert_ne!(
            reader.read_line(&mut line).unwrap(),
            0,
            "Incomplete request"
        );
        if line == "\r\n" {
            break;
        }
    }
    let body = routes.get(&path).expect("Unexpected request");
    let truncated = path.starts_with("/truncated/") && path.ends_with(".raw");
    let length = body.len() + usize::from(truncated);
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    stream.flush().unwrap();
    if path.ends_with(".raw") {
        let (ready, timeout) = release
            .1
            .wait_timeout_while(
                release.0.lock().unwrap(),
                Duration::from_secs(15),
                |ready| !*ready,
            )
            .unwrap();
        assert!(
            *ready && !timeout.timed_out(),
            "Download was never released"
        );
    }
    stream.write_all(body).unwrap();
}

fn assert_success(output: Output) {
    assert!(output.status.success(), "{output:?}");
}

// Ensure a failed assertion cannot leave an updater waiting on the test server.
struct Update(Child);

impl Update {
    fn start(root: &Root) -> Self {
        Self(
            root.command("update")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap(),
        )
    }

    fn finish(mut self) {
        assert!(self.0.wait().unwrap().success());
    }
}

impl Drop for Update {
    fn drop(&mut self) {
        if self.0.try_wait().unwrap().is_none() {
            self.0.kill().unwrap();
            self.0.wait().unwrap();
        }
    }
}

#[test]
fn status_preserves_temporary_files_with_multiple_configs() {
    let root = Root::new();
    root.configure("foo", "http://unused.invalid");
    root.configure("bar", "http://unused.invalid");
    let temporary = root.store().join(format!("{}.tmp", image_name("foo")));
    fs::write(&temporary, BODY).unwrap();
    assert_success(root.command("status").output().unwrap());
    assert_eq!(fs::read(temporary).unwrap(), BODY);
}

#[test]
fn status_does_not_interrupt_download() {
    let server = Server::new(&["foo", "bar"], true);
    let root = Root::new();
    root.configure("foo", &server.url);
    root.configure("bar", &server.url);
    let update = Update::start(&root);
    root.wait_for_downloads(1);
    assert_success(root.command("status").output().unwrap());
    assert_eq!(root.temporary_files().len(), 1);
    server.resume();
    update.finish();
    assert_eq!(
        fs::read(root.store().join(image_name("foo"))).unwrap(),
        BODY
    );
    assert_eq!(
        fs::read(root.store().join(image_name("bar"))).unwrap(),
        BODY
    );
    let reference = root.0.path().join("permissions-reference");
    fs::write(&reference, BODY).unwrap();
    assert_eq!(
        fs::metadata(root.store().join(image_name("foo")))
            .unwrap()
            .permissions()
            .mode(),
        fs::metadata(reference).unwrap().permissions().mode()
    );
    assert!(root.temporary_files().is_empty());
}

#[test]
fn concurrent_updates_use_distinct_temporary_files() {
    let server = Server::new(&["foo"], true);
    let root = Root::new();
    root.configure("foo", &server.url);
    let first = Update::start(&root);
    root.wait_for_downloads(1);
    let second = Update::start(&root);
    root.wait_for_downloads(2);
    server.resume();
    first.finish();
    second.finish();
    assert_eq!(
        fs::read(root.store().join(image_name("foo"))).unwrap(),
        BODY
    );
    assert!(root.temporary_files().is_empty());
}

#[test]
fn failed_updates_return_error_and_allow_other_updates_to_finish() {
    let server = Server::new(&["good", "bad", "truncated"], false);
    let root = Root::new();
    for name in ["good", "bad", "truncated"] {
        root.configure(name, &server.url);
    }
    let output = root.command("update").output().unwrap();
    assert!(!output.status.success(), "{output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("bad: Invalid hash"), "{stderr}");
    assert!(stderr.contains("truncated:"), "{stderr}");
    assert!(
        !String::from_utf8(output.stdout)
            .unwrap()
            .contains("Successfully updated all sysexts")
    );
    assert_eq!(
        fs::read(root.store().join(image_name("good"))).unwrap(),
        BODY
    );
    assert!(!root.store().join(image_name("bad")).exists());
    assert!(!root.store().join(image_name("truncated")).exists());
    assert!(root.temporary_files().is_empty());
}

#[test]
fn rename_failure_includes_paths_and_cleans_up_temporary_file() {
    let server = Server::new(&["foo"], true);
    let root = Root::new();
    root.configure("foo", &server.url);
    let mut update = Update::start(&root);
    root.wait_for_downloads(1);
    let temporary = root.temporary_files().pop().unwrap();
    let destination = root.store().join(image_name("foo"));
    fs::create_dir(&destination).unwrap();
    server.resume();
    let stderr = update.0.stderr.take().unwrap();
    let output = std::io::read_to_string(stderr).unwrap();
    assert!(!update.0.wait().unwrap().success());
    assert!(
        output.contains(&format!(
            "Failed to rename {} to {}",
            temporary.display(),
            destination.display()
        )),
        "{output}"
    );
    assert!(destination.is_dir());
    assert!(root.temporary_files().is_empty());
}
