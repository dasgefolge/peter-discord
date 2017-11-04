#![warn(trivial_casts)]
#![deny(unused)]
#![forbid(unused_extern_crates, unused_import_braces)]

extern crate peter;
extern crate rand;
extern crate serenity;
extern crate typemap;

use std::process;
use std::collections::{BTreeMap, HashSet};

use rand::{Rng, thread_rng};

use serenity::prelude::*;
use serenity::framework::StandardFramework;
use serenity::model::{Guild, GuildId, Permissions, Ready, VoiceState};

use typemap::Key;

use peter::bitbar;

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
    let owners = {
        let mut owners = HashSet::default();
        owners.insert(serenity::http::get_current_application_info().expect("couldn't get application info").owner.id);
        owners
    };
    client.with_framework(StandardFramework::new()
        .configure(|c| c
            .allow_whitespace(true) // allow ! command
            .case_insensitivity(true) // allow !Command
            .on_mention(true) // allow @peter command
            .owners(owners)
            .prefix("!") // allow !command
        )
        .on("ping", |_, msg, _| {
            let mut rng = thread_rng();
            msg.channel_id.say(if rng.gen_weighted_bool(1024) {
                format!("BWO{}{}G", "R".repeat(rng.gen_range(3, 20)), "N".repeat(rng.gen_range(1, 5))) // PINGCEPTION
            } else {
                "pong".to_owned()
            })?;
            Ok(())
        })
    );
    client.start().expect("client error");
}
