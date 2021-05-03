//! Helper functions for maintaining the guild member list on disk, which is used by gefolge.org to verify logins.

use {
    std::{
        collections::BTreeSet,
        io,
    },
    chrono::prelude::*,
    serde::{
        Deserialize,
        Serialize,
    },
    serenity::model::prelude::*,
    tokio::{
        fs::File,
        io::{
            AsyncReadExt as _,
            AsyncWriteExt as _,
        },
    },
    crate::Error,
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
pub async fn add(member: Member, join_date: Option<DateTime<Utc>>) -> Result<(), Error> {
    let mut f = File::create(format!("{}/{}.json", PROFILES_DIR, member.user.id)).await?;
    let buf = serde_json::to_vec_pretty(&Profile {
        bot: member.user.bot,
        discriminator: member.user.discriminator,
        joined: member.joined_at.or(join_date),
        nick: member.nick,
        roles: member.roles.into_iter().collect(),
        snowflake: member.user.id,
        username: member.user.name,
    })?;
    f.write_all(&buf).await?;
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

/// (Re)initialize the list of Gefolge guild members.
pub async fn set<I: IntoIterator<Item=Member>>(members: I) -> Result<(), Error> {
    /*
    let mut read_dir = fs::read_dir(PROFILES_DIR).await?;
    while let Some(entry) = read_dir.try_next().await? {
        fs::remove_file(entry?.path()).await?;
    }
    */
    for member in members.into_iter() { //TODO parallel?
        add(member, None).await?;
    }
    Ok(())
}

/// Update the data for a guild member. Equivalent to `remove` followed by `add`.
pub async fn update(member: Member) -> Result<(), Error> {
    let join_date = remove(&member).await?;
    add(member, join_date).await?;
    Ok(())
}
