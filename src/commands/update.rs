use std::path::Path;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use anyhow::{Context as _, Result};
use chrono::Utc;
use semver::Version;

use super::Context;
use crate::update::{self, Receipt};

/// How long a command that already finished waits for the daily check.
const NOTICE_WAIT: Duration = Duration::from_secs(1);

pub fn run(ctx: &Context, check_only: bool) -> Result<()> {
    let exe = std::env::current_exe().context("could not find the running program")?;
    let dir = exe
        .parent()
        .context("could not find the folder of the running program")?;
    run_in(ctx, check_only, dir, update::receipt_path().as_deref())
}

/// `run` for the programs in `dir`, installed by the script that wrote the
/// receipt at `receipt` (if any).
pub fn run_in(ctx: &Context, check_only: bool, dir: &Path, receipt: Option<&Path>) -> Result<()> {
    update::remove_leftovers(dir);
    let current = update::current();
    let release = update::latest_release(&ctx.releases_url, ctx.timeout())
        .context("could not check for a new release; check your connection and try again")?;
    let _ = update::remember(&ctx.paths, Utc::now(), Some(&release.version));
    if release.version <= current {
        println!("StealthLingo {current} is the latest version.");
        return Ok(());
    }
    println!(
        "StealthLingo {} is available (you have {current}).",
        release.version
    );
    if check_only {
        println!("Run `stealthlingo update` to install it.");
        return Ok(());
    }
    let Some(receipt) = receipt.and_then(|path| Receipt::for_dir(path, dir)) else {
        println!(
            "This copy was not installed with the StealthLingo installer, so it is not replaced \
             automatically. Update it the way you installed it:"
        );
        println!("  installer:    {}", update::INSTALLER_COMMAND);
        println!("  from source:  {}", update::CARGO_COMMAND);
        return Ok(());
    };
    let Some(target) = update::target() else {
        println!("There is no prebuilt release for this platform. Update from source:");
        println!("  {}", update::CARGO_COMMAND);
        return Ok(());
    };
    let archive = update::find_download(&ctx.releases_url, &release, target, ctx.timeout())
        .context("the update was not installed")?;
    println!("Downloading {}...", archive.name);
    update::install(&archive, dir, ctx.timeout()).context("the update was not installed")?;
    if let Err(err) = receipt.set_version(&release.version) {
        eprintln!("Warning: {err:#}");
    }
    println!("Updated StealthLingo to {}.", release.version);
    Ok(())
}

/// Starts the daily check for a new release when it is turned on and due.
pub fn start_daily_check(ctx: &Context) -> Option<Receiver<Version>> {
    let now = Utc::now();
    if !ctx.config.check_updates || !update::check_due(&ctx.paths, now) {
        return None;
    }
    update::spawn_check(ctx.paths.clone(), ctx.releases_url.clone(), now)
}

/// Mentions a newer release found by `check`, waiting briefly for it.
pub fn report_daily_check(check: Option<Receiver<Version>>) {
    if let Some(version) = check.and_then(|rx| rx.recv_timeout(NOTICE_WAIT).ok()) {
        eprintln!("StealthLingo {version} is available; run `stealthlingo update` to install it.");
    }
}
