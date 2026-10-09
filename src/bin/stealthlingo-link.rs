//! Handler for `stealthlingo://` links, started by the OS when a link printed
//! by `lookup` is Ctrl+clicked. Built without a console window on Windows so
//! clicking does not flash a terminal; failures are written to `link-errors.log`
//! in the data directory instead.

#![cfg_attr(windows, windows_subsystem = "windows")]

use std::io::Write;

fn main() {
    let Some(link) = std::env::args().nth(1) else {
        return;
    };
    let result = stealthlingo::commands::Context::load()
        .and_then(|ctx| stealthlingo::commands::links::open(&ctx, &link));
    if let Err(err) = result {
        if let Ok(paths) = stealthlingo::config::Paths::resolve() {
            let log = paths.data_dir.join("link-errors.log");
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log)
            {
                let _ = writeln!(file, "{} {link}: {err:#}", chrono::Utc::now().to_rfc3339());
            }
        }
        std::process::exit(1);
    }
}
