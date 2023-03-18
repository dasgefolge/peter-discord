//! Some utilities for working with emoji (both Unicode and custom) and message reactions.

use {
    std::mem,
    discord_message_parser::{
        MessagePart,
        serenity::MessageExt as _,
    },
    serenity::model::prelude::*,
};

/// An iterator over all the emoji in a message.
///
/// Note that the `animated` field of yielded values is bogus and should not be relied upon.
pub struct Iter<'a>(MessagePart<'a>);

impl<'a> Iter<'a> {
    /// Create an iterator over all emoji in the given text.
    pub fn new(msg: &'a Message) -> Self {
        Self(msg.parse())
    }
}

fn message_part_next_emoji(part: &mut MessagePart<'_>) -> Option<ReactionType> {
    if let MessagePart::Nested(inner) = part {
        while !inner.is_empty() {
            let mut part = mem::replace(&mut inner[0], MessagePart::Empty);
            if let Some(emoji) = message_part_next_emoji(&mut part) {
                inner[0] = part;
                return Some(emoji)
            } else {
                inner.remove(0);
            }
        }
        return None
    }
    match mem::replace(part, MessagePart::Empty) {
        MessagePart::Nested(_) => unreachable!(),
        MessagePart::CustomEmoji(emoji) => {
            Some(emoji.into())
        }
        MessagePart::UnicodeEmoji(emoji) => {
            Some(ReactionType::Unicode(emoji.to_owned()))
        }
        _ => None,
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = ReactionType;

    fn next(&mut self) -> Option<ReactionType> {
        message_part_next_emoji(&mut self.0)
    }
}

/// Given a number in `0..26`, returns the regional indicator emoji corresponding to the letter in this position of the alphabet.
///
/// # Panics
///
/// Panics if the number is greater than 25.
pub fn nth_letter(n: u8) -> ReactionType {
    if n >= 26 { panic!("letter not in range") }
    ReactionType::Unicode(::std::char::from_u32('🇦' as u32 + n as u32).expect("failed to create regional indicator").to_string())
}
