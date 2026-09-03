use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use serde_json::Value;

use oh_validate::{format, stage2, stage3, stage4};

const USAGE: &str = "usage: oh-validate <manifest.schema.json> <file-or-dir>... \
[--stage2 <context/v1.jsonld>] [--stage3 <shapes/manifest-v1.ttl>] [--stage4] [--write]";

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();

    // Optional gates: `--stage2 <vendored context>`, `--stage3 <shapes>`
    // (stage 3 needs the stage-2 loader, so --stage3 implies --stage2).
    let mut stage2_context: Option<String> = None;
    let mut stage3_shapes: Option<String> = None;
    for flag in ["--stage2", "--stage3"] {
        if let Some(pos) = args.iter().position(|a| a == flag) {
            if pos + 1 >= args.len() {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
            let value = args.remove(pos + 1);
            args.remove(pos);
            match flag {
                "--stage2" => stage2_context = Some(value),
                _ => stage3_shapes = Some(value),
            }
        }
    }
    // `--stage4` merges all documents into ONE registry graph:
    // RDFC-1.0 fingerprint + critical invariants as SPARQL.
    let stage4 = if let Some(pos) = args.iter().position(|a| a == "--stage4") {
        args.remove(pos);
        true
    } else {
        false
    };
    if (stage3_shapes.is_some() || stage4) && stage2_context.is_none() {
        eprintln!("--stage3/--stage4 require --stage2 (the offline context loader)");
        return ExitCode::from(2);
    }
    // `--write` normalizes non-canonical formatting in place instead
    // of reporting it as an error.
    let write = if let Some(pos) = args.iter().position(|a| a == "--write") {
        args.remove(pos);
        true
    } else {
        false
    };

    if args.len() < 2 {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }
    match run(
        &args[0],
        &args[1..],
        stage2_context.as_deref(),
        stage3_shapes.as_deref(),
        stage4,
        write,
    ) {
        Ok(0) => ExitCode::SUCCESS,
        Ok(n) => {
            eprintln!("FAIL: {n} error(s)");
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn run(
    schema_path: &str,
    targets: &[String],
    stage2_context: Option<&str>,
    stage3_shapes: Option<&str>,
    stage4: bool,
    write: bool,
) -> Result<usize> {
    let schema_raw = std::fs::read_to_string(schema_path)
        .with_context(|| format!("reading schema {schema_path}"))?;
    let schema: Value = serde_json::from_str(&schema_raw).context("parsing schema")?;
    let validator = jsonschema::options()
        .should_validate_formats(true)
        .build(&schema)
        .context("compiling schema")?;

    let loader = stage2_context
        .map(|p| stage2::VendoredLoader::from_file(Path::new(p)))
        .transpose()?;

    let mut files: Vec<PathBuf> = Vec::new();
    for t in targets {
        collect_json_files(Path::new(t), &mut files)?;
    }
    files.sort();
    if files.is_empty() {
        anyhow::bail!("no .json files found in given targets");
    }

    let mut errors = 0usize;
    // @id must be unique across the whole set (registry-wide identity)
    let mut seen_ids: HashMap<String, PathBuf> = HashMap::new();
    // Raw documents for the stage-4 whole-graph merge.
    let mut corpus: Vec<(String, String)> = Vec::new();

    for file in &files {
        let raw =
            std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
        corpus.push((file.display().to_string(), raw.clone()));
        let doc: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => {
                println!("{}: invalid JSON: {e}", file.display());
                errors += 1;
                continue;
            }
        };

        let mut file_errors = 0usize;
        for err in validator.iter_errors(&doc) {
            println!("{}: {}: {}", file.display(), err.instance_path(), err);
            file_errors += 1;
        }
        errors += file_errors;

        // Stage 1b: canonical byte formatting (schema-valid files only
        // — a broken file gets content fixes before style fixes).
        if file_errors == 0
            && let Some(msg) = format::check(&raw, &doc)
        {
            if write {
                std::fs::write(file, format::canonical(&doc))
                    .with_context(|| format!("normalizing {}", file.display()))?;
                println!("{}: normalized to canonical formatting", file.display());
            } else {
                println!("{}: format: {}", file.display(), msg);
                errors += 1;
            }
        }

        // Stage 2 (round-trip gate) only for documents that passed
        // stage 1 — schema violations would drown the drift report.
        if file_errors == 0
            && let Some(loader) = &loader
        {
            let mut stage2_errors = 0usize;
            for finding in stage2::check(&raw, loader)? {
                println!("{}: stage 2: {}", file.display(), finding);
                stage2_errors += 1;
            }
            errors += stage2_errors;

            // Stage 3 only on stage-2-clean documents: SHACL findings
            // on a non-canonical graph would be noise.
            if stage2_errors == 0
                && let Some(shapes) = stage3_shapes
                && let Some(report) = stage3::check(&raw, loader, Path::new(shapes))?
            {
                let first = report
                    .lines()
                    .filter(|l| l.contains("resultPath") || l.contains("Constraint"))
                    .take(3)
                    .collect::<Vec<_>>()
                    .join(" | ");
                println!(
                    "{}: stage 3: SHACL violations ({})",
                    file.display(),
                    if first.is_empty() {
                        "see report"
                    } else {
                        &first
                    }
                );
                errors += 1;
            }
        }

        if let Some(id) = doc.get("@id").and_then(Value::as_str) {
            // filename stem must match the @id slug — Git path and URI never diverge
            let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let id_slug = id.rsplit('/').next().unwrap_or("");
            if stem != id_slug {
                println!(
                    "{}: filename '{stem}' does not match @id slug '{id_slug}'",
                    file.display()
                );
                errors += 1;
            }
            if let Some(prev) = seen_ids.insert(id.to_string(), file.clone()) {
                println!(
                    "{}: duplicate @id '{id}' (also in {})",
                    file.display(),
                    prev.display()
                );
                errors += 1;
            }
        }
    }

    // Stage 4 only on a fully clean corpus — one broken document
    // would poison the merged graph and drown the diagnostics.
    if stage4 {
        if errors > 0 {
            println!("stage 4: skipped ({errors} earlier error(s))");
        } else if let Some(loader) = &loader {
            let report = stage4::check(&corpus, loader)?;
            println!(
                "stage 4: graph fingerprint (RDFC-1.0/SHA-256): {}",
                report.fingerprint
            );
            for v in &report.violations {
                println!("stage 4: {v}");
            }
            errors += report.violations.len();
        }
    }

    println!(
        "checked {} file(s): {}",
        files.len(),
        if errors == 0 {
            "OK".into()
        } else {
            format!("{errors} error(s)")
        }
    );
    Ok(errors)
}

fn collect_json_files(path: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if path.is_dir() {
        for entry in
            std::fs::read_dir(path).with_context(|| format!("reading dir {}", path.display()))?
        {
            collect_json_files(&entry?.path(), out)?;
        }
    } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
        out.push(path.to_path_buf());
    }
    Ok(())
}
