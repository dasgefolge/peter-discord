#![deny(rust_2018_idioms, unused, unused_import_braces, unused_lifetimes, unused_qualifications, warnings)]
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
};

pub mod commands;
pub mod config;
pub mod emoji;
pub mod ipc;
pub mod lang;
pub mod parse;
pub mod twitch;
pub mod user_list;
pub mod werewolf;

pub const GEFOLGE: GuildId = GuildId(355761290809180170);

pub const ADMIN: RoleId = RoleId(355776689051140099);
pub const QUIZMASTER: RoleId = RoleId(847443327069454378);
pub const MENSCH: RoleId = RoleId(386753710434287626);
pub const GUEST: RoleId = RoleId(784929665478557737);

pub const FENHL: UserId = UserId(86841168427495424);

/// `typemap` key for the PostgreSQL database connection.
pub struct Database;

impl TypeMapKey for Database {
    type Value = PgPool;
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)] ChannelIdParse(#[from] ChannelIdParseError),
    #[error(transparent)] Env(#[from] env::VarError),
    #[error(transparent)] Io(#[from] io::Error),
    #[error(transparent)] Ipc(#[from] crate::ipc::Error),
    #[error(transparent)] Json(#[from] serde_json::Error),
    #[error(transparent)] QwwStartGame(#[from] quantum_werewolf::game::state::StartGameError),
    #[error(transparent)] RoleIdParse(#[from] RoleIdParseError),
    #[error(transparent)] Serenity(#[from] serenity::Error),
    #[error(transparent)] Sql(#[from] sqlx::Error),
    #[error(transparent)] Twitch(#[from] twitch_helix::Error),
    #[error(transparent)] UserIdParse(#[from] UserIdParseError),
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
