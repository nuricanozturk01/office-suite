//! Repo tasks. `cargo run -p xtask -- arch` enforces module boundaries (CLAUDE.md §4.2.7).

#![forbid(unsafe_code)]

mod arch;
mod budget;
mod vendor;

use std::path::Path;
use std::process::ExitCode;

use anyhow::Context;

#[tokio::main]
async fn main() -> ExitCode {
    let task = std::env::args().nth(1);
    let result = match task.as_deref() {
        Some("arch") => arch::check(),
        Some("vendor-check") => vendor_check(),
        Some("budget") => budget(),
        Some("migrate") => migrate().await,
        _ => Err(anyhow::anyhow!(
            "usage: xtask <arch|vendor-check|budget|migrate>"
        )),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            tracing_fallback(&err);
            ExitCode::FAILURE
        }
    }
}

/// Fails when `vendor/` provenance (SHA, licenses, listed crates) is not in order.
fn vendor_check() -> anyhow::Result<()> {
    let violations = vendor::vendor_check(Path::new("."));
    if violations.is_empty() {
        Ok(())
    } else {
        anyhow::bail!("vendor violations:\n{}", violations.join("\n"))
    }
}

/// Fails when a built asset exceeds its gzip budget in `budgets.toml`.
fn budget() -> anyhow::Result<()> {
    let root = Path::new(".");
    let violations = budget::budget_violations(root, &budget::load(root)?);
    if violations.is_empty() {
        Ok(())
    } else {
        anyhow::bail!("budget violations:\n{}", violations.join("\n"))
    }
}

/// Applies every module's migrations to `DATABASE_URL`.
async fn migrate() -> anyhow::Result<()> {
    let url = std::env::var("DATABASE_URL").context("DATABASE_URL")?;
    let pool = platform::db::connect(&secrecy::SecretString::from(url)).await?;
    platform::db::migrate(&pool, &platform::migrator()).await?;
    platform::db::migrate(&pool, &identity::migrator()).await?;
    platform::db::migrate(&pool, &documents::migrator()).await?;
    platform::db::migrate(&pool, &spreadsheets::migrator()).await?;
    platform::db::migrate(&pool, &presentations::migrator()).await?;
    platform::db::migrate(&pool, &pdfs::migrator()).await?;
    platform::db::migrate(&pool, &storage::migrator()).await?;
    platform::db::migrate(&pool, &workspace::migrator()).await?;
    platform::db::migrate(&pool, &collaboration::migrator()).await?;
    Ok(())
}

fn tracing_fallback(err: &anyhow::Error) {
    use std::io::Write;
    // INVARIANT: xtask is a dev tool; stderr write failure is not actionable.
    let _ = writeln!(std::io::stderr(), "{err:#}");
}
