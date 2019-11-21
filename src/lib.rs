//! The base library for the Gefolge Discord bot, Peter

#![deny(missing_docs, rust_2018_idioms, unused, unused_import_braces, unused_qualifications, warnings)]

use {
    std::{
        env,
        fmt,
        io::{
            self,
            BufReader,
            prelude::*
        },
        net::TcpStream,
        sync::Arc
    },
    derive_more::From,
    serenity::{
        client::bridge::gateway::ShardManager,
        model::prelude::*,
        prelude::*
    },
    typemap::Key
};

pub mod commands;
pub mod emoji;
pub mod lang;
pub mod parse;
pub mod user_list;
pub mod voice;
pub mod werewolf;

/// The Gefolge guild's ID.
pub const GEFOLGE: GuildId = GuildId(355761290809180170);

/// The address and port where the bot listens for IPC commands.
pub const IPC_ADDR: &str = "127.0.0.1:18807";

#[derive(Debug, From)]
#[allow(missing_docs)]
pub enum Error {
    Annotated(String, Box<Error>),
    ChannelIdParse(ChannelIdParseError),
    Env(env::VarError),
    #[from(ignore)]
    GameAction(String),
    Io(io::Error),
    Json(serde_json::Error),
    /// Returned if a Serenity context was required outside of an event handler but the `ready` event has not been received yet.
    MissingContext,
    /// Returned by the user list handler if a user has no join date.
    MissingJoinDate,
    /// The reply to an IPC command did not end in a newline.
    MissingNewline,
    QwwStartGame(quantum_werewolf::game::state::StartGameError),
    RoleIdParse(RoleIdParseError),
    Serenity(serenity::Error),
    /// Returned from `listen_ipc` if a command line was not valid shell lexer tokens.
    #[from(ignore)]
    Shlex(String),
    /// Returned from `listen_ipc` if an unknown command is received.
    #[from(ignore)]
    UnknownCommand(Vec<String>),
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

impl<T, E: IntoResultExt> IntoResultExt for std::result::Result<T, E> {
    type T = std::result::Result<T, E::T>;

    fn annotate(self, note: impl ToString) -> std::result::Result<T, E::T> {
        self.map_err(|e| e.annotate(note))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::Annotated(ref msg, ref e) => write!(f, "{}: {}", msg, e),
            Error::ChannelIdParse(ref e) => e.fmt(f),
            Error::Env(ref e) => e.fmt(f),
            Error::GameAction(ref s) => write!(f, "invalid game action: {}", s),
            Error::Io(ref e) => e.fmt(f),
            Error::Json(ref e) => e.fmt(f),
            Error::MissingContext => write!(f, "Serenity context not available before ready event"),
            Error::MissingJoinDate => write!(f, "encountered user without join date"),
            Error::MissingNewline => write!(f, "the reply to an IPC command did not end in a newline"),
            Error::QwwStartGame(ref e) => e.fmt(f),
            Error::RoleIdParse(ref e) => e.fmt(f),
            Error::Serenity(ref e) => e.fmt(f),
            Error::Shlex(ref e) => write!(f, "failed to parse IPC command line: {}", e),
            Error::UnknownCommand(ref args) => write!(f, "unknown command: {:?}", args),
            Error::UserIdParse(ref e) => e.fmt(f)
        }
    }
}

#[allow(missing_docs)]
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// `typemap` key for the serenity shard manager.
pub struct ShardManagerContainer;

impl Key for ShardManagerContainer {
    type Value = Arc<Mutex<ShardManager>>;
}

/// Sends an IPC command to the bot.
///
/// **TODO:** document available IPC commands
pub fn send_ipc_command<T: fmt::Display, I: IntoIterator<Item = T>>(cmd: I) -> Result<String, Error> {
    let mut stream = TcpStream::connect(IPC_ADDR)?;
    writeln!(&mut stream, "{}", cmd.into_iter().map(|arg| shlex::quote(&arg.to_string()).into_owned()).collect::<Vec<_>>().join(" "))?;
    let mut buf = String::default();
    BufReader::new(stream).read_line(&mut buf)?;
    if buf.pop() != Some('\n') { return Err(Error::MissingNewline); }
    Ok(buf)
}

/// Utility function to shut down all shards.
pub fn shut_down(ctx: &Context) {
    ctx.invisible(); // hack to prevent the bot showing as online when it's not
    let data = ctx.data.read();
    let mut shard_manager = data.get::<ShardManagerContainer>().expect("missing shard manager").lock();
    shard_manager.shutdown_all();
}
