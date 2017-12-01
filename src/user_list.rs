//! Helper functions for maintaining the guild member list on disk, which is used by gefolge.org to verify logins.

use std::collections::BTreeSet;
use std::io::{self, BufReader};
use std::io::prelude::*;
use std::fs::{File, OpenOptions};
use std::str::FromStr;

use serenity::model::UserId;

const FILE_PATH: &'static str = "/usr/local/share/fidera/discord-snowflakes.txt";

/// Add a Discord account to the list of Gefolge guild members.
pub fn add(user_id: UserId) -> io::Result<()> {
    // check if the snowflake is already in the list
    let f = BufReader::new(File::open(FILE_PATH)?);
    if f.lines().collect::<Result<Vec<_>, _>>()?.into_iter().any(|line| UserId::from_str(&line).ok().map_or(false, |iter_id| iter_id == user_id)) { return Ok(()); }
    // it's not, so add it
    let mut f = OpenOptions::new().append(true).create(true).open(FILE_PATH)?;
    writeln!(f, "{}", user_id)?;
    Ok(())
}

/// Remove a Discord account from the list of Gefolge guild members.
pub fn remove(user_id: UserId) -> ::Result<()> {
    // check if the snowflake is still in the list
    let f = BufReader::new(File::open(FILE_PATH)?);
    let mut ids = f.lines().map(|line_result| line_result.map_err(Into::<::Error>::into).and_then(|line| UserId::from_str(&line).map_err(Into::<::Error>::into))).collect::<Result<BTreeSet<_>, _>>()?;
    if !ids.contains(&user_id) { return Ok(()); }
    // it is, so remove it
    ids.remove(&user_id);
    let mut f = File::create(FILE_PATH)?;
    for iter_id in ids {
        writeln!(f, "{}", iter_id)?;
    }
    Ok(())
}

/// (Re)initialize the list of Gefolge guild members.
pub fn set<I: IntoIterator<Item=UserId>>(user_ids: I) -> io::Result<()> {
    let ids = user_ids.into_iter().collect::<BTreeSet<_>>();
    let mut f = File::create(FILE_PATH)?;
    for iter_id in ids {
        writeln!(f, "{}", iter_id)?;
    }
    Ok(())
}
