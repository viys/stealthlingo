use std::fs;
use std::io::{Cursor, Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::time::Duration;

use chrono::{TimeDelta, Utc};
use sha2::{Digest, Sha256};
use stealthlingo::commands::{update as update_cmd, Context};
use stealthlingo::config::Paths;
use stealthlingo::update::{self, Receipt};

const TIMEOUT: Duration = Duration::from_secs(5);
const OFFLINE: &str = "http://127.0.0.1:9/releases";

fn exe(name: &str) -> String {
    format!("{name}{}", std::env::consts::EXE_SUFFIX)
}

struct Route {
    prefix: String,
    status: u16,
    headers: String,
    body: Vec<u8>,
}

fn route(prefix: impl Into<String>, body: Vec<u8>) -> Route {
    Route {
        prefix: prefix.into(),
        status: 200,
        headers: String::new(),
        body,
    }
}

/// Answers every request on a local port with the first route whose path
/// prefix matches (404 otherwise). Returns the base URL.
fn serve(routes: Vec<Route>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut request = Vec::new();
            let mut buf = [0; 4096];
            while !request.windows(4).any(|w| w == b"\r\n\r\n") {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => request.extend_from_slice(&buf[..n]),
                }
            }
            let request = String::from_utf8_lossy(&request);
            let path = request.split_whitespace().nth(1).unwrap_or("").to_string();
            let found = routes.iter().find(|r| path.starts_with(r.prefix.as_str()));
            let (status, headers, body) = found.map_or((404, "", &[][..]), |r| {
                (r.status, r.headers.as_str(), &r.body[..])
            });
            let _ = write!(
                stream,
                "HTTP/1.1 {status} X\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(body);
        }
    });
    base
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A Windows-style release archive: programs at the top level.
fn zip_archive(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (name, data) in files {
        zip.start_file(*name, options).unwrap();
        zip.write_all(data).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

/// A Unix-style release archive: everything inside one folder.
fn tar_xz_archive(folder: &str, files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut tar = tar::Builder::new(Vec::new());
    for (name, data) in files {
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        tar.append_data(&mut header, format!("{folder}/{name}"), *data)
            .unwrap();
    }
    let tar = tar.into_inner().unwrap();
    let mut xz = Vec::new();
    lzma_rs::xz_compress(&mut Cursor::new(tar), &mut xz).unwrap();
    xz
}

/// One platform's archive in a published release.
struct Published {
    target: &'static str,
    extension: &'static str,
    archive: Vec<u8>,
    /// What the manifest lists as its SHA-256; `None` leaves it out.
    sha256: Option<String>,
}

impl Published {
    fn new(target: &'static str, extension: &'static str, archive: Vec<u8>) -> Self {
        let sha256 = Some(sha256_hex(&archive));
        Self {
            target,
            extension,
            archive,
            sha256,
        }
    }
}

/// Serves a release the way GitHub does: `releases/latest` redirects to the
/// tag, and the downloads include dist's manifest. Returns the releases URL.
fn publish(version: &str, archives: Vec<Published>) -> String {
    let tag = format!("v{version}");
    let mut artifacts = serde_json::Map::new();
    let mut routes = vec![Route {
        prefix: "/releases/latest".to_string(),
        status: 302,
        headers: format!("Location: /viys/stealthlingo/releases/tag/{tag}\r\n"),
        body: Vec::new(),
    }];
    for published in archives {
        let name = format!("stealthlingo-{}.{}", published.target, published.extension);
        let mut artifact = serde_json::json!({
            "name": name,
            "kind": "executable-zip",
            "target_triples": [published.target],
        });
        if let Some(sha256) = published.sha256 {
            artifact["checksums"] = serde_json::json!({ "sha256": sha256 });
        }
        artifacts.insert(name.clone(), artifact);
        routes.push(route(
            format!("/releases/download/{tag}/{name}"),
            published.archive,
        ));
    }
    let manifest = serde_json::json!({ "announcement_tag": tag, "artifacts": artifacts });
    routes.push(route(
        format!("/releases/download/{tag}/dist-manifest.json"),
        manifest.to_string().into_bytes(),
    ));
    format!("{}/releases", serve(routes))
}

fn new_programs() -> Vec<(String, &'static [u8])> {
    vec![
        (exe("stealthlingo"), b"new main".as_slice()),
        (exe("link-stealthlingo"), b"new link".as_slice()),
        ("README.md".to_string(), b"readme".as_slice()),
    ]
}

fn as_refs<'a>(files: &'a [(String, &'static [u8])]) -> Vec<(&'a str, &'static [u8])> {
    files.iter().map(|(n, d)| (n.as_str(), *d)).collect()
}

const WINDOWS: &str = "x86_64-pc-windows-msvc";
const LINUX: &str = "x86_64-unknown-linux-gnu";

fn windows_zip() -> Published {
    Published::new(WINDOWS, "zip", zip_archive(&as_refs(&new_programs())))
}

fn linux_tar_xz() -> Published {
    let archive = tar_xz_archive(&format!("stealthlingo-{LINUX}"), &as_refs(&new_programs()));
    Published::new(LINUX, "tar.xz", archive)
}

/// The archive for the platform the tests run on.
fn this_platform() -> Option<Published> {
    let target = update::target()?;
    let files = new_programs();
    Some(if target.contains("windows") {
        Published::new(target, "zip", zip_archive(&as_refs(&files)))
    } else {
        let archive = tar_xz_archive(&format!("stealthlingo-{target}"), &as_refs(&files));
        Published::new(target, "tar.xz", archive)
    })
}

fn install_dir(dir: &Path) {
    fs::write(dir.join(exe("stealthlingo")), "old main").unwrap();
    fs::write(dir.join(exe("link-stealthlingo")), "old link").unwrap();
}

fn contents(dir: &Path, name: &str) -> String {
    fs::read_to_string(dir.join(exe(name))).unwrap()
}

fn leftovers(dir: &Path) -> Vec<String> {
    fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".new") || n.ends_with(".old"))
        .collect()
}

/// Finds and installs the latest release for `target` into `dir`.
fn install_latest(releases: &str, target: &str, dir: &Path) -> anyhow::Result<Vec<String>> {
    let release = update::latest_release(releases, TIMEOUT)?;
    let archive = update::find_download(releases, &release, target, TIMEOUT)?;
    update::install(&archive, dir, TIMEOUT)
}

#[test]
fn the_latest_release_is_read_from_the_redirect() {
    let releases = publish("99.1.0", vec![windows_zip()]);
    let release = update::latest_release(&releases, TIMEOUT).unwrap();
    assert_eq!(release.version.to_string(), "99.1.0");
    assert_eq!(release.tag, "v99.1.0");

    let no_redirect = format!(
        "{}/releases",
        serve(vec![route("/releases/latest", vec![])])
    );
    let err = update::latest_release(&no_redirect, TIMEOUT).unwrap_err();
    assert!(format!("{err:#}").contains("without naming"), "{err:#}");
}

#[test]
fn zip_releases_replace_both_programs() {
    let dir = tempfile::tempdir().unwrap();
    install_dir(dir.path());
    let releases = publish("99.0.0", vec![windows_zip(), linux_tar_xz()]);

    let replaced = install_latest(&releases, WINDOWS, dir.path()).unwrap();
    assert_eq!(
        replaced,
        vec![exe("link-stealthlingo"), exe("stealthlingo")]
    );
    assert_eq!(contents(dir.path(), "stealthlingo"), "new main");
    assert_eq!(contents(dir.path(), "link-stealthlingo"), "new link");
    assert!(!dir.path().join("README.md").exists());
    assert!(
        leftovers(dir.path()).is_empty(),
        "{:?}",
        leftovers(dir.path())
    );
}

#[test]
fn tar_xz_releases_are_unpacked_from_their_folder() {
    let dir = tempfile::tempdir().unwrap();
    install_dir(dir.path());
    let releases = publish("99.0.0", vec![windows_zip(), linux_tar_xz()]);

    install_latest(&releases, LINUX, dir.path()).unwrap();
    assert_eq!(contents(dir.path(), "stealthlingo"), "new main");
    assert_eq!(contents(dir.path(), "link-stealthlingo"), "new link");
    assert!(leftovers(dir.path()).is_empty());
}

#[test]
fn a_bad_checksum_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    install_dir(dir.path());
    let mut published = windows_zip();
    published.sha256 = Some(sha256_hex(b"something else"));
    let releases = publish("99.0.0", vec![published]);

    let err = install_latest(&releases, WINDOWS, dir.path()).unwrap_err();
    assert!(format!("{err:#}").contains("checksum"), "{err:#}");
    assert_eq!(contents(dir.path(), "stealthlingo"), "old main");
    assert_eq!(contents(dir.path(), "link-stealthlingo"), "old link");
    assert!(leftovers(dir.path()).is_empty());
}

#[test]
fn releases_without_a_checksum_the_program_or_this_platform_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    install_dir(dir.path());

    let mut unchecked = windows_zip();
    unchecked.sha256 = None;
    let releases = publish("99.0.0", vec![unchecked]);
    let err = install_latest(&releases, WINDOWS, dir.path()).unwrap_err();
    assert!(format!("{err:#}").contains("no checksum"), "{err:#}");

    let readme_only = Published::new(WINDOWS, "zip", zip_archive(&[("README.md", b"readme")]));
    let releases = publish("99.0.0", vec![readme_only]);
    let err = install_latest(&releases, WINDOWS, dir.path()).unwrap_err();
    assert!(format!("{err:#}").contains("does not contain"), "{err:#}");

    let err = install_latest(&releases, "riscv64gc-unknown-linux-gnu", dir.path()).unwrap_err();
    assert!(format!("{err:#}").contains("no download"), "{err:#}");
    assert_eq!(contents(dir.path(), "stealthlingo"), "old main");
}

fn context(dir: &Path, releases_url: &str) -> Context {
    let mut ctx = Context::load_from(Paths::at(dir.join("data"))).unwrap();
    ctx.releases_url = releases_url.to_string();
    ctx.config.http_timeout_secs = 5;
    ctx
}

fn receipt(path: &Path, prefix: &Path) {
    let json = serde_json::json!({
        "binaries": [exe("link-stealthlingo"), exe("stealthlingo")],
        "install_layout": "cargo-home",
        "install_prefix": prefix.to_string_lossy(),
        "version": "0.1.5",
    });
    fs::write(path, json.to_string()).unwrap();
}

#[test]
fn update_installs_over_a_scripted_installation_and_updates_its_receipt() {
    let Some(published) = this_platform() else {
        return;
    };
    let releases = publish("99.0.0", vec![published]);
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("bin");
    fs::create_dir(&bin).unwrap();
    install_dir(&bin);
    let receipt_path = dir.path().join("receipt.json");
    receipt(&receipt_path, dir.path());
    let ctx = context(dir.path(), &releases);

    update_cmd::run_in(&ctx, false, &bin, Some(&receipt_path)).unwrap();
    assert_eq!(contents(&bin, "stealthlingo"), "new main");
    assert_eq!(contents(&bin, "link-stealthlingo"), "new link");
    let saved: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&receipt_path).unwrap()).unwrap();
    assert_eq!(saved["version"], "99.0.0");
    assert_eq!(saved["install_layout"], "cargo-home");
}

#[test]
fn update_leaves_other_installations_alone() {
    let Some(published) = this_platform() else {
        return;
    };
    let releases = publish("99.0.0", vec![published]);
    let dir = tempfile::tempdir().unwrap();
    install_dir(dir.path());
    let elsewhere = tempfile::tempdir().unwrap();
    let receipt_path = dir.path().join("receipt.json");
    receipt(&receipt_path, elsewhere.path());
    let ctx = context(dir.path(), &releases);

    update_cmd::run_in(&ctx, false, dir.path(), Some(&receipt_path)).unwrap();
    update_cmd::run_in(&ctx, false, dir.path(), None).unwrap();
    assert_eq!(contents(dir.path(), "stealthlingo"), "old main");
    assert!(Receipt::for_dir(&receipt_path, elsewhere.path()).is_some());
}

#[test]
fn update_check_only_reports_and_remembers_the_release() {
    let Some(published) = this_platform() else {
        return;
    };
    let releases = publish("99.0.0", vec![published]);
    let dir = tempfile::tempdir().unwrap();
    install_dir(dir.path());
    let receipt_path = dir.path().join("receipt.json");
    receipt(&receipt_path, dir.path());
    let ctx = context(dir.path(), &releases);
    assert_eq!(update::known_newer(&ctx.paths), None);

    update_cmd::run_in(&ctx, true, dir.path(), Some(&receipt_path)).unwrap();
    assert_eq!(contents(dir.path(), "stealthlingo"), "old main");
    assert_eq!(
        update::known_newer(&ctx.paths).map(|v| v.to_string()),
        Some("99.0.0".to_string())
    );
    assert!(!update::check_due(&ctx.paths, Utc::now()));
}

#[test]
fn older_releases_are_not_installed() {
    let Some(published) = this_platform() else {
        return;
    };
    let releases = publish("0.0.1", vec![published]);
    let dir = tempfile::tempdir().unwrap();
    install_dir(dir.path());
    let receipt_path = dir.path().join("receipt.json");
    receipt(&receipt_path, dir.path());
    let ctx = context(dir.path(), &releases);

    update_cmd::run_in(&ctx, false, dir.path(), Some(&receipt_path)).unwrap();
    assert_eq!(contents(dir.path(), "stealthlingo"), "old main");
    assert_eq!(update::known_newer(&ctx.paths), None);
}

#[test]
fn update_explains_when_github_cannot_be_reached() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(dir.path(), OFFLINE);
    let err = update_cmd::run_in(&ctx, true, dir.path(), None).unwrap_err();
    assert!(
        format!("{err:#}").contains("check your connection"),
        "{err:#}"
    );
}

#[test]
fn the_daily_check_runs_once_a_day_and_reports_newer_releases() {
    let releases = publish("99.0.0", vec![windows_zip()]);
    let dir = tempfile::tempdir().unwrap();
    let mut ctx = context(dir.path(), &releases);

    ctx.config.check_updates = false;
    assert!(update_cmd::start_daily_check(&ctx).is_none());

    ctx.config.check_updates = true;
    let check = update_cmd::start_daily_check(&ctx).expect("a first check is due");
    let found = check.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(found.to_string(), "99.0.0");
    assert!(update_cmd::start_daily_check(&ctx).is_none());
    assert!(update::check_due(
        &ctx.paths,
        Utc::now() + TimeDelta::hours(25)
    ));
}

#[test]
fn an_unreachable_github_is_tried_again_only_the_next_day() {
    let dir = tempfile::tempdir().unwrap();
    let ctx = context(dir.path(), OFFLINE);
    let check = update_cmd::start_daily_check(&ctx).expect("a first check is due");
    assert!(check.recv_timeout(Duration::from_secs(10)).is_err());
    assert!(!update::check_due(&ctx.paths, Utc::now()));
    assert_eq!(update::known_newer(&ctx.paths), None);
}
