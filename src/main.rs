#![warn(trivial_casts)]
#![deny(unused)]
#![forbid(unused_import_braces)]

extern crate chrono;
extern crate peter;
extern crate serenity;
extern crate shlex;
extern crate typemap;

use std::{
    collections::{
        BTreeMap,
        HashMap,
        HashSet
    },
    env,
    io::prelude::*,
    iter,
    net::TcpListener,
    sync::Arc,
    thread
};

use chrono::prelude::*;

use serenity::{
    framework::standard::{
        StandardFramework,
        help_commands
    },
    model::{
        channel::Message,
        gateway::Ready,
        guild::{
            Guild,
            Member
        },
        id::{
            GuildId,
            RoleId,
            UserId
        },
        permissions::Permissions,
        user::User,
        voice::VoiceState
    },
    prelude::*
};

use typemap::Key;

use peter::{
    GEFOLGE,
    ShardManagerContainer,
    bitbar,
    commands,
    shut_down,
    user_list,
    werewolf
};

struct Handler;

impl EventHandler for Handler {
    fn ready(&self, ctx: Context, ready: Ready) {
        let guilds = ready.user.guilds().expect("failed to get guilds");
        if guilds.is_empty() {
            println!("[!!!!] No guilds found, use following URL to invite the bot:");
            println!("[ ** ] {}", ready.user.invite_url(Permissions::all()).expect("failed to generate invite URL"));
            shut_down(&ctx);
        } else if guilds.len() > 1 {
            println!("[!!!!] Multiple guilds found");
            shut_down(&ctx);
        }
    }

    fn guild_ban_addition(&self, _: Context, _: GuildId, user: User) {
        user_list::remove(user).expect("failed to remove banned user from user list");
    }

    fn guild_ban_removal(&self, _: Context, guild_id: GuildId, user: User) {
        user_list::add(guild_id.member(user).expect("failed to get unbanned guild member")).expect("failed to add unbanned user to user list");
    }

    fn guild_create(&self, ctx: Context, guild: Guild, _: bool) {
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

    fn guild_member_addition(&self, _: Context, _: GuildId, member: Member) {
        user_list::add(member).expect("failed to add new guild member to user list");
    }

    fn guild_member_removal(&self, _: Context, _: GuildId, user: User, _: Option<Member>) {
        user_list::remove(user).expect("failed to remove removed guild member from user list");
    }

    fn guild_member_update(&self, _: Context, _: Option<Member>, member: Member) {
        user_list::update(member).expect("failed to update guild member info in user list");
    }

    fn guild_members_chunk(&self, _: Context, _: GuildId, members: HashMap<UserId, Member>) {
        for member in members.values() {
            user_list::add(member.clone()).expect("failed to add chunk of guild members to user list");
        }
    }

    fn message(&self, mut ctx: Context, msg: Message) {
        if msg.author.bot { return; } // ignore bots to prevent message loops
        if msg.channel_id == werewolf::TEXT_CHANNEL || msg.author.create_dm_channel().ok().map_or(false, |dm| dm.id == msg.channel_id) {
            if let Some(action) = werewolf::parse_action(&mut ctx, msg.author.id, &msg.content) {
                match action.and_then(|action| werewolf::handle_action(&mut ctx, action)) {
                    Ok(()) => { msg.react("👀").expect("reaction failed"); }
                    Err(peter::Error::GameAction(err_msg)) => { msg.reply(&err_msg).expect("failed to reply to game action"); }
                    Err(e) => { panic!("failed to handle game action: {:?}", e); }
                }
            }
        }
    }

    fn voice_state_update(&self, ctx: Context, _: Option<GuildId>, voice_state: VoiceState) {
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

fn main() -> Result<(), peter::Error> {
    // read config
    let token = env::var("DISCORD_TOKEN")?;
    let mut client = Client::new(&token, Handler)?;
    let owners = {
        let mut owners = HashSet::default();
        owners.insert(serenity::http::get_current_application_info()?.owner.id);
        owners
    };
    {
        let mut data = client.data.lock();
        data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
        data.insert::<bitbar::VoiceStates>(BTreeMap::default());
        data.insert::<werewolf::GameState>(werewolf::GameState::default());
    }
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
        .help(help_commands::with_embeds)
        .command("in", |c| c
            .check(|_, msg, _, _| msg.channel_id == werewolf::TEXT_CHANNEL)
            .exec(werewolf::command_in)
        )
        .command("out", |c| c
            .check(|_, msg, _, _| msg.channel_id == werewolf::TEXT_CHANNEL)
            .exec(werewolf::command_out)
        )
        .cmd("ping", commands::ping)
        .cmd("poll", commands::poll)
        .cmd("quit", commands::Quit)
        .cmd("test", commands::Test)
    );
    // listen for IPC commands
    {
        thread::spawn(move || -> Result<(), _> { //TODO change to Result<!, _>
            for stream in TcpListener::bind("127.0.0.1:18807")?.incoming() {
                let mut stream = stream?;
                let mut buf = String::default();
                stream.read_to_string(&mut buf)?;
                let args = shlex::split(&buf).ok_or(peter::Error::Unknown(()))?;
                match &args[0][..] {
                    "add-role" => {
                        let user = args[1].parse::<UserId>()?;
                        let role = args[2].parse::<RoleId>()?;
                        let roles = iter::once(role).chain(GEFOLGE.member(user)?.roles.into_iter());
                        GEFOLGE.edit_member(user, |m| m.roles(roles))?;
                    }
                    "msg" => {
                        let rcpt = args[1].parse::<UserId>()?;
                        rcpt.create_dm_channel()?.say(&args[2])?;
                    }
                    _ => { return Err(peter::Error::UnknownCommand(args)); }
                }
            }
            unreachable!();
        });
    }
    // connect to Discord
    client.start_autosharded()?;
    Ok(())
}
