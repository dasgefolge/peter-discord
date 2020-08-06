#![deny(rust_2018_idioms, unused, unused_import_braces, unused_qualifications, warnings)]

use {
    std::{
        collections::{
            BTreeMap,
            HashMap
        },
        env,
        fs::File,
        iter,
        sync::Arc,
        thread,
        time::Duration
    },
    async_std::task::block_on,
    chrono::prelude::*,
    parking_lot::Condvar,
    serenity::{
        framework::standard::StandardFramework,
        model::prelude::*,
        prelude::*,
        utils::MessageBuilder
    },
    typemap::Key,
    peter::{
        Config,
        Error,
        GEFOLGE,
        ShardManagerContainer,
        commands,
        shut_down,
        twitch,
        user_list,
        voice::{
            self,
            VoiceStates
        },
        werewolf
    }
};

#[derive(Default)]
struct Handler(Arc<(Mutex<Option<Context>>, Condvar)>);

impl EventHandler for Handler {
    fn ready(&self, ctx: Context, ready: Ready) {
        let (ref ctx_arc, ref cond) = *self.0;
        let mut ctx_guard = ctx_arc.lock();
        *ctx_guard = Some(ctx.clone());
        cond.notify_all();
        let guilds = ready.user.guilds(&ctx).expect("failed to get guilds");
        if guilds.is_empty() {
            println!("[!!!!] No guilds found, use following URL to invite the bot:");
            println!("[ ** ] {}", ready.user.invite_url(&ctx, Permissions::all()).expect("failed to generate invite URL"));
            shut_down(&ctx);
        }
    }

    fn guild_ban_addition(&self, _: Context, guild_id: GuildId, user: User) {
        if guild_id != GEFOLGE { return; }
        user_list::remove(user).expect("failed to remove banned user from user list");
    }

    fn guild_ban_removal(&self, ctx: Context, guild_id: GuildId, user: User) {
        if guild_id != GEFOLGE { return; }
        user_list::add(guild_id.member(ctx, user).expect("failed to get unbanned guild member")).expect("failed to add unbanned user to user list");
    }

    fn guild_create(&self, ctx: Context, guild: Guild, _: bool) {
        if guild.id != GEFOLGE { return; }
        user_list::set(guild.members.values().cloned()).expect("failed to initialize user list");
        let mut chan_map = <VoiceStates as Key>::Value::default();
        for (user_id, voice_state) in guild.voice_states {
            if let Some(channel_id) = voice_state.channel_id {
                let user = user_id.to_user(&ctx).expect("failed to get user info");
                let (_, ref mut users) = chan_map.entry(channel_id)
                    .or_insert_with(|| (channel_id.name(&ctx).expect("failed to get channel name"), Vec::default()));
                match users.binary_search_by_key(&(user.name.clone(), user.discriminator), |user| (user.name.clone(), user.discriminator)) {
                    Ok(idx) => { users[idx] = user; }
                    Err(idx) => { users.insert(idx, user); }
                }
            }
        }
        let mut data = ctx.data.write();
        data.insert::<VoiceStates>(chan_map);
        let chan_map = data.get::<VoiceStates>().expect("missing voice states map");
        voice::dump_info(chan_map).expect("failed to update BitBar plugin");
    }

    fn guild_member_addition(&self, _: Context, guild_id: GuildId, member: Member) {
        if guild_id != GEFOLGE { return; }
        user_list::add(member).expect("failed to add new guild member to user list");
    }

    fn guild_member_removal(&self, _: Context, guild_id: GuildId, user: User, _: Option<Member>) {
        if guild_id != GEFOLGE { return; }
        user_list::remove(user).expect("failed to remove removed guild member from user list");
    }

    fn guild_member_update(&self, _: Context, _: Option<Member>, member: Member) {
        if member.guild_id != GEFOLGE { return; }
        user_list::update(member).expect("failed to update guild member info in user list");
    }

    fn guild_members_chunk(&self, _: Context, guild_id: GuildId, members: HashMap<UserId, Member>) {
        if guild_id != GEFOLGE { return; }
        for member in members.values() {
            user_list::add(member.clone()).expect("failed to add chunk of guild members to user list");
        }
    }

    fn message(&self, mut ctx: Context, msg: Message) {
        if msg.author.bot { return; } // ignore bots to prevent message loops
        if ctx.data.read().get::<Config>().expect("missing config").channels.werewolf.iter().any(|(_, conf)| conf.channel == msg.channel_id) {
            if let Some(action) = werewolf::parse_action(&mut ctx, msg.author.id, &msg.content) {
                match action.and_then(|action| werewolf::handle_action(&mut ctx, action)) {
                    Ok(()) => { msg.react(ctx, "👀").expect("reaction failed"); }
                    Err(Error::GameAction(err_msg)) => { msg.reply(ctx, &err_msg).expect("failed to reply to game action"); }
                    Err(e) => { panic!("failed to handle game action: {}", e); }
                }
            }
        }
    }

    fn voice_state_update(&self, ctx: Context, guild_id: Option<GuildId>, _old: Option<VoiceState>, new: VoiceState) {
        if guild_id.map_or(true, |gid| gid != GEFOLGE) { return; } //TODO make sure this works, i.e. serenity never passes None for GEFOLGE
        let user = new.user_id.to_user(&ctx).expect("failed to get user info");
        let mut data = ctx.data.write();
        let ignored_channels = data.get::<Config>().expect("missing config").channels.ignored.clone();
        let chan_map = data.get_mut::<VoiceStates>().expect("missing voice states map");
        let was_empty = chan_map.iter().all(|(channel_id, (_, members))| members.is_empty() || ignored_channels.contains(channel_id));
        let mut empty_channels = Vec::default();
        for (channel_id, (_, users)) in chan_map.iter_mut() {
            users.retain(|iter_user| iter_user.id != user.id);
            if users.is_empty() {
                empty_channels.push(*channel_id);
            }
        }
        for channel_id in empty_channels {
            chan_map.remove(&channel_id);
        }
        let chan_id = new.channel_id;
        if let Some(channel_id) = chan_id {
            let (_, ref mut users) = chan_map.entry(channel_id)
                .or_insert_with(|| (channel_id.name(&ctx).expect("failed to get channel name"), Vec::default()));
            match users.binary_search_by_key(&(user.name.clone(), user.discriminator), |user| (user.name.clone(), user.discriminator)) {
                Ok(idx) => { users[idx] = user.clone(); }
                Err(idx) => { users.insert(idx, user.clone()); }
            }
        }
        let is_empty = chan_map.iter().all(|(channel_id, (_, members))| members.is_empty() || ignored_channels.contains(channel_id));
        voice::dump_info(chan_map).expect("failed to update BitBar plugin");
        if was_empty && !is_empty {
            let config = data.get::<Config>().expect("missing config");
            let mut msg_builder = MessageBuilder::default();
            msg_builder.push("Discord Party? ");
            MessageBuilder::mention(&mut msg_builder, &user);
            msg_builder.push(" ist jetzt im voice channel ");
            msg_builder.mention(&chan_id.unwrap());
            config.channels.voice.say(&ctx, msg_builder).expect("failed to send channel message"); //TODO don't prefix channel name with `#`
        }
    }
}

fn main() -> Result<(), Error> {
    let mut args = env::args().peekable();
    let _ = args.next(); // ignore executable name
    if args.peek().is_some() {
        println!("{}", peter::ipc::send(args)?);
    } else {
        // read config
        let config = serde_json::from_reader::<_, Config>(File::open("/usr/local/share/fidera/config.json")?)?;
        let handler = Handler::default();
        let ctx_arc_ipc = handler.0.clone();
        let ctx_arc_twitch = handler.0.clone();
        let mut client = Client::new(&config.peter.bot_token, handler)?;
        let owners = iter::once(client.cache_and_http.http.get_current_application_info()?.owner.id).collect();
        {
            let mut data = client.data.write();
            data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
            data.insert::<Config>(config);
            data.insert::<VoiceStates>(BTreeMap::default());
            data.insert::<werewolf::GameState>(HashMap::default());
        }
        client.with_framework(StandardFramework::new()
            .configure(|c| c
                .with_whitespace(true) // allow ! command
                .case_insensitivity(true) // allow !Command
                .no_dm_prefix(true) // allow /msg @peter command (also allows game actions in DMs and “did not understand DM” error messages to work)
                .on_mention(Some(UserId(365936493539229699))) // allow @peter command
                .owners(owners)
                .prefix("!") // allow !command
            )
            .after(|_, _, command_name, result| {
                if let Err(why) = result {
                    println!("{}: Command '{}' returned error {:?}", Utc::now().format("%Y-%m-%d %H:%M:%S"), command_name, why);
                }
            })
            .unrecognised_command(|ctx, msg, _| {
                if msg.author.bot { return; } // ignore bots to prevent message loops
                if msg.is_private() {
                    if let Some(action) = werewolf::parse_action(ctx, msg.author.id, &msg.content) {
                        match action.and_then(|action| werewolf::handle_action(ctx, action)) {
                            Ok(()) => { msg.react(ctx, "👀").expect("reaction failed"); }
                            Err(Error::GameAction(err_msg)) => { msg.reply(ctx, &err_msg).expect("failed to reply to game action"); }
                            Err(e) => { panic!("failed to handle game action: {}", e); }
                        }
                    } else {
                        // reply when command isn't recognized
                        msg.reply(ctx, "ich habe diese Nachricht nicht verstanden").expect("failed to reply to unrecognized DM");
                    }
                }
            })
            //.help(help_commands::with_embeds) //TODO fix help?
            .group(&commands::GROUP)
        );
        // listen for IPC commands
        {
            thread::Builder::new().name("Peter IPC".into()).spawn(move || {
                if let Err(e) = peter::ipc::listen(ctx_arc_ipc.clone(), &|ctx, thread_kind, e| peter::notify_thread_crash(ctx, thread_kind, e.into())) { //TODO remove `if` after changing from `()` to `!`
                    eprintln!("{}", e);
                    peter::notify_thread_crash(&ctx_arc_ipc.0.lock(), "IPC", e.into());
                }
            })?;
        }
        // check Twitch stream status
        {
            thread::Builder::new().name("Peter Twitch".into()).spawn(move || {
                if let Err(e) = block_on(twitch::alerts(ctx_arc_twitch.clone())) { //TODO remove `if` after changing from `()` to `!`
                    eprintln!("{}", e);
                    peter::notify_thread_crash(&ctx_arc_twitch.0.lock(), "Twitch", e);
                }
            })?;
        }
        // connect to Discord
        client.start_autosharded()?;
        thread::sleep(Duration::from_secs(1)); // wait to make sure websockets can be closed cleanly
    }
    Ok(())
}
