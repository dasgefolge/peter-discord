#![deny(rust_2018_idioms, unused, unused_import_braces, unused_qualifications, warnings)]

use {
    std::{
        collections::{
            BTreeMap,
            BTreeSet
        },
        env,
        fmt,
        io::{
            self,
            prelude::*
        },
        process::{
            Command,
            Stdio
        },
        sync::Arc
    },
    derive_more::From,
    serde::Deserialize,
    serenity::{
        client::bridge::gateway::ShardManager,
        model::prelude::*,
        prelude::*
    },
    typemap::Key
};

pub mod commands;
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
    /// Returned by the user list handler if a user has no join date.
    MissingJoinDate,
    /// The reply to an IPC command did not end in a newline.
    MissingNewline,
    QwwStartGame(quantum_werewolf::game::state::StartGameError),
    RoleIdParse(RoleIdParseError),
    Serenity(serenity::Error),
    Twitch(twitch_helix::Error),
    UserIdParse(UserIdParseError)
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
            Error::MissingJoinDate => write!(f, "encountered user without join date"),
            Error::MissingNewline => write!(f, "the reply to an IPC command did not end in a newline"),
            Error::QwwStartGame(e) => e.fmt(f),
            Error::RoleIdParse(e) => e.fmt(f),
            Error::Serenity(e) => e.fmt(f),
            Error::Twitch(e) => e.fmt(f),
            Error::UserIdParse(e) => e.fmt(f)
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub channels: ConfigChannels,
    pub peter: ConfigPeter,
    twitch: twitch::Config
}

impl Key for Config {
    type Value = Config;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigChannels {
    pub ignored: BTreeSet<ChannelId>,
    pub voice: ChannelId,
    pub werewolf: BTreeMap<GuildId, werewolf::Config>
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigPeter {
    pub bot_token: String,
    self_assignable_roles: BTreeSet<RoleId>
}

/// `typemap` key for the serenity shard manager.
pub struct ShardManagerContainer;

impl Key for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

pub fn notify_thread_crash(ctx: &Option<Context>, thread_kind: &str, e: Error) {
    if ctx.as_ref().and_then(|ctx| FENHL.to_user(ctx).and_then(|fenhl| fenhl.dm(ctx, |m| m.content(format!("{} thread crashed: {} (`{:?}`)", thread_kind, e, e)))).ok()).is_none() {
        let mut child = Command::new("mail")
            .arg("-s")
            .arg(format!("Peter {} thread crashed", thread_kind))
            .arg("fenhl@fenhl.net")
            .stdin(Stdio::piped())
            .spawn()
            .expect("failed to spawn mail");
        {
            let stdin = child.stdin.as_mut().expect("failed to open mail stdin");
            write!(stdin, "Peter {} thread crashed with the following error:\n{}\n{:?}\n", thread_kind, e, e).expect("failed to write to mail stdin");
        }
        child.wait().expect("failed to wait for mail subprocess"); //TODO check exit status
    }
}

/// Utility function to shut down all shards.
pub fn shut_down(ctx: &Context) {
    ctx.invisible(); // hack to prevent the bot showing as online when it's not
    let data = ctx.data.read();
    let mut shard_manager = data.get::<ShardManagerContainer>().expect("missing shard manager").lock();
    shard_manager.shutdown_all();
}
