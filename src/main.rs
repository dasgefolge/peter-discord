#![warn(trivial_casts)]
#![deny(unused)]
#![forbid(unused_import_braces)]

extern crate chrono;
extern crate peter;
extern crate serenity;
extern crate typemap;

use std::{process, thread};
use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::prelude::*;

use serenity::prelude::*;
use serenity::framework::standard::{StandardFramework, help_commands};
use serenity::model::{Guild, GuildId, Member, Message, Permissions, Ready, User, UserId, VoiceState};

use typemap::Key;

use peter::{bitbar, commands, user_list, werewolf};

struct Handler;

impl EventHandler for Handler {
    fn on_ready(&self, ctx: Context, ready: Ready) {
        let guilds = ready.user.guilds().expect("failed to get guilds");
        if guilds.is_empty() {
            println!("[!!!!] No guilds found, use following URL to invite the bot:");
            println!("[ ** ] {}", ready.user.invite_url(Permissions::all()).expect("failed to generate invite URL"));
            ctx.quit().expect("failed to quit");
            process::exit(1); //TODO (serenity 0.5.0) remove
        } else if guilds.len() > 1 {
            println!("[!!!!] Multiple guilds found");
            ctx.quit().expect("failed to quit");
            process::exit(1); //TODO (serenity 0.5.0) remove
        }
    }

    fn on_guild_ban_addition(&self, _: Context, _: GuildId, user: User) {
        user_list::remove(user).expect("failed to remove banned user from user list");
    }

    fn on_guild_ban_removal(&self, _: Context, guild_id: GuildId, user: User) {
        user_list::add(guild_id.member(user).expect("failed to get unbanned guild member")).expect("failed to add unbanned user to user list");
    }

    fn on_guild_create(&self, ctx: Context, guild: Guild, _: bool) {
        user_list::set(guild.members.values().cloned()).expect("failed to initialize user list");
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
        let mut data = ctx.data.lock();
        data.insert::<bitbar::VoiceStates>(chan_map);
        let chan_map = data.get::<bitbar::VoiceStates>().expect("missing voice states map");
        bitbar::dump_info(chan_map).expect("failed to update BitBar plugin");
    }

    fn on_guild_member_addition(&self, _: Context, _: GuildId, member: Member) {
        user_list::add(member).expect("failed to add new guild member to user list");
    }

    fn on_guild_member_removal(&self, _: Context, _: GuildId, user: User, _: Option<Member>) {
        user_list::remove(user).expect("failed to remove removed guild member from user list");
    }

    fn on_guild_members_chunk(&self, _: Context, _: GuildId, members: HashMap<UserId, Member>) {
        for member in members.values() {
            user_list::add(member.clone()).expect("failed to add chunk of guild members to user list");
        }
    }

    fn on_message(&self, mut ctx: Context, msg: Message) {
        if msg.author.bot { return; } // ignore bots to prevent message loops
        thread::Builder::new().name("peter message handler".into()).spawn(move || { //TODO (serenity 0.5.0) remove spawn wrapper
            if msg.channel_id == werewolf::TEXT_CHANNEL || msg.author.create_dm_channel().ok().map_or(false, |dm| dm.id == msg.channel_id) {
                if let Some(action) = werewolf::parse_action(&mut ctx, msg.author.id, &msg.content) {
                    match action.and_then(|action| werewolf::handle_action(&mut ctx, action)) {
                        Ok(()) => { msg.react("👀").expect("reaction failed"); }
                        Err(peter::Error::GameAction(err_msg)) => { msg.reply(&err_msg).expect("failed to reply to game action"); }
                        Err(e) => { panic!("failed to handle game action: {:?}", e); }
                    }
                }
            }
        }).expect("failed to spawn message handler thread");
    }

    fn on_voice_state_update(&self, ctx: Context, _: Option<GuildId>, voice_state: VoiceState) {
        let user = voice_state.user_id.get().expect("failed to get user info");
        let mut data = ctx.data.lock();
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
        data.insert::<werewolf::GameState>(werewolf::GameState::default());
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
        .after(|_, _, command_name, result| {
            if let Err(why) = result {
                println!("{}: Command '{}' returned error {:?}", Utc::now().format("%Y-%m-%d %H:%M:%S"), command_name, why);
            }
        })
        .command("help", |c| c
            .exec_help(help_commands::with_embeds)
        )
        .command("in", |c| c
            .check(|_, msg, _, _| msg.channel_id == werewolf::TEXT_CHANNEL)
            .exec(werewolf::command_in)
        )
        .command("out", |c| c
            .check(|_, msg, _, _| msg.channel_id == werewolf::TEXT_CHANNEL)
            .exec(werewolf::command_out)
        )
        .on("ping", commands::ping)
        .on("poll", commands::poll)
        .command("quit", |c| c
            .exec(commands::quit)
            .owners_only(true)
        )
        .command("test", |c| c
            .exec(commands::test)
            .owners_only(true)
        )
    );
    client.start().expect("client error");
}
