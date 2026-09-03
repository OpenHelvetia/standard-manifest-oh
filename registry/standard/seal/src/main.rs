//! CLI for the artifact seal gate (L0.6). See lib.rs and
//! VERSIONING.md for the rules it enforces.
//!
//! Usage:
//!   oh-seal [--root <registry/standard dir>]                 check (default)
//!   oh-seal [--root …] --update <relative path>              re-record a draft hash
//!   oh-seal [--root …] --publish <relative path> --date <YYYY-MM-DD>

use std::path::PathBuf;
use std::process::ExitCode;

const USAGE: &str = "usage: oh-seal [--root <dir>] \
[--update <path> | --publish <path> --date <YYYY-MM-DD>]";

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> anyhow::Result<ExitCode> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();

    let mut take_value = |flag: &str| -> anyhow::Result<Option<String>> {
        if let Some(pos) = args.iter().position(|a| a == flag) {
            if pos + 1 >= args.len() {
                anyhow::bail!("{USAGE}");
            }
            let value = args.remove(pos + 1);
            args.remove(pos);
            Ok(Some(value))
        } else {
            Ok(None)
        }
    };

    let root = oh_seal::resolve_root(take_value("--root")?.map(PathBuf::from));
    let update = take_value("--update")?;
    let publish = take_value("--publish")?;
    let date = take_value("--date")?;
    if !args.is_empty() {
        anyhow::bail!("unexpected arguments {args:?}\n{USAGE}");
    }

    match (update, publish) {
        (Some(path), None) => {
            oh_seal::update(&root, &path)?;
            println!("updated: {path} re-recorded as draft");
            Ok(ExitCode::SUCCESS)
        }
        (None, Some(path)) => {
            let date = date.ok_or_else(|| anyhow::anyhow!("--publish requires --date\n{USAGE}"))?;
            oh_seal::publish(&root, &path, &date)?;
            println!("published: {path} sealed as of {date}");
            Ok(ExitCode::SUCCESS)
        }
        (Some(_), Some(_)) => anyhow::bail!("--update and --publish are mutually exclusive"),
        (None, None) => {
            let findings = oh_seal::check(&root)?;
            for f in &findings {
                println!("seal: {f}");
            }
            if findings.is_empty() {
                println!("seal gate: all registered artifacts match ({} rows)", {
                    oh_seal::load_register(&root)?.artifacts.len()
                });
                Ok(ExitCode::SUCCESS)
            } else {
                eprintln!("FAIL: {} seal finding(s)", findings.len());
                Ok(ExitCode::FAILURE)
            }
        }
    }
}
