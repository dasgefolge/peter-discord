#![deny(rust_2018_idioms, unused, unused_crate_dependencies, unused_import_braces, unused_lifetimes, unused_qualifications, warnings)]
#![forbid(unsafe_code)]

use {
    std::{
        env,
        io,
    },
    serenity::{
        model::prelude::*,
        prelude::*,
    },
    sqlx::PgPool,
    wheel::traits::IsNetworkError,
};

pub mod config;
pub mod ipc;
pub mod lang;
pub mod parse;
pub mod twitch;
pub mod user_list;
pub mod werewolf;

pub const GEFOLGE: GuildId = GuildId::new(355761290809180170);

pub const ADMIN: RoleId = RoleId::new(355776689051140099);
pub const QUIZMASTER: RoleId = RoleId::new(847443327069454378);
pub const MENSCH: RoleId = RoleId::new(386753710434287626);
pub const GUEST: RoleId = RoleId::new(784929665478557737);

pub const FENHL: UserId = UserId::new(86841168427495424);

/// `typemap` key for the PostgreSQL database connection.
pub struct Database;

impl TypeMapKey for Database {
    type Value = PgPool;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)] Env(#[from] env::VarError),
    #[error(transparent)] Io(#[from] io::Error),
    #[error(transparent)] Ipc(#[from] ipc::Error),
    #[error(transparent)] Json(#[from] serde_json::Error),
    #[error(transparent)] QwwStartGame(#[from] quantum_werewolf::game::state::StartGameError),
    #[error(transparent)] Serenity(#[from] serenity::Error),
    #[error(transparent)] Sql(#[from] sqlx::Error),
    #[error(transparent)] Twitch(#[from] twitch_helix::Error),
    #[error(transparent)] Wheel(#[from] wheel::Error),
    #[error("invalid game action: {0}")]
    GameAction(String),
    /// Returned if the config is not present in Serenity context.
    #[error("config missing in Serenity context")]
    MissingConfig,
    /// Returned if a Serenity context was required outside of an event handler but the `ready` event has not been received yet.
    #[error("Serenity context not available before ready event")]
    MissingContext,
    /// The reply to an IPC command did not end in a newline.
    #[error("the reply to an IPC command did not end in a newline")]
    MissingNewline,
    #[error("Twitch returned unexpected user info")]
    TwitchUserLookup,
}

impl IsNetworkError for Error {
    fn is_network_error(&self) -> bool {
        match self {
            | Self::Env(_)
            | Self::Ipc(_)
            | Self::Json(_)
            | Self::QwwStartGame(_)
            | Self::Sql(_)
            | Self::GameAction(_)
            | Self::MissingConfig
            | Self::MissingContext
            | Self::MissingNewline
            | Self::TwitchUserLookup
                => false,
            Self::Io(e) => e.is_network_error(),
            Self::Serenity(e) => match e {
                serenity::Error::Http(HttpError::Request(e)) => e.is_request() || e.is_connect() || e.is_timeout() || e.status().is_some_and(|status| status.is_server_error()),
                serenity::Error::Io(e) => e.is_network_error(),
                serenity::Error::Tungstenite(e) => e.is_network_error(),
                _ => false,
            },
            Self::Twitch(e) => match e {
                twitch_helix::Error::ExactlyOne(_) | twitch_helix::Error::InvalidHeaderValue(_) | twitch_helix::Error::ResponseJson(_, _) => false,
                twitch_helix::Error::HttpStatus(e, _) | twitch_helix::Error::Reqwest(e) => e.is_network_error(),
            },
            Self::Wheel(e) => e.is_network_error(),
        }
    }
}
