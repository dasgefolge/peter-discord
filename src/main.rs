#![warn(trivial_casts)]
#![forbid(unused, unused_extern_crates, unused_import_braces)]

extern crate peter;
extern crate serenity;
extern crate typemap;

use std::process;
use std::collections::BTreeMap;

use peter::bitbar;

use serenity::prelude::*;
use serenity::model::{Guild, GuildId, Permissions, Ready, VoiceState};

use typemap::Key;

struct Handler;

impl EventHandler for Handler {
    fn on_ready(&self, ctxt: Context, ready: Ready) {
        let guilds = ready.user.guilds().expect("failed to get guilds");
        if guilds.is_empty() {
            println!("[!!!!] No guilds found, use following URL to invite the bot:");
            println!("[ ** ] {}", ready.user.invite_url(Permissions::all()).expect("failed to generate invite URL"));
            ctxt.quit().expect("failed to quit");
            process::exit(1);
        } else if guilds.len() > 1 {
            println!("[!!!!] Multiple guilds found");
            ctxt.quit().expect("failed to quit");
            process::exit(1);
        }
    }

    fn on_guild_create(&self, ctxt: Context, guild: Guild, _: bool) {
        let mut chan_map = <bitbar::VoiceStates as Key>::Value::default();
        for (user_id, voice_state) in guild.voice_states {
            if let Some(channel_id) = voice_state.channel_id {
                let user = user_id.get().expect("failed to get user info");
                let users = chan_map.entry(channel_id.name().expect("failed to get channel name"))
                    .or_insert_with(Vec::default);
                match users.binary_search_by_key(&(user.name.clone(), user.discriminator), |user| (user.name.clone(), user.discriminator)) {
                    Ok(idx) => { users[idx] = user; }
                    Err(idx) => { users.insert(idx, user); }
                }
            }
        }
        let mut data = ctxt.data.lock();
        data.insert::<bitbar::VoiceStates>(chan_map);
        let chan_map = data.get_mut::<bitbar::VoiceStates>().expect("missing voice states map");
        bitbar::dump_info(chan_map).expect("failed to update BitBar plugin");
    }

    fn on_voice_state_update(&self, ctxt: Context, _: Option<GuildId>, voice_state: VoiceState) {
        let user = voice_state.user_id.get().expect("failed to get user info");
        let mut data = ctxt.data.lock();
        let chan_map = data.get_mut::<bitbar::VoiceStates>().expect("missing voice states map");
        let mut empty_channels = Vec::default();
        for (channel_name, users) in chan_map.iter_mut() {
            users.retain(|iter_user| iter_user.id != user.id);
            if users.is_empty() {
                empty_channels.push(channel_name.to_owned());
            }
        }
        for channel_name in empty_channels {
            chan_map.remove(&channel_name);
        }
        if let Some(channel_id) = voice_state.channel_id {
            let users = chan_map.entry(channel_id.name().expect("failed to get channel name"))
                .or_insert_with(Vec::default);
            match users.binary_search_by_key(&(user.name.clone(), user.discriminator), |user| (user.name.clone(), user.discriminator)) {
                Ok(idx) => { users[idx] = user; }
                Err(idx) => { users.insert(idx, user); }
            }
        }
        bitbar::dump_info(chan_map).expect("failed to update BitBar plugin");
    }
}

fn main() {
    let mut client = Client::new(peter::TOKEN, Handler);
    {
        let mut data = client.data.lock();
        data.insert::<bitbar::VoiceStates>(BTreeMap::default());
    }
    client.start().expect("client error");
}
