//! Helper functions for maintaining the guild member list on disk, which is used by gefolge.org to verify logins.

use {
    std::{
        fs::File,
        io::{
            self,
            prelude::*
        }
    },
    serde_json::json,
    serenity::model::prelude::*,
    crate::Error
};

const PROFILES_DIR: &'static str = "/usr/local/share/fidera/profiles";

/// Add a Discord account to the list of Gefolge guild members.
pub fn add(member: Member) -> Result<(), Error> {
    let user = member.user.read().clone();
    let mut f = File::create(format!("{}/{}.json", PROFILES_DIR, user.id))?;
    write!(f, "{:#}", json!({
        "bot": user.bot,
        "discriminator": user.discriminator,
        "joined": if let Some(join_date) = member.joined_at { join_date } else { return Err(Error::MissingJoinDate) },
        "nick": member.nick,
        "roles": member.roles,
        "snowflake": user.id,
        "username": user.name
    }))?;
    Ok(())
}

/// Remove a Discord account from the list of Gefolge guild members.
pub fn remove<U: Into<UserId>>(_ /*user*/: U) -> io::Result<()> {
    /*
    match fs::remove_file(format!("{}/{}.json", PROFILES_DIR, user.into())) {
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        r => r
    }
    */
    Ok(())
}

/// (Re)initialize the list of Gefolge guild members.
pub fn set<I: IntoIterator<Item=Member>>(members: I) -> Result<(), Error> {
    /*
    for entry in fs::read_dir(PROFILES_DIR)? {
        fs::remove_file(entry?.path())?;
    }
    */
    for member in members.into_iter() {
        add(member)?;
    }
    Ok(())
}

/// Update the data for a guild member. Equivalent to `remove` followed by `add`.
pub fn update(member: Member) -> Result<(), Error> {
    remove(&member)?;
    add(member)?;
    Ok(())
}
