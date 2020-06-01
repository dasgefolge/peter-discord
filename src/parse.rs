//! Utilities for parsing messages into commands and game actions

use {
    std::{
        str::FromStr,
        sync::Arc
    },
    itertools::Itertools as _,
    serenity::{
        model::prelude::*,
        prelude::*
    }
};

/// Returns a role given its mention or name, but only if it's the entire command.
pub fn eat_role_full(cmd: &mut &str, guild: Option<Arc<RwLock<Guild>>>) -> Option<RoleId> {
    let original_cmd = *cmd;
    if let Some(role_id) = eat_role_mention(cmd) {
        if cmd.is_empty() {
            Some(role_id)
        } else {
            *cmd = original_cmd;
            None
        }
    } else if let Some(guild) = guild {
        guild.read()
            .roles
            .iter()
            .filter_map(|(&role_id, role)| if role.name == *cmd { Some(role_id) } else { None })
            .exactly_one()
            .ok()
    } else {
        None
    }
}

pub fn eat_role_mention(cmd: &mut &str) -> Option<RoleId> {
    if !cmd.starts_with('<') || !cmd.contains('>') {
        return None;
    }
    let mut maybe_mention = String::default();
    let mut chars = cmd.chars();
    while let Some(c) = chars.next() {
        maybe_mention.push(c);
        if c == '>' {
            if let Ok(id) = RoleId::from_str(&maybe_mention) {
                eat_word(cmd);
                return Some(id);
            }
            return None;
        }
    }
    None
}

#[allow(missing_docs)]
pub fn eat_user_mention(subj: &mut &str) -> Option<UserId> {
    if !subj.starts_with('<') || !subj.contains('>') {
        return None;
    }
    let mut maybe_mention = String::default();
    let mut chars = subj.chars();
    while let Some(c) = chars.next() {
        maybe_mention.push(c);
        if c == '>' {
            if let Ok(id) = UserId::from_str(&maybe_mention) {
                *subj = &subj[maybe_mention.len()..]; // consume mention text
                return Some(id);
            }
            return None;
        }
    }
    None
}

#[allow(missing_docs)]
pub fn eat_whitespace(subj: &mut &str) {
    while subj.starts_with(' ') {
        *subj = &subj[1..];
    }
}

fn eat_word(cmd: &mut &str) -> Option<String> {
    if let Some(word) = next_word(&cmd) {
        *cmd = &cmd[word.len()..];
        eat_whitespace(cmd);
        Some(word)
    } else {
        None
    }
}

#[allow(missing_docs)]
pub fn next_word(subj: &str) -> Option<String> {
    let mut word = String::default();
    for c in subj.chars() {
        if c == ' ' { break; }
        word.push(c);
    }
    if word.is_empty() { None } else { Some(word) }
}

/*
mention_parser!(channel_mention -> ChannelId);
mention_parser!(emoji_mention -> EmojiIdentifier);
mention_parser!(role_mention -> RoleId);
*/
