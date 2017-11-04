//! The base library for the Gefolge Discord bot, Peter

#![cfg_attr(test, deny(warnings))]
#![warn(trivial_casts)]
#![deny(unused, missing_docs, unused_qualifications)]
#![forbid(unused_extern_crates, unused_import_braces)]

extern crate rand;
extern crate serenity;
extern crate typemap;

pub mod bitbar;
pub mod commands;

/// The token for the Peter bot user.
pub const TOKEN: &str = "MzY1OTM2NDkzNTM5MjI5Njk5.DMUAVw.JteeTwsjbHOtNAHdMGXQJllCK6k";
