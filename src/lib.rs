//! The base library for the Gefolge Discord bot, Peter

#![cfg_attr(test, deny(warnings))]
#![warn(trivial_casts)]
#![deny(unused, missing_docs, unused_qualifications)]
#![forbid(unused_extern_crates, unused_import_braces)]

#[macro_use] extern crate lazy_static;
extern crate num;
extern crate parking_lot;
extern crate quantum_werewolf;
extern crate rand;
extern crate regex;
#[macro_use] extern crate serde_json;
extern crate serenity;
extern crate typemap;
#[macro_use] extern crate wrapped_enum;

use std::{fmt, io};

use serenity::model::{GuildId, UserIdParseError};

pub mod bitbar;
pub mod commands;
pub mod emoji;
pub mod lang;
pub mod parse;
pub mod user_list;
pub mod werewolf;

/// The Gefolge guild's ID.
pub const GEFOLGE: GuildId = GuildId(355761290809180170);

/// The token for the Peter bot user.
pub const TOKEN: &str = "MzY1OTM2NDkzNTM5MjI5Njk5.DMUAVw.JteeTwsjbHOtNAHdMGXQJllCK6k";

wrapped_enum! {
    #[allow(missing_docs)]
    #[derive(Debug)]
    pub enum Error {
        #[allow(missing_docs)]
        GameAction(String),
        #[allow(missing_docs)]
        Io(io::Error),
        #[allow(missing_docs)]
        Poison(::std::sync::PoisonError<()>),
        #[allow(missing_docs)]
        QwwStartGame(quantum_werewolf::game::state::StartGameError),
        #[allow(missing_docs)]
        Serenity(serenity::Error),
        #[allow(missing_docs)]
        UserIdParse(UserIdParseError),
        #[allow(missing_docs)]
        Unknown(())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::GameAction(ref s) => write!(f, "invalid game action: {}", s),
            Error::Io(ref e) => e.fmt(f),
            Error::Poison(ref e) => e.fmt(f),
            Error::QwwStartGame(ref e) => e.fmt(f),
            Error::Serenity(ref e) => e.fmt(f),
            Error::UserIdParse(ref e) => e.fmt(f),
            Error::Unknown(()) => write!(f, "unknown error")
        }
    }
}

#[allow(missing_docs)]
pub type Result<T> = ::std::result::Result<T, Error>;
