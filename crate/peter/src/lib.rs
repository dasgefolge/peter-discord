#![deny(rust_2018_idioms, unused, unused_import_braces, unused_lifetimes, unused_qualifications, warnings)]

use {
    std::{
        env,
        fmt,
        io,
        process::Stdio,
        time::Duration,
    },
    derive_more::From,
    serenity::{
        model::prelude::*,
        prelude::*,
    },
    serenity_utils::RwFuture,
    tokio::{
        io::AsyncWriteExt as _,
        process::Command,
    },
};

pub mod commands;
pub mod config;
pub mod emoji;
pub mod ipc;
pub mod lang;
pub mod parse;
pub mod twitch;
pub mod user_list;
pub mod voice;
pub mod werewolf;

const FENHL: UserId = UserId(86841168427495424);
pub const GEFOLGE: GuildId = GuildId(355761290809180170);

#[derive(Debug, From)]
pub enum Error {
    Annotated(String, Box<Error>),
    ChannelIdParse(ChannelIdParseError),
    Env(env::VarError),
    #[from(ignore)]
    GameAction(String),
    Io(io::Error),
    Ipc(crate::ipc::Error),
    Json(serde_json::Error),
    /// Returned if the config is not present in Serenity context.
    MissingConfig,
    /// Returned if a Serenity context was required outside of an event handler but the `ready` event has not been received yet.
    MissingContext,
    /// The reply to an IPC command did not end in a newline.
    MissingNewline,
    QwwStartGame(quantum_werewolf::game::state::StartGameError),
    RoleIdParse(RoleIdParseError),
    Serenity(serenity::Error),
    Twitch(twitch_helix::Error),
    TwitchUserLookup,
    UserIdParse(UserIdParseError),
}

/// A helper trait for annotating errors with more informative error messages.
pub trait IntoResultExt {
    /// The return type of the `annotate` method.
    type T;

    /// Annotates an error with an additional message which is displayed along with the error.
    fn annotate(self, note: impl ToString) -> Self::T;
}

impl<E: Into<Error>> IntoResultExt for E {
    type T = Error;

    fn annotate(self, note: impl ToString) -> Error {
        Error::Annotated(note.to_string(), Box::new(self.into()))
    }
}

impl<T, E: IntoResultExt> IntoResultExt for Result<T, E> {
    type T = Result<T, E::T>;

    fn annotate(self, note: impl ToString) -> Result<T, E::T> {
        self.map_err(|e| e.annotate(note))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Annotated(msg, e) => write!(f, "{}: {}", msg, e),
            Error::ChannelIdParse(e) => e.fmt(f),
            Error::Env(e) => e.fmt(f),
            Error::GameAction(s) => write!(f, "invalid game action: {}", s),
            Error::Io(e) => e.fmt(f),
            Error::Ipc(e) => e.fmt(f),
            Error::Json(e) => e.fmt(f),
            Error::MissingConfig => write!(f, "config missing in Serenity context"),
            Error::MissingContext => write!(f, "Serenity context not available before ready event"),
            Error::MissingNewline => write!(f, "the reply to an IPC command did not end in a newline"),
            Error::QwwStartGame(e) => e.fmt(f),
            Error::RoleIdParse(e) => e.fmt(f),
            Error::Serenity(e) => e.fmt(f),
            Error::Twitch(e) => e.fmt(f),
            Error::TwitchUserLookup => write!(f, "Twitch returned unexpected user info"),
            Error::UserIdParse(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for Error {}

pub async fn notify_thread_crash(ctx: RwFuture<Context>, thread_kind: String, e: impl Into<Error>, auto_retry: Option<Duration>) {
    let ctx = ctx.read().await;
    let e = e.into();
    if let Ok(fenhl) = FENHL.to_user(&*ctx).await {
        if fenhl.dm(&*ctx, |m| m.content(format!("{} thread crashed: {} (`{:?}`), {}", thread_kind, e, e, if let Some(auto_retry) = auto_retry { format!("auto-retrying in `{:?}`", auto_retry) } else { format!("**not** auto-retrying") }))).await.is_ok() {
            return
        }
    }
    let mut child = Command::new("mail")
        .arg("-s")
        .arg(format!("Peter {} thread crashed", thread_kind))
        .arg("fenhl@fenhl.net")
        .stdin(Stdio::piped())
        .spawn()
        .expect("failed to spawn mail");
    {
        let input = format!("Peter {} thread crashed with the following error:\n{}\n{:?}\n", thread_kind, e, e).into_bytes();
        let stdin = child.stdin.as_mut().expect("failed to open mail stdin");
        stdin.write_all(&input).await.expect("failed to write to mail stdin");
    }
    let exit_status = child.wait().await.expect("failed to wait for mail subprocess");
    if !exit_status.success() {
        panic!("mail exited with {} while notifying thread crash", exit_status)
    }
}
