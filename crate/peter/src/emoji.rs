//! Some utilities for working with emoji (both Unicode and custom) and message reactions.

use {
    std::{
        collections::BTreeSet,
        ffi::OsString,
        fmt,
        fs,
        io,
        mem,
        str::FromStr,
    },
    derive_more::From,
    once_cell::sync::Lazy,
    regex::Regex,
    serenity::model::prelude::*,
};

static FILENAME_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("^([0-9a-f]{1,6}(?:-[0-9a-f]{1,6})*)\\.svg$").expect("failed to compile twemoji filename regex"));
static CUSTOM_EMOJI_REGEX: Lazy<Regex> = Lazy::new(|| Regex::new("^<:[0-9A-Z_a-z]{2,}:[0-9]+>").expect("failed to compile custom emoji regex"));

/// An error that can occur while parsing emoji from a message.
#[derive(Debug, From)]
pub enum Error {
    /// An error occurred while decoding a filename.
    FilenameDecode(OsString),
    /// A `std::io::Error` occurred.
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::FilenameDecode(ref s) => write!(f, "failed to read twemoji filename: {:?}", s),
            Error::Io(ref e) => write!(f, "io error while building emoji db: {}", e),
        }
    }
}

impl std::error::Error for Error {}

/// An iterator over all the emoji in a message.
///
/// Note that the `animated` field of yielded values is bogus and should not be relied upon.
pub struct Iter {
    text: String,
    emoji: Vec<String>,
}

impl Iter {
    /// Create an iterator over all emoji in the given text.
    pub fn new(text: String) -> Result<Iter, Error> {
        let mut emoji = BTreeSet::default();
        for entry in fs::read_dir("/opt/git/github.com/twitter/twemoji/master/assets/svg")? {
            let file_name = entry?.file_name().into_string()?;
            if let Some(capture) = FILENAME_REGEX.captures(&file_name).and_then(|captures| captures.get(1)) {
                // convert the filename encoding the emoji (e.g. 1f3f3-fe0f-200d-1f308.svg) to the emoji itself (e.g. 🏳️‍🌈)
                emoji.insert(capture.as_str().split('-').filter_map(|hex| u32::from_str_radix(hex, 16).ok().and_then(::std::char::from_u32)).collect());
            }
        }
        Ok(Iter {
            text,
            emoji: emoji.into_iter().collect(),
        })
    }
}

impl Iterator for Iter {
    type Item = ReactionType;

    fn next(&mut self) -> Option<ReactionType> {
        let text = mem::replace(&mut self.text, String::default());
        let mut text = &text[..];
        loop {
            if let Some(captures) = CUSTOM_EMOJI_REGEX.captures(text) {
                let capture = captures.get(0).expect("failed to capture match object").as_str();
                if let Ok(emoji_id) = EmojiIdentifier::from_str(capture) {
                    self.text = text[capture.len()..].to_owned();
                    break Some(emoji_id.into())
                }
            }
            if let Some(emoji) = self.emoji.iter().rev().filter(|&emoji| text.starts_with(emoji)).next() { // longest emoji first
                self.text = text[emoji.len()..].to_owned();
                break Some(ReactionType::Unicode(emoji.to_owned()))
            }
            match text.char_indices().nth(1) {
                Some((idx, _)) => text = &text[idx..],
                None => break None,
            }
        }
    }
}

/// Given a number in `0..26`, returns the regional indicator emoji corresponding to the letter in this position of the alphabet.
pub fn nth_letter(n: u8) -> ReactionType {
    if n >= 26 { panic!("letter not in range") }
    ReactionType::Unicode(::std::char::from_u32('🇦' as u32 + n as u32).expect("failed to create regional indicator").to_string())
}
