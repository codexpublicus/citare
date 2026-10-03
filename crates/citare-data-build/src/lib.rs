//! Build-time data compilation.
//!
//! Reads reporters-db and courts-db JSON, performs template variable
//! substitution, and generates Rust source for compiled extractors.

pub mod courts;
pub mod harvard;
pub mod regexes;
pub mod reporters;
pub mod template;
