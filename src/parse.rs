//! Utilities for parsing messages into commands and game actions

use std::str::FromStr;

use serenity::model::UserId;

#[allow(missing_docs)]
pub fn user_mention(subj: &mut &str) -> Option<UserId> {
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

/*
mention_parser!(channel_mention -> ChannelId);
mention_parser!(emoji_mention -> EmojiIdentifier);
mention_parser!(role_mention -> RoleId);
*/
