//! `coverage` — the ontology coverage gate (masterplan L0.5).
//!
//! Checks that every term emitted by the platform's published artefacts
//! (context + three schemas) is covered by `ontology/oh.ttl`, and that the
//! committed `ontology/oh.md` matches its regeneration from the Turtle
//! master. Exit code 0 only when both gates are green; orphaned terms are
//! warnings and never fail the run.
//!
//! Usage:
//!   coverage [--root <registry/standard dir>] [--write-doc]
//!
//! `--write-doc` rewrites `oh.md` instead of checking it for drift.

use std::path::PathBuf;
use std::process::ExitCode;

use oh_coverage::{Paths, doc, load, report};

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let mut root: Option<PathBuf> = None;
    let mut write_doc = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => {
                let value = args.next().ok_or("--root needs a directory argument")?;
                root = Some(PathBuf::from(value));
            }
            "--write-doc" => write_doc = true,
            "--help" | "-h" => {
                println!("usage: coverage [--root <registry/standard dir>] [--write-doc]");
                return Ok(ExitCode::SUCCESS);
            }
            other => return Err(format!("unknown argument '{other}' (see --help)")),
        }
    }
    // Default: this crate lives at registry/standard/ontology/coverage.
    let root = root.unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."));
    let paths = Paths::under(&root);

    let loaded = load(&paths)?;
    let coverage = report::check(&loaded.model, &loaded.requirements);
    print!("{}", report::render(&coverage));

    let mut failed = !coverage.ok();

    let generated = doc::generate(&loaded.model);
    if write_doc {
        std::fs::write(&paths.ontology_doc, &generated)
            .map_err(|e| format!("cannot write {}: {e}", paths.ontology_doc.display()))?;
        println!("doc: {} regenerated.", paths.ontology_doc.display());
    } else {
        let committed = std::fs::read_to_string(&paths.ontology_doc)
            .map_err(|e| format!("cannot read {}: {e}", paths.ontology_doc.display()))?;
        if committed == generated {
            println!("doc: oh.md matches its regeneration from oh.ttl.");
        } else {
            println!(
                "doc: FAILURE — oh.md differs from its regeneration from oh.ttl (run with --write-doc and commit the result)."
            );
            failed = true;
        }
    }

    Ok(if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}
