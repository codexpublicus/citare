//! Build-time data compilation.
//!
//! Reads reporters-db and courts-db JSON, performs template variable
//! substitution, and builds the data bundle ([`bundle`]) the tokenizer
//! loads.

pub mod bundle;
pub mod courts;
pub mod harvard;
pub mod regexes;
pub mod reporters;
pub mod template;
