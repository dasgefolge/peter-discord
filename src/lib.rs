//! The base library for the Gefolge Discord bot, Peter

#![cfg_attr(test, deny(warnings))]
#![warn(trivial_casts)]
#![deny(unused, missing_docs, unused_qualifications)]
#![forbid(unused_extern_crates, unused_import_braces)]

#[macro_use] extern crate lazy_static;
extern crate num;
extern crate quantum_werewolf;
extern crate rand;
extern crate regex;
extern crate serenity;
extern crate typemap;
#[macro_use] extern crate wrapped_enum;

use std::fmt;

pub mod bitbar;
pub mod commands;
pub mod emoji;
pub mod lang;
pub mod parse;
pub mod werewolf;

/// The token for the Peter bot user.
pub const TOKEN: &str = "MzY1OTM2NDkzNTM5MjI5Njk5.DMUAVw.JteeTwsjbHOtNAHdMGXQJllCK6k";

wrapped_enum! {
    #[allow(missing_docs)]
    #[derive(Debug)]
    pub enum Error {
        #[allow(missing_docs)]
        QwwStartGame(quantum_werewolf::game::state::StartGameError),
        #[allow(missing_docs)]
        Serenity(serenity::Error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::QwwStartGame(ref e) => e.fmt(f),
            Error::Serenity(ref e) => e.fmt(f)
        }
    }
}

#[allow(missing_docs)]
pub type Result<T> = ::std::result::Result<T, Error>;
