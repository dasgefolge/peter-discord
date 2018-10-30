//! Helper functions for the BitBar plugin.

use std::{
    collections::BTreeMap,
    fs::File,
    io
};
use serde_json;
use serenity::model::user::User;
use typemap::Key;

/// `typemap` key for the voice state data required by the BitBar plugin: A mapping of voice channel names to users.
pub struct VoiceStates;

impl Key for VoiceStates {
    type Value = BTreeMap<String, Vec<User>>;
}

/// Takes a mapping from voice channel names to users and dumps the output for the gefolge.org API.
pub fn dump_info(voice_states: &<VoiceStates as Key>::Value) -> io::Result<()> {
    let f = File::create("/usr/local/share/fidera/discord/voice-state.json")?;
    serde_json::to_writer(f, &json!({
        "channels": voice_states.into_iter()
            .map(|(channel_name, members)| json!({
                "members": members.into_iter()
                    .map(|user| json!({
                        "discriminator": user.discriminator,
                        "snowflake": user.id,
                        "username": user.name
                    }))
                    .collect::<Vec<_>>(),
                "name": channel_name
            }))
            .collect::<Vec<_>>()
    }))?;
    Ok(())
}
