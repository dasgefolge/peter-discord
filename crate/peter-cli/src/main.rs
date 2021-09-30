#![deny(rust_2018_idioms, unused, unused_import_braces, unused_lifetimes, unused_qualifications, warnings)]

use {
    std::{
        collections::HashMap,
        sync::Arc,
        time::{
            Duration,
            Instant,
        },
    },
    async_trait::async_trait,
    serenity::{
        client::bridge::gateway::GatewayIntents,
        futures::TryFutureExt as _,
        model::prelude::*,
        prelude::*,
        utils::MessageBuilder,
    },
    serenity_utils::{
        RwFuture,
        builder::ErrorNotifier,
        shut_down,
    },
    tokio::time::sleep,
    peter::{
        Error,
        FENHL,
        GEFOLGE,
        commands,
        config::Config,
        twitch,
        user_list,
        voice::{
            self,
            VoiceStates,
        },
        werewolf,
    },
};

struct Handler(Arc<Mutex<Option<tokio::sync::oneshot::Sender<Context>>>>);

impl Handler {
    fn new() -> (Handler, RwFuture<Context>) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        (Handler(Arc::new(Mutex::new(Some(tx)))), RwFuture::new(async move { rx.await.expect("failed to store handler context") }))
    }
}

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("Ready");
        if let Some(tx) = self.0.lock().await.take() {
            if let Err(_) = tx.send(ctx.clone()) {
                panic!("failed to send context")
            }
        }
        let guilds = ready.user.guilds(&ctx).await.expect("failed to get guilds");
        if guilds.is_empty() {
            println!("[!!!!] No guilds found, use following URL to invite the bot:");
            println!("[ ** ] {}", ready.user.invite_url(&ctx, Permissions::all()).await.expect("failed to generate invite URL"));
            shut_down(&ctx).await;
        }
    }

    async fn guild_ban_addition(&self, _: Context, guild_id: GuildId, user: User) {
        println!("User {} was banned from {}", user.name, guild_id);
        if guild_id != GEFOLGE { return; }
        user_list::remove(user).await.expect("failed to remove banned user from user list");
    }

    async fn guild_ban_removal(&self, ctx: Context, guild_id: GuildId, user: User) {
        println!("User {} was unbanned from {}", user.name, guild_id);
        if guild_id != GEFOLGE { return; }
        user_list::add(guild_id.member(ctx, user).await.expect("failed to get unbanned guild member"), None).await.expect("failed to add unbanned user to user list");
    }

    async fn guild_create(&self, ctx: Context, guild: Guild, _: bool) {
        println!("Connected to {}", guild.name);
        if guild.id != GEFOLGE { return; }
        user_list::set(guild.members.values().cloned()).await.expect("failed to initialize user list");
        let VoiceStates(mut chan_map) = VoiceStates::default();
        for (user_id, voice_state) in guild.voice_states {
            if let Some(channel_id) = voice_state.channel_id {
                let user = user_id.to_user(&ctx).await.expect("failed to get user info");
                if chan_map.get(&channel_id).is_none() {
                    chan_map.insert(channel_id, (channel_id.name(&ctx).await.expect("failed to get channel name"), Vec::default()));
                }
                let (_, ref mut users) = chan_map.get_mut(&channel_id).expect("just inserted");
                match users.binary_search_by_key(&(user.name.clone(), user.discriminator), |user| (user.name.clone(), user.discriminator)) {
                    Ok(idx) => { users[idx] = user; }
                    Err(idx) => { users.insert(idx, user); }
                }
            }
        }
        let mut data = ctx.data.write().await;
        data.insert::<VoiceStates>(VoiceStates(chan_map));
        let chan_map = data.get::<VoiceStates>().expect("missing voice states map");
        voice::dump_info(chan_map).await.expect("failed to update BitBar plugin");
    }

    async fn guild_member_addition(&self, _: Context, guild_id: GuildId, member: Member) {
        println!("User {} joined {}", member.user.name, guild_id);
        if guild_id != GEFOLGE { return; }
        user_list::add(member, None).await.expect("failed to add new guild member to user list");
    }

    async fn guild_member_removal(&self, _: Context, guild_id: GuildId, user: User, _: Option<Member>) {
        println!("User {} left {}", user.name, guild_id);
        if guild_id != GEFOLGE { return; }
        user_list::remove(user).await.expect("failed to remove removed guild member from user list");
    }

    async fn guild_member_update(&self, _: Context, _: Option<Member>, member: Member) {
        println!("Member data for {} updated", member.user.name);
        if member.guild_id != GEFOLGE { return; }
        user_list::update(member).await.expect("failed to update guild member info in user list");
    }

    async fn guild_members_chunk(&self, _: Context, chunk: GuildMembersChunkEvent) {
        println!("Received chunk of members for guild {}", chunk.guild_id);
        if chunk.guild_id != GEFOLGE { return; }
        for member in chunk.members.values() {
            user_list::add(member.clone(), None).await.expect("failed to add chunk of guild members to user list");
        }
    }

    async fn voice_state_update(&self, ctx: Context, guild_id: Option<GuildId>, _old: Option<VoiceState>, new: VoiceState) {
        println!("Voice states in guild {:?} updated", guild_id);
        if guild_id.map_or(true, |gid| gid != GEFOLGE) { return; } //TODO make sure this works, i.e. serenity never passes None for GEFOLGE
        let user = new.user_id.to_user(&ctx).await.expect("failed to get user info");
        let mut data = ctx.data.write().await;
        let ignored_channels = data.get::<Config>().expect("missing config").channels.ignored.clone();
        let voice_states = data.get_mut::<VoiceStates>().expect("missing voice states map");
        let VoiceStates(ref mut chan_map) = voice_states;
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
            if chan_map.get(&channel_id).is_none() {
                chan_map.insert(channel_id, (channel_id.name(&ctx).await.expect("failed to get channel name"), Vec::default()));
            }
            let (_, ref mut users) = chan_map.get_mut(&channel_id).expect("just inserted");
            match users.binary_search_by_key(&(user.name.clone(), user.discriminator), |user| (user.name.clone(), user.discriminator)) {
                Ok(idx) => { users[idx] = user.clone(); }
                Err(idx) => { users.insert(idx, user.clone()); }
            }
        }
        let is_empty = chan_map.iter().all(|(channel_id, (_, members))| members.is_empty() || ignored_channels.contains(channel_id));
        voice::dump_info(voice_states).await.expect("failed to update voice state dump");
        if was_empty && !is_empty {
            let config = data.get::<Config>().expect("missing config");
            let mut msg_builder = MessageBuilder::default();
            msg_builder.push("Discord Party? ");
            MessageBuilder::mention(&mut msg_builder, &user);
            msg_builder.push(" ist jetzt im voice channel ");
            msg_builder.mention(&chan_id.unwrap());
            config.channels.voice.say(&ctx, msg_builder).await.expect("failed to send channel message"); //TODO don't prefix channel name with `#`
        }
    }
}

#[serenity_utils::main(ipc = "peter::ipc")]
async fn main() -> Result<serenity_utils::Builder, Error> {
    let config = Config::new().await?;
    Ok(serenity_utils::builder(config.peter.bot_token.clone()).await?
        .error_notifier(ErrorNotifier::User(FENHL))
        .raw_event_handler_with_ctx(
            Handler::new,
            GatewayIntents::GUILD_BANS
            | GatewayIntents::GUILDS
            | GatewayIntents::GUILD_PRESENCES // required for guild member data in guild_create
            | GatewayIntents::GUILD_MEMBERS
            | GatewayIntents::GUILD_VOICE_STATES,
        )
        .commands(Some("!"), &commands::GROUP)
        .plain_message(|ctx, msg| Box::pin(async move {
            (msg.is_private() || ctx.data.read().await.get::<Config>().expect("missing config").werewolf.iter().any(|(_, conf)| conf.text_channel == msg.channel_id)) && {
                if let Some(action) = werewolf::parse_action(ctx, msg.author.id, &msg.content).await {
                    match async move { action }.and_then(|action| werewolf::handle_action(ctx, msg, action)).await {
                        Ok(()) => {} // reaction is posted in handle_action
                        Err(Error::GameAction(err_msg)) => { msg.reply(ctx, &err_msg).await.expect("failed to reply to game action"); }
                        Err(e) => { panic!("failed to handle game action: {}", e); }
                    }
                    true
                } else {
                    false
                }
    }
        }))
        .unrecognized_message("ich habe diese Nachricht nicht verstanden")
        .data::<Config>(config)
        .data::<VoiceStates>(VoiceStates::default())
        .data::<werewolf::GameState>(HashMap::default())
        .task(|ctx_fut, notify_thread_crash| async move {
            // check Twitch stream status
            let mut last_crash = Instant::now();
            let mut wait_time = Duration::from_secs(1);
            loop {
                let e = match twitch::alerts(ctx_fut.clone()).await {
                    Ok(never) => match never {},
                    Err(e) => e,
                };
                if last_crash.elapsed() >= Duration::from_secs(60 * 60 * 24) {
                    wait_time = Duration::from_secs(1); // reset wait time after no crash for a day
                } else {
                    wait_time *= 2; // exponential backoff
                }
                eprintln!("{}", e);
                if wait_time >= Duration::from_secs(2) { // only notify on multiple consecutive errors
                    notify_thread_crash(format!("Twitch"), Box::new(e), Some(wait_time)).await;
                }
                sleep(wait_time).await; // wait before attempting to reconnect
                last_crash = Instant::now();
            }
        })
    )
}
