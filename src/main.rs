#![deny(rust_2018_idioms, unused, unused_import_braces, unused_qualifications, warnings)]

use {
    std::{
        collections::{
            BTreeMap,
            HashMap
        },
        env,
        fs::File,
        io::{
            self,
            BufReader,
            prelude::*
        },
        iter,
        net::{
            TcpListener,
            TcpStream
        },
        process::{
            Command,
            Stdio
        },
        sync::Arc,
        thread,
        time::Duration
    },
    chrono::prelude::*,
    serde::Deserialize,
    serenity::{
        framework::standard::StandardFramework,
        model::prelude::*,
        prelude::*,
        utils::MessageBuilder
    },
    typemap::Key,
    peter::{
        GEFOLGE,
        Error,
        IntoResultExt as _,
        Result,
        ShardManagerContainer,
        commands,
        shut_down,
        user_list,
        voice::{
            self,
            VoiceStates
        },
        werewolf
    }
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Config {
    channels: ConfigChannels,
    peter: ConfigPeter
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigChannels {
    voice: ChannelId
}

impl Key for ConfigChannels {
    type Value = ConfigChannels;
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ConfigPeter {
    bot_token: String
}

#[derive(Default)]
struct Handler(Arc<Mutex<Option<Context>>>);

impl EventHandler for Handler {
    fn ready(&self, ctx: Context, ready: Ready) {
        *self.0.lock() = Some(ctx.clone());
        let guilds = ready.user.guilds(&ctx).expect("failed to get guilds");
        if guilds.is_empty() {
            println!("[!!!!] No guilds found, use following URL to invite the bot:");
            println!("[ ** ] {}", ready.user.invite_url(&ctx, Permissions::all()).expect("failed to generate invite URL"));
            shut_down(&ctx);
        } else if guilds.len() > 1 {
            println!("[!!!!] Multiple guilds found");
            shut_down(&ctx);
        }
    }

    fn guild_ban_addition(&self, _: Context, _: GuildId, user: User) {
        user_list::remove(user).expect("failed to remove banned user from user list");
    }

    fn guild_ban_removal(&self, ctx: Context, guild_id: GuildId, user: User) {
        user_list::add(guild_id.member(ctx, user).expect("failed to get unbanned guild member")).expect("failed to add unbanned user to user list");
    }

    fn guild_create(&self, ctx: Context, guild: Guild, _: bool) {
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
        if msg.channel_id == werewolf::TEXT_CHANNEL {
            if let Some(action) = werewolf::parse_action(&mut ctx, msg.author.id, &msg.content) {
                match action.and_then(|action| werewolf::handle_action(&mut ctx, action)) {
                    Ok(()) => { msg.react(ctx, "👀").expect("reaction failed"); }
                    Err(Error::GameAction(err_msg)) => { msg.reply(ctx, &err_msg).expect("failed to reply to game action"); }
                    Err(e) => { panic!("failed to handle game action: {}", e); }
                }
            }
        }
    }

    fn voice_state_update(&self, ctx: Context, _: Option<GuildId>, _old: Option<VoiceState>, new: VoiceState) {
        let user = new.user_id.to_user(&ctx).expect("failed to get user info");
        let mut data = ctx.data.write();
        let chan_map = data.get_mut::<VoiceStates>().expect("missing voice states map");
        let was_empty = chan_map.iter().all(|(channel_id, (_, members))| *channel_id == voice::BIBLIOTHEK || members.is_empty());
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
        let is_empty = chan_map.iter().all(|(channel_id, (_, members))| *channel_id == voice::BIBLIOTHEK || members.is_empty());
        voice::dump_info(chan_map).expect("failed to update BitBar plugin");
        if was_empty && !is_empty {
            let channel_config = data.get::<ConfigChannels>().expect("missing channels config");
            channel_config.voice.say(&ctx, MessageBuilder::default().push("Discord Party? ").mention(&user).push(" ist jetzt im voice channel ").mention(&chan_id.unwrap())).expect("failed to send channel message"); //TODO
        }
    }
}

fn listen_ipc(ctx_arc: Arc<Mutex<Option<Context>>>) -> Result<()> { //TODO change return type to Result<!>
    for stream in TcpListener::bind(peter::IPC_ADDR).annotate("IPC listener")?.incoming() {
        if let Err(e) = stream.map_err(|e| e.annotate("incoming stream")).and_then(|stream| handle_ipc_client(&ctx_arc, stream)) {
            notify_ipc_crash(e);
        }
    }
    unreachable!();
}

fn handle_ipc_client(ctx_arc: &Mutex<Option<Context>>, stream: TcpStream) -> Result<()> {
    let mut last_error = Ok(());
    let mut buf = String::default();
    for line in BufReader::new(&stream).lines() {
        let line = match line {
            Ok(line) => line,
            Err(e) => if e.kind() == io::ErrorKind::ConnectionReset {
                break; // connection reset by peer, consider the IPC session terminated
            } else {
                return Err(e.annotate("IPC client line"));
            }
        };
        buf.push_str(&line);
        let args = match shlex::split(&buf) {
            Ok(args) => {
                last_error = Ok(());
                buf.clear();
                args
            }
            Err(e) => {
                last_error = Err(Error::Shlex(e, line));
                buf.push('\n');
                continue;
            }
        };
        match &args[0][..] {
            "add-role" => {
                let ctx_guard = ctx_arc.lock();
                let ctx = ctx_guard.as_ref().ok_or(Error::MissingContext)?;
                let user = args[1].parse::<UserId>().annotate("failed to parse user snowflake")?;
                let role = args[2].parse::<RoleId>().annotate("failed to parse role snowflake")?;
                let roles = iter::once(role).chain(GEFOLGE.member(ctx, user).annotate("failed to get member data")?.roles.into_iter());
                GEFOLGE.edit_member(ctx, user, |m| m.roles(roles)).annotate("failed to edit roles")?;
                writeln!(&mut &stream, "role added")?;
            }
            "channel-msg" => {
                let ctx_guard = ctx_arc.lock();
                let ctx = ctx_guard.as_ref().ok_or(Error::MissingContext)?;
                let channel = args[1].parse::<ChannelId>().annotate("failed to parse channel snowflake")?;
                channel.say(ctx, &args[2]).annotate("failed to send channel message")?;
                writeln!(&mut &stream, "message sent")?;
            }
            "msg" => {
                let ctx_guard = ctx_arc.lock();
                let ctx = ctx_guard.as_ref().ok_or(Error::MissingContext)?;
                let rcpt = args[1].parse::<UserId>().annotate("failed to parse user snowflake")?;
                rcpt.create_dm_channel(ctx).annotate("failed to get/create DM channel")?.say(ctx, &args[2]).annotate("failed to send DM")?;
                writeln!(&mut &stream, "message sent")?;
            }
            "quit" => {
                let ctx_guard = ctx_arc.lock();
                let ctx = ctx_guard.as_ref().ok_or(Error::MissingContext)?;
                shut_down(&ctx);
                thread::sleep(Duration::from_secs(1)); // wait to make sure websockets can be closed cleanly
                writeln!(&mut &stream, "shutdown complete")?;
            }
            "set-display-name" => {
                let ctx_guard = ctx_arc.lock();
                let ctx = ctx_guard.as_ref().ok_or(Error::MissingContext)?;
                let user = args[1].parse::<UserId>().annotate("failed to parse user for set-display-name")?.to_user(ctx).annotate("failed to get user for set-display-name")?;
                let new_display_name = &args[2];
                match GEFOLGE.edit_member(ctx, &user, |e| e.nickname(new_display_name)) {
                    Ok(()) => {
                        writeln!(&mut &stream, "display name set").annotate("failed to send set-display-name confirmation")?;
                    }
                    Err(serenity::Error::Http(e)) => if let HttpError::UnsuccessfulRequest(response) = *e {
                        writeln!(&mut &stream, "failed to set display name: {:?}", response)?;
                    } else {
                        //TODO use box patterns to eliminate this branch and use the next match arm instead
                        return Err(serenity::Error::Http(e)).annotate("failed to edit member");
                    },
                    Err(e) => { return Err(e).annotate("failed to edit member"); }
                }
            }
            _ => { return Err(Error::UnknownCommand(args)); }
        }
    }
    last_error
}

fn notify_ipc_crash(e: Error) {
    let mut child = Command::new("ssmtp")
        .arg("fenhl@fenhl.net")
        .stdin(Stdio::piped())
        .spawn()
        .expect("failed to spawn ssmtp");
    {
        let stdin = child.stdin.as_mut().expect("failed to open ssmtp stdin");
        write!(
            stdin,
            "To: fenhl@fenhl.net\nFrom: {}@{}\nSubject: Peter IPC thread crashed\n\nPeter IPC thread crashed with the following error:\n{}\n",
            whoami::username(),
            whoami::hostname(),
            e
        ).expect("failed to write to ssmtp stdin");
    }
    child.wait().expect("failed to wait for ssmtp subprocess"); //TODO check exit status
}

fn main() -> Result<()> {
    let mut args = env::args().peekable();
    let _ = args.next(); // ignore executable name
    if args.peek().is_some() {
        println!("{}", peter::send_ipc_command(args)?);
    } else {
        // read config
        let config = serde_json::from_reader::<_, Config>(File::open("/usr/local/share/fidera/config.json")?)?;
        let handler = Handler::default();
        let ctx_arc = handler.0.clone();
        let mut client = Client::new(&config.peter.bot_token, handler)?;
        let owners = iter::once(client.cache_and_http.http.get_current_application_info()?.owner.id).collect();
        {
            let mut data = client.data.write();
            data.insert::<ShardManagerContainer>(Arc::clone(&client.shard_manager));
            data.insert::<ConfigChannels>(config.channels);
            data.insert::<VoiceStates>(BTreeMap::default());
            data.insert::<werewolf::GameState>(werewolf::GameState::default());
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
                if let Err(e) = listen_ipc(ctx_arc) { //TODO remove `if` after changing from `()` to `!`
                    eprintln!("{}", e);
                    notify_ipc_crash(e);
                }
            })?;
        }
        // connect to Discord
        client.start_autosharded()?;
        thread::sleep(Duration::from_secs(1)); // wait to make sure websockets can be closed cleanly
    }
    Ok(())
}
