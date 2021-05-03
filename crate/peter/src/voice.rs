//! Helper functions for the BitBar plugin.

use {
    std::{
        collections::BTreeMap,
        io,
    },
    serde_json::{
        self,
        json,
    },
    serenity::{
        model::prelude::*,
        prelude::*,
    },
    tokio::{
        fs::File,
        io::AsyncWriteExt as _,
    },
};

/// `typemap` key for the voice state data required by the gefolge.org API: A mapping of voice channel names to users.
#[derive(Default)]
pub struct VoiceStates(pub BTreeMap<ChannelId, (String, Vec<User>)>);

impl TypeMapKey for VoiceStates {
    type Value = VoiceStates;
}

/// Takes a mapping from voice channel names to users and dumps the output for the gefolge.org API.
pub async fn dump_info(VoiceStates(voice_states): &VoiceStates) -> io::Result<()> {
    let mut f = File::create("/usr/local/share/fidera/discord/voice-state.json").await?;
    let buf = serde_json::to_vec(&json!({
        "channels": voice_states.into_iter()
            .map(|(channel_id, (channel_name, members))| json!({
                "members": members.into_iter()
                    .map(|user| json!({
                        "discriminator": user.discriminator,
                        "snowflake": user.id,
                        "username": user.name
                    }))
                    .collect::<Vec<_>>(),
                "name": channel_name,
                "snowflake": channel_id
            }))
            .collect::<Vec<_>>()
    }))?;
    f.write_all(&buf).await?;
    Ok(())
}
