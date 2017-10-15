#![warn(trivial_casts)]
#![forbid(unused, unused_extern_crates, unused_import_braces)]

extern crate peter;
extern crate serenity;

use std::process;
use std::collections::BTreeMap;

use peter::bitbar;

use serenity::prelude::*;
use serenity::model::{Guild, Permissions, Ready};

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

    fn on_guild_create(&self, _: Context, guild: Guild, _: bool) {
        let mut chan_map = BTreeMap::default();
        for (user_id, voice_state) in guild.voice_states {
            if let Some(channel_id) = voice_state.channel_id {
                let user = user_id.get().expect("failed to get user info");
                chan_map.entry(channel_id.name().expect("failed to get channel name"))
                    .or_insert_with(Vec::default)
                    .push(user);
            }
        }
        bitbar::dump_info(chan_map).expect("failed to update BitBar plugin");
    }
}

fn main() {
    let mut client = Client::new(peter::TOKEN, Handler);
    client.start().expect("client error");
}
