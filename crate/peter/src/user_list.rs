//! Helper functions for maintaining the guild member list on disk, which is used by gefolge.org to verify logins.

use {
    std::{
        collections::BTreeSet,
        future::Future,
        io,
        path::Path,
        pin::Pin,
    },
    chrono::prelude::*,
    serde::{
        Deserialize,
        Serialize,
    },
    serenity::{
        model::prelude::*,
        prelude::*,
    },
    tokio::{
        fs::{
            self,
            File,
        },
        io::AsyncReadExt as _,
    },
    crate::{
        Error,
        GEFOLGE,
    },
};

const PROFILES_DIR: &'static str = "/usr/local/share/fidera/profiles";

#[derive(Deserialize, Serialize)]
struct Profile {
    bot: bool,
    discriminator: u16,
    joined: Option<DateTime<Utc>>,
    nick: Option<String>,
    roles: BTreeSet<RoleId>,
    snowflake: UserId,
    username: String,
}

/// Add a Discord account to the list of Gefolge guild members.
pub async fn add(member: &Member, join_date: Option<DateTime<Utc>>) -> Result<(), Error> {
    let buf = serde_json::to_vec_pretty(&Profile {
        bot: member.user.bot,
        discriminator: member.user.discriminator,
        joined: member.joined_at.map(|joined_at| *joined_at).or(join_date),
        nick: member.nick.clone(),
        roles: member.roles.iter().copied().collect(),
        snowflake: member.user.id,
        username: member.user.name.clone(),
    })?;
    fs::write(Path::new(PROFILES_DIR).join(format!("{}.json", member.user.id)), &buf).await?;
    Ok(())
}

/// Remove a Discord account from the list of Gefolge guild members.
pub async fn remove<U: Into<UserId>>(user: U) -> io::Result<Option<DateTime<Utc>>> {
    let join_date = match File::open(format!("{}/{}.json", PROFILES_DIR, user.into())).await {
        Ok(mut f) => {
            let mut buf = Vec::default();
            f.read_to_end(&mut buf).await?;
            serde_json::from_slice::<Profile>(&buf)?.joined
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(e),
    };
    /*
    match fs::remove_file(format!("{}/{}.json", PROFILES_DIR, user.into())).await {
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        r => r
    }
    */
    Ok(join_date)
}

pub enum Exporter {}

impl serenity_utils::handler::user_list::ExporterMethods for Exporter {
    fn upsert<'a>(_: &'a Context, member: &'a Member) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'a>> {
        Box::pin(async move {
            if member.guild_id != GEFOLGE { return Ok(()) }
            let join_date = remove(member).await?;
            add(member, join_date).await?;
            Ok(())
        })
    }

    fn replace_all<'a>(_: &'a Context, members: Vec<&'a Member>) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'a>> {
        Box::pin(async move {
            /*
            let mut read_dir = fs::read_dir(PROFILES_DIR).await?;
            while let Some(entry) = read_dir.try_next().await? {
                fs::remove_file(entry?.path()).await?;
            }
            */
            for member in members { //TODO parallel?
                if member.guild_id == GEFOLGE {
                    add(member, None).await?;
                }
            }
            Ok(())
        })
    }

    fn remove<'a>(_: &'a Context, UserId(user_id): UserId, guild_id: GuildId) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'a>> {
        Box::pin(async move {
            if guild_id != GEFOLGE { return Ok(()) }
            remove(user_id).await?;
            Ok(())
        })
    }
}
