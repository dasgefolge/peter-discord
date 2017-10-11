#![warn(trivial_casts)]
#![deny(unused)]
#![forbid(unused_extern_crates, unused_import_braces)]

extern crate peter;
extern crate serenity;

use std::process;
use std::collections::BTreeMap;

use serenity::model::{Guild, Permissions, Ready};
use serenity::prelude::*;

const LOGO: &str = "iVBORw0KGgoAAAANSUhEUgAAACYAAAAmCAYAAACoPemuAAAABGdBTUEAALGPC/xhBQAAACBjSFJNAAB6JgAAgIQAAPoAAACA6AAAdTAAAOpgAAA6mAAAF3CculE8AAAACXBIWXMAABYlAAAWJQFJUiTwAAABWWlUWHRYTUw6Y29tLmFkb2JlLnhtcAAAAAAAPHg6eG1wbWV0YSB4bWxuczp4PSJhZG9iZTpuczptZXRhLyIgeDp4bXB0az0iWE1QIENvcmUgNS40LjAiPgogICA8cmRmOlJERiB4bWxuczpyZGY9Imh0dHA6Ly93d3cudzMub3JnLzE5OTkvMDIvMjItcmRmLXN5bnRheC1ucyMiPgogICAgICA8cmRmOkRlc2NyaXB0aW9uIHJkZjphYm91dD0iIgogICAgICAgICAgICB4bWxuczp0aWZmPSJodHRwOi8vbnMuYWRvYmUuY29tL3RpZmYvMS4wLyI+CiAgICAgICAgIDx0aWZmOk9yaWVudGF0aW9uPjE8L3RpZmY6T3JpZW50YXRpb24+CiAgICAgIDwvcmRmOkRlc2NyaXB0aW9uPgogICA8L3JkZjpSREY+CjwveDp4bXBtZXRhPgpMwidZAAABvklEQVRYCe1WiXGDMBDEqSAlUIJLoASXQAnuQOnA6YB04BJSgkuwO0gJya3gmJUiJCHC4MxwM/I97J7Wh8Cuqt32CewTyJ7AUZAmG91jwVnF7tL1e1ivBTvciH8u4P+iaEMIU4OwqywVmvJGieJb4n1SPTvE6HVDCGkp13qJ/5I+MHjl1yjkGkh3WUaWNvhrX1NvCV07uKnNIGALc7S8bKEgZ09HpRC2mpZqHfU87cT+hbCTznNDj9eUtfGeSrb1+RokVVZTya38kA5v2iXDA/vIwE1CUi9QfWtzg06SKR7jNJ7Ccl2x1uf89jkESrgpxwRxQsaE4gujQwC/xniOfRxywwAvDuH9WlVyxrx91kn1qYTilCnWx01x5+K572HOxG7MHOLxvRO4FipNfYkQ1tb8exzLG2Fg4SmN4XCtk4X/cyYDy70E3m/CxWeIk4f/3Upf5+ORatsIIDYl8GPX517DEYDFjkKPkM9OVmyDZkRW1TmBDfW5Et8k+C1hbYhCqCnXIMq3oxSwGVY3+Ea8bycpcK9Q3PokzVPkiwILPKYWEqO1JqdnSCDOxFJTEeybJU3rJeSdu09gn8CMCfwATTVPuywMjbAAAAAASUVORK5CYII=";

struct Handler;

impl EventHandler for Handler {
    fn on_ready(&self, ctxt: Context, ready: Ready) {
        let guilds = ready.user.guilds().expect("failed to get guilds");
        if guilds.is_empty() {
            println!("Invite to guild|color=blue href={}", ready.user.invite_url(Permissions::all()).expect("failed to generate invite URL"));
            ctxt.quit().expect("failed to quit");
            process::exit(1);
        } else if guilds.len() > 1 {
            println!("multiple guilds found");
            ctxt.quit().expect("failed to quit");
            process::exit(1);
        }
    }

    fn on_guild_create(&self, ctxt: Context, guild: Guild, _: bool) {
        let mut total = 0;
        let mut chan_map = BTreeMap::default();
        for (user_id, voice_state) in guild.voice_states {
            if let Some(channel_id) = voice_state.channel_id {
                total += 1;
                let user = user_id.get().expect("failed to get user info");
                chan_map.entry(channel_id.name().expect("failed to get channel name"))
                    .or_insert_with(Vec::default)
                    .push(user);
            }
        }
        if total > 0 {
            println!("{}|templateImage={}", total, LOGO);
            for (channel_name, mut users) in chan_map {
                println!("---");
                println!("{}|size=10", channel_name);
                users.sort_by_key(|user| (user.name.clone(), user.discriminator));
                for user in users {
                    println!("{}#{:04}", user.name, user.discriminator);
                }
            }
        }
        ctxt.quit().expect("failed to quit");
        process::exit(0);
    }
}

fn main() {
    let mut client = Client::new(peter::TOKEN, Handler);
    client.start().expect("client error");
}
