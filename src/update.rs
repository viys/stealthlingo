//! New releases on GitHub: a once-a-day check, and replacing the installed
//! binaries with the release archive for this platform.

use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::io::{Cursor, Read};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, TimeDelta, Utc};
use reqwest::blocking::Client;
use reqwest::header::LOCATION;
use reqwest::redirect::Policy;
use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::Paths;
use crate::dictionary::USER_AGENT;

/// GitHub releases page. Tests point the context at a local server instead.
/// The API at api.github.com is avoided: it allows only 60 requests an hour
/// per address, which people behind a shared address run out of.
pub const RELEASES_URL: &str = "https://github.com/viys/stealthlingo/releases";

/// Reinstalls with the script from the README.
pub const INSTALLER_COMMAND: &str = if cfg!(windows) {
    "powershell -ExecutionPolicy Bypass -c \"irm https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.ps1 | iex\""
} else {
    "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/viys/stealthlingo/releases/latest/download/stealthlingo-installer.sh | sh"
};

pub const CARGO_COMMAND: &str = "cargo install --git https://github.com/viys/stealthlingo --force";

const CHECK_INTERVAL: TimeDelta = TimeDelta::hours(24);

/// Time limit for the automatic check, which nobody is waiting for.
pub const BACKGROUND_TIMEOUT: Duration = Duration::from_secs(3);

/// Release archives are a few megabytes; allow for slow connections.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);

/// Programs shipped in a release archive, without the platform's extension.
/// The main program comes last, so a failure part way leaves it untouched.
const BINARIES: [&str; 2] = ["link-stealthlingo", "stealthlingo"];

/// The version of this build.
pub fn current() -> Version {
    Version::parse(env!("CARGO_PKG_VERSION")).expect("the package version is valid semver")
}

#[derive(Debug, Clone)]
pub struct Release {
    pub version: Version,
    pub tag: String,
}

/// Finds the latest release from where GitHub redirects `releases/latest`
/// (`.../releases/tag/v1.2.3`), without downloading the page.
pub fn latest_release(releases_url: &str, timeout: Duration) -> Result<Release> {
    let http = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(timeout)
        .redirect(Policy::none())
        .build()?;
    let response = http
        .get(format!("{releases_url}/latest"))
        .send()
        .context("could not reach GitHub")?;
    let status = response.status();
    let tag = response
        .headers()
        .get(LOCATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|location| location.rsplit_once("/tag/"))
        .map(|(_, tag)| tag.trim_end_matches('/').to_string())
        .with_context(|| format!("GitHub answered {status} without naming the latest release"))?;
    let version = Version::parse(tag.strip_prefix('v').unwrap_or(&tag))
        .with_context(|| format!("the latest release tag \"{tag}\" is not a version number"))?;
    Ok(Release { version, tag })
}

/// The archive of a release for one platform.
#[derive(Debug, Clone)]
pub struct Download {
    pub name: String,
    pub url: String,
    sha256: String,
}

/// The part of `dist-manifest.json`, published with every release, that
/// lists the archives and their checksums.
#[derive(Deserialize)]
struct Manifest {
    #[serde(default)]
    artifacts: HashMap<String, Artifact>,
}

#[derive(Deserialize)]
struct Artifact {
    name: String,
    #[serde(default)]
    kind: String,
    #[serde(default)]
    target_triples: Vec<String>,
    #[serde(default)]
    checksums: HashMap<String, String>,
}

/// Looks up the archive for `target` in the release's manifest.
pub fn find_download(
    releases_url: &str,
    release: &Release,
    target: &str,
    timeout: Duration,
) -> Result<Download> {
    let base = format!("{releases_url}/download/{}", release.tag);
    let http = Client::builder()
        .user_agent(USER_AGENT)
        .timeout(timeout)
        .build()?;
    let manifest: Manifest =
        serde_json::from_slice(&download(&http, &format!("{base}/dist-manifest.json"))?)
            .with_context(|| {
                format!("the file list of release {} is not valid", release.version)
            })?;
    let artifact = manifest
        .artifacts
        .into_values()
        .find(|a| a.kind == "executable-zip" && a.target_triples.iter().any(|t| t == target))
        .with_context(|| format!("release {} has no download for {target}", release.version))?;
    let sha256 = artifact.checksums.get("sha256").cloned().with_context(|| {
        format!(
            "release {} has no checksum for {}, so it cannot be verified",
            release.version, artifact.name
        )
    })?;
    Ok(Download {
        url: format!("{base}/{}", artifact.name),
        name: artifact.name,
        sha256,
    })
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct CheckRecord {
    checked_at: Option<DateTime<Utc>>,
    latest: Option<String>,
}

fn read_record(paths: &Paths) -> CheckRecord {
    fs::read_to_string(paths.update_check_file())
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

/// Notes that a check ran at `now` and, when it got an answer, the latest version.
pub fn remember(paths: &Paths, now: DateTime<Utc>, latest: Option<&Version>) -> Result<()> {
    let mut record = read_record(paths);
    record.checked_at = Some(now);
    if let Some(latest) = latest {
        record.latest = Some(latest.to_string());
    }
    let path = paths.update_check_file();
    fs::write(&path, serde_json::to_string(&record)?)
        .with_context(|| format!("could not write {}", path.display()))
}

fn newer(version: Version) -> Option<Version> {
    (version > current()).then_some(version)
}

/// The latest release an earlier check saw, if it is newer than this build.
pub fn known_newer(paths: &Paths) -> Option<Version> {
    newer(read_record(paths).latest?.parse().ok()?)
}

/// Whether a day has passed since the last check.
pub fn check_due(paths: &Paths, now: DateTime<Utc>) -> bool {
    match read_record(paths).checked_at {
        Some(at) => now < at || now - at >= CHECK_INTERVAL,
        None => true,
    }
}

/// Checks for a newer release on another thread; the receiver gets its
/// version if there is one. The attempt is recorded first, so when GitHub
/// cannot be reached it is tried again only the next day.
pub fn spawn_check(
    paths: Paths,
    releases_url: String,
    now: DateTime<Utc>,
) -> Option<Receiver<Version>> {
    remember(&paths, now, None).ok()?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        if let Ok(release) = latest_release(&releases_url, BACKGROUND_TIMEOUT) {
            let _ = remember(&paths, Utc::now(), Some(&release.version));
            if let Some(version) = newer(release.version) {
                let _ = tx.send(version);
            }
        }
    });
    Some(rx)
}

/// The release target this build runs on, as release archives name it.
pub fn target() -> Option<&'static str> {
    if cfg!(all(
        target_os = "windows",
        target_arch = "x86_64",
        target_env = "msvc"
    )) {
        Some("x86_64-pc-windows-msvc")
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some("x86_64-apple-darwin")
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("aarch64-apple-darwin")
    } else if cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        Some("x86_64-unknown-linux-gnu")
    } else if cfg!(all(
        target_os = "linux",
        target_arch = "aarch64",
        target_env = "gnu"
    )) {
        Some("aarch64-unknown-linux-gnu")
    } else {
        None
    }
}

/// Where the install script writes its receipt.
pub fn receipt_path() -> Option<PathBuf> {
    let base = match std::env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
        Some(dir) => PathBuf::from(dir),
        None if cfg!(windows) => PathBuf::from(std::env::var_os("LOCALAPPDATA")?),
        None => directories::BaseDirs::new()?.home_dir().join(".config"),
    };
    Some(base.join("stealthlingo").join("stealthlingo-receipt.json"))
}

/// What the install script recorded about an installation.
#[derive(Debug)]
pub struct Receipt {
    path: PathBuf,
    json: serde_json::Value,
}

impl Receipt {
    /// Reads the receipt at `path` if it describes an installation in `dir`.
    pub fn for_dir(path: &Path, dir: &Path) -> Option<Self> {
        let json: serde_json::Value = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
        let prefix = PathBuf::from(json.get("install_prefix")?.as_str()?);
        // Depending on the layout, binaries go in the prefix itself or in its `bin`.
        let installed_here = same_dir(&prefix, dir) || same_dir(&prefix.join("bin"), dir);
        installed_here.then(|| Self {
            path: path.to_path_buf(),
            json,
        })
    }

    /// Records the version now installed, as the install script would.
    pub fn set_version(mut self, version: &Version) -> Result<()> {
        self.json["version"] = version.to_string().into();
        fs::write(&self.path, self.json.to_string())
            .with_context(|| format!("could not update {}", self.path.display()))
    }
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

fn exe_name(binary: &str) -> String {
    format!("{binary}{}", std::env::consts::EXE_SUFFIX)
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = OsString::from(path.as_os_str());
    name.push(suffix);
    PathBuf::from(name)
}

/// Deletes programs an earlier update moved aside while they were running.
pub fn remove_leftovers(dir: &Path) {
    for binary in BINARIES {
        let _ = fs::remove_file(with_suffix(&dir.join(exe_name(binary)), ".old"));
    }
}

/// Downloads `archive`, checks it against its published SHA-256 and
/// replaces the StealthLingo programs in `dir`. Returns the file names that
/// were replaced.
pub fn install(archive: &Download, dir: &Path, connect_timeout: Duration) -> Result<Vec<String>> {
    let http = Client::builder()
        .user_agent(USER_AGENT)
        .connect_timeout(connect_timeout)
        .timeout(DOWNLOAD_TIMEOUT)
        .build()?;
    let bytes = download(&http, &archive.url)?;
    verify(&bytes, &archive.sha256)?;
    let files = extract(&archive.name, &bytes)?;
    replace(dir, &files)?;
    Ok(files.into_iter().map(|(name, _)| name).collect())
}

fn download(http: &Client, url: &str) -> Result<Vec<u8>> {
    let response = http
        .get(url)
        .send()
        .with_context(|| format!("could not download {url}"))?;
    let status = response.status();
    if !status.is_success() {
        bail!("downloading {url} failed: {status}");
    }
    Ok(response
        .bytes()
        .with_context(|| format!("could not download {url}"))?
        .to_vec())
}

fn verify(bytes: &[u8], expected_hex: &str) -> Result<()> {
    let actual: String = Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if !expected_hex.trim().eq_ignore_ascii_case(&actual) {
        bail!("the download does not match its published checksum; nothing was changed, try again later");
    }
    Ok(())
}

/// The last component of a path inside an archive.
fn base_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}

/// The StealthLingo programs in a release archive, wherever they sit in it.
fn extract(archive_name: &str, bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>> {
    let wanted: Vec<String> = BINARIES.iter().map(|b| exe_name(b)).collect();
    let mut found: Vec<(String, Vec<u8>)> = Vec::new();
    let damaged = "the downloaded archive is damaged; nothing was changed, try again later";
    if archive_name.ends_with(".zip") {
        let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).context(damaged)?;
        for i in 0..zip.len() {
            let mut file = zip.by_index(i).context(damaged)?;
            let name = base_name(&file.name().context(damaged)?).to_string();
            if file.is_file() && wanted.contains(&name) {
                let mut data = Vec::new();
                file.read_to_end(&mut data).context(damaged)?;
                found.push((name, data));
            }
        }
    } else if archive_name.ends_with(".tar.xz") {
        let mut tar_bytes = Vec::new();
        lzma_rs::xz_decompress(&mut Cursor::new(bytes), &mut tar_bytes)
            .map_err(|err| anyhow!("{err:?}"))
            .context(damaged)?;
        let mut tar = tar::Archive::new(Cursor::new(tar_bytes));
        for entry in tar.entries().context(damaged)? {
            let mut entry = entry.context(damaged)?;
            let path = entry.path().context(damaged)?;
            let name = base_name(&path.to_string_lossy()).to_string();
            if entry.header().entry_type().is_file() && wanted.contains(&name) {
                let mut data = Vec::new();
                entry.read_to_end(&mut data).context(damaged)?;
                found.push((name, data));
            }
        }
    } else {
        bail!("cannot unpack {archive_name}");
    }
    let main = exe_name("stealthlingo");
    if !found.iter().any(|(name, _)| *name == main) {
        bail!("the release archive does not contain {main}; nothing was changed");
    }
    found.sort_by_key(|(name, _)| *name == main);
    Ok(found)
}

/// Puts each file in `dir`. Running programs cannot be overwritten on
/// Windows but can be renamed, so the old file is moved to `<name>.old`
/// first and deleted when it is no longer in use.
fn replace(dir: &Path, files: &[(String, Vec<u8>)]) -> Result<()> {
    let mut staged = Vec::new();
    let written = files.iter().try_for_each(|(name, data)| {
        let target = dir.join(name);
        let new = with_suffix(&target, ".new");
        fs::write(&new, data).with_context(|| {
            format!(
                "could not write to {}; check that you have permission to change it",
                dir.display()
            )
        })?;
        staged.push((new.clone(), target));
        make_executable(&new)
    });
    if let Err(err) = written {
        for (new, _) in &staged {
            let _ = fs::remove_file(new);
        }
        return Err(err);
    }
    for (new, target) in &staged {
        let old = with_suffix(target, ".old");
        let _ = fs::remove_file(&old);
        let moved = target.exists();
        if moved {
            fs::rename(target, &old)
                .with_context(|| format!("could not move {} aside", target.display()))?;
        }
        if let Err(err) = fs::rename(new, target) {
            if moved {
                let _ = fs::rename(&old, target);
            }
            return Err(err).with_context(|| format!("could not replace {}", target.display()));
        }
        if moved {
            let _ = fs::remove_file(&old);
        }
    }
    Ok(())
}

#[cfg(unix)]
fn make_executable(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .with_context(|| format!("could not make {} executable", path.display()))
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksums_must_match() {
        let sum = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        assert!(verify(b"hello", sum).is_ok());
        assert!(verify(b"hello", &sum.to_uppercase()).is_ok());
        assert!(verify(b"hullo", sum).is_err());
        assert!(verify(b"hello", "").is_err());
    }

    #[test]
    fn archive_paths_are_reduced_to_file_names() {
        assert_eq!(
            base_name("stealthlingo-x86_64-unknown-linux-gnu/stealthlingo"),
            "stealthlingo"
        );
        assert_eq!(base_name("stealthlingo.exe"), "stealthlingo.exe");
    }
}
