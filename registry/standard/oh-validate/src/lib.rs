//! oh-validate — validation tooling for the OpenHelvetia manifest
//! standard. Stage 1 (JSON Schema + slug/uniqueness) lives in the
//! binary; stage 2 (JSON-LD round-trip gate, E16 Ziff. 5c) is exposed
//! here so integration tests and future callers (site-gen gate, CI)
//! can drive it directly.

pub mod format;
pub mod stage2;
pub mod stage3;
pub mod stage4;
