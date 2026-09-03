//! `oh-seal` — the artifact seal gate (masterplan L0.6).
//!
//! Enforces the versioning rules of `VERSIONING.md` mechanically over
//! `artifacts.json`, the closed-world register of the standard's
//! artifacts:
//!
//! - every registered file must exist and match its recorded SHA-256;
//! - a `draft` mismatch demands a conscious re-record (`--update`) in
//!   the same commit as the content change;
//! - a `published` mismatch is a hard stop — published artifacts are
//!   byte-frozen, change means a NEW version, never an edit;
//! - every file in the artifact locations must be registered (closed
//!   world — nothing ships unsealed by omission).
//!
//! The register file itself is written back in one canonical shape
//! (2-space indent, LF, trailing newline) so diffs carry content.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// One registered artifact. Field order = serialization order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    /// Path relative to the standard root (`registry/standard`).
    pub path: String,
    /// Canonical IRI once served; `null` for validation-internal files.
    pub iri: Option<String>,
    /// `draft` (pre-publication) or `published` (byte-frozen).
    pub status: Status,
    pub sha256: String,
    /// Publication date (YYYY-MM-DD), present from publication on.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_on: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Draft,
    Published,
    /// Byte-frozen like `published`, but by SUPERSESSION (a newer
    /// version replaced it pre-publication) — never updatable, no
    /// `published_on` required (Reflexionsschleife lens 4: the
    /// testing standard's superseded/ discipline joins the gate).
    Superseded,
}

/// One scanned directory of the closed world.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirRule {
    pub path: String,
    pub extensions: Vec<String>,
}

/// The closed world as DATA in the register itself (lens 4: one
/// gate, many standards — the code no longer hardcodes one
/// standard's layout).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Locations {
    pub dirs: Vec<DirRule>,
    pub files: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Register {
    #[serde(rename = "$comment")]
    pub comment: String,
    pub locations: Locations,
    pub artifacts: Vec<Artifact>,
}

pub const REGISTER_FILE: &str = "artifacts.json";

pub fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn load_register(root: &Path) -> Result<Register> {
    let path = root.join(REGISTER_FILE);
    let raw = fs::read_to_string(&path)
        .with_context(|| format!("reading register {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parsing {}", path.display()))
}

/// Writes the register back in the one canonical shape.
pub fn save_register(root: &Path, register: &Register) -> Result<()> {
    let mut out = serde_json::to_string_pretty(register).context("serializing register")?;
    out.push('\n');
    fs::write(root.join(REGISTER_FILE), out).context("writing register")?;
    Ok(())
}

/// Enumerates every file the register's closed world considers an
/// artifact (locations are register DATA — a new location is a
/// deliberate register+VERSIONING.md change, never a code edit).
fn scan_artifacts(root: &Path, locations: &Locations) -> Result<Vec<String>> {
    let mut found = Vec::new();
    for rule in &locations.dirs {
        let at_root = rule.path == ".";
        let dir_path = if at_root {
            root.to_path_buf()
        } else {
            root.join(&rule.path)
        };
        if !dir_path.is_dir() {
            continue;
        }
        for entry in
            fs::read_dir(&dir_path).with_context(|| format!("reading {}", dir_path.display()))?
        {
            let path = entry?.path();
            if path.is_file()
                && let Some(ext) = path.extension().and_then(|e| e.to_str())
                && rule.extensions.iter().any(|e| e == ext)
            {
                let file = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
                // The register itself is the one root file that never
                // needs a row — it seals, it is not sealed.
                if at_root && file == REGISTER_FILE {
                    continue;
                }
                if at_root {
                    found.push(file.to_string());
                } else {
                    found.push(format!("{}/{file}", rule.path));
                }
            }
        }
    }
    for file in &locations.files {
        if root.join(file).is_file() {
            found.push(file.clone());
        }
    }
    found.sort();
    Ok(found)
}

/// The gate: returns one finding per violation; empty = green.
pub fn check(root: &Path) -> Result<Vec<String>> {
    let register = load_register(root)?;
    let mut findings = Vec::new();

    // Reflexionsschleife AM, the material finding: every one of the
    // five registers covered its subdirectories through `dirs` but its
    // own root only through explicitly named `files` — so a NEW file
    // at the root (exactly where a young standard's artifacts are
    // born) shipped unsealed in silence. The closed world must close
    // over the root: a register without a `.` rule is itself a
    // finding, not a configuration choice.
    if !register.locations.dirs.iter().any(|r| r.path == ".") {
        findings.push(format!(
            "{REGISTER_FILE}: locations.dirs has no `.` rule — the closed world is OPEN \
             at the register's own root; add {{\"path\": \".\", \"extensions\": [...]}} \
             covering the artifact extensions this home seals"
        ));
    }

    for artifact in &register.artifacts {
        let file = root.join(&artifact.path);
        let bytes = match fs::read(&file) {
            Ok(b) => b,
            Err(_) => {
                findings.push(format!(
                    "{}: registered but missing on disk — restore it or remove the row \
                     (published rows are never removed, they are superseded)",
                    artifact.path
                ));
                continue;
            }
        };
        let actual = sha256_hex(&bytes);
        if actual != artifact.sha256 {
            match artifact.status {
                Status::Draft => findings.push(format!(
                    "{}: DRAFT DRIFT — content changed without re-recording the seal; \
                     run `oh-seal --update {}` in the SAME commit as the change",
                    artifact.path, artifact.path
                )),
                Status::Published => findings.push(format!(
                    "{}: PUBLISHED ARTIFACT MODIFIED — published artifacts are \
                     byte-frozen (immutable from first publication); revert the edit \
                     and create a NEW version instead",
                    artifact.path
                )),
                Status::Superseded => findings.push(format!(
                    "{}: SUPERSEDED ARTIFACT MODIFIED — superseded versions are \
                     byte-frozen history; revert the edit",
                    artifact.path
                )),
            }
        }
        if artifact.status == Status::Published && artifact.published_on.is_none() {
            findings.push(format!(
                "{}: published without a published_on date — publish only via `oh-seal --publish`",
                artifact.path
            ));
        }
    }

    let registered: Vec<&str> = register.artifacts.iter().map(|a| a.path.as_str()).collect();
    for file in scan_artifacts(root, &register.locations)? {
        if !registered.contains(&file.as_str()) {
            findings.push(format!(
                "{file}: artifact file NOT in the register — nothing ships unsealed; \
                 add a row to {REGISTER_FILE} (status draft) with its SHA-256"
            ));
        }
    }

    Ok(findings)
}

fn position_of(register: &Register, path: &str) -> Result<usize> {
    register
        .artifacts
        .iter()
        .position(|a| a.path == path)
        .with_context(|| format!("{path}: not in the register"))
}

/// Re-records a DRAFT artifact's hash after a conscious change.
pub fn update(root: &Path, path: &str) -> Result<()> {
    let mut register = load_register(root)?;
    let idx = position_of(&register, path)?;
    if register.artifacts[idx].status != Status::Draft {
        bail!("{path}: only draft artifacts are updatable — published/superseded are byte-frozen");
    }
    let bytes = fs::read(root.join(path)).with_context(|| format!("{path}: reading for update"))?;
    register.artifacts[idx].sha256 = sha256_hex(&bytes);
    save_register(root, &register)
}

/// Seals a draft artifact as published at its CURRENT bytes.
/// Refuses when the recorded hash is stale — sealing binds to bytes
/// that were consciously recorded, never to whatever lies on disk.
pub fn publish(root: &Path, path: &str, date: &str) -> Result<()> {
    if !is_iso_date(date) {
        bail!("--date must be YYYY-MM-DD, got «{date}»");
    }
    let mut register = load_register(root)?;
    let idx = position_of(&register, path)?;
    if register.artifacts[idx].status == Status::Published {
        bail!(
            "{path}: already published ({})",
            register.artifacts[idx]
                .published_on
                .as_deref()
                .unwrap_or("no date")
        );
    }
    let bytes =
        fs::read(root.join(path)).with_context(|| format!("{path}: reading for publish"))?;
    let actual = sha256_hex(&bytes);
    if actual != register.artifacts[idx].sha256 {
        bail!(
            "{path}: recorded hash is stale — run the gate (and `--update` consciously) \
             before publishing; sealing binds to recorded bytes"
        );
    }
    register.artifacts[idx].status = Status::Published;
    register.artifacts[idx].published_on = Some(date.to_string());
    save_register(root, &register)
}

fn is_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| match i {
            4 | 7 => *c == b'-',
            _ => c.is_ascii_digit(),
        })
}

/// Resolves the standard root for CLI and tests.
pub fn resolve_root(explicit: Option<PathBuf>) -> PathBuf {
    explicit.unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_of_empty_is_the_known_constant() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn iso_date_validation() {
        assert!(is_iso_date("2026-08-20"));
        assert!(!is_iso_date("20.08.2026"));
        assert!(!is_iso_date("2026-8-20"));
        assert!(!is_iso_date(""));
    }
}
