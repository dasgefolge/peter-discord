//! Helper functions for maintaining the guild member list on disk, which is used by gefolge.org to verify logins.

use std::io;
use std::io::prelude::*;
use std::fs::{self, File};

use serenity::model::{Member, UserId};

const PROFILES_DIR: &'static str = "/usr/local/share/fidera/profiles";

/// Add a Discord account to the list of Gefolge guild members.
pub fn add(member: Member) -> ::Result<()> {
    let user = member.user.read().map_err(|_| ::std::sync::PoisonError::new(()))?.clone();
    let mut f = File::create(format!("{}/{}.json", PROFILES_DIR, user.id))?;
    write!(f, "{:#}", json!({
        "bot": user.bot,
        "discriminator": user.discriminator,
        "joined": if let Some(join_date) = member.joined_at { join_date } else { return Err(::Error::Unknown(())) },
        "nick": member.nick,
        "snowflake": user.id,
        "username": user.name
    }))?;
    Ok(())
}

/// Remove a Discord account from the list of Gefolge guild members.
pub fn remove<U: Into<UserId>>(user: U) -> io::Result<()> {
    match fs::remove_file(format!("{}/{}.json", PROFILES_DIR, user.into())) {
        Err(ref e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        r => r
    }
}

/// (Re)initialize the list of Gefolge guild members.
pub fn set<I: IntoIterator<Item=Member>>(members: I) -> ::Result<()> {
    for entry in fs::read_dir(PROFILES_DIR)? {
        fs::remove_file(entry?.path())?;
    }
    for member in members.into_iter() {
        add(member)?;
    }
    Ok(())
}
