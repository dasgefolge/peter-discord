#![deny(rust_2018_idioms, unused, unused_import_braces, unused_lifetimes, unused_qualifications, warnings)]

use {
    std::{
        collections::{
            BTreeSet,
            HashMap,
        },
        fmt,
        future::Future,
        pin::Pin,
        time::{
            Duration,
            Instant,
        },
    },
    itertools::Itertools as _,
    serde_json::json,
    serenity::{
        futures::TryFutureExt as _,
        model::prelude::*,
        prelude::*,
        utils::MessageBuilder,
    },
    serenity_utils::{
        builder::ErrorNotifier,
        handler::{
            HandlerMethods as _,
            voice_state::VoiceStates,
        },
        slash::*,
    },
    tokio::{
        fs,
        time::sleep,
    },
    peter::{
        Error,
        FENHL,
        GEFOLGE,
        GUEST,
        MENSCH,
        commands,
        config::Config,
        twitch,
        werewolf,
    },
};

enum VoiceStateExporter {}

impl serenity_utils::handler::voice_state::ExporterMethods for VoiceStateExporter {
    fn dump_info<'a>(_: &'a Context, guild_id: GuildId, VoiceStates(voice_states): &'a VoiceStates) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'a>> {
        Box::pin(async move {
            if guild_id != GEFOLGE { return Ok(()) }
            let buf = serde_json::to_vec_pretty(&json!({
                "channels": voice_states.into_iter()
                    .map(|(channel_id, (channel_name, members))| json!({
                        "members": members.into_iter()
                            .map(|user| json!({
                                "discriminator": user.discriminator,
                                "snowflake": user.id,
                                "username": user.name,
                            }))
                            .collect_vec(),
                        "name": channel_name,
                        "snowflake": channel_id,
                    }))
                    .collect_vec()
            }))?;
            fs::write("/usr/local/share/fidera/discord/voice-state.json", buf).await?;
            Ok(())
        })
    }

    fn ignored_channels<'a>(ctx: &'a Context) -> Pin<Box<dyn Future<Output = Result<BTreeSet<ChannelId>, Box<dyn std::error::Error + Send + Sync>>> + Send + 'a>> {
        Box::pin(async move {
            let data = ctx.data.read().await;
            Ok(data.get::<Config>().expect("missing config").channels.ignored.clone())
        })
    }

    fn notify_start<'a>(ctx: &'a Context, user_id: UserId, guild_id: GuildId, channel_id: ChannelId) -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send + 'a>> {
        Box::pin(async move {
            if guild_id != GEFOLGE { return Ok(()) }
            let data = ctx.data.read().await;
            let config = data.get::<Config>().expect("missing config");
            let mut msg_builder = MessageBuilder::default();
            msg_builder.push("Discord Party? ");
            msg_builder.mention(&user_id);
            msg_builder.push(" ist jetzt im voice channel ");
            msg_builder.mention(&channel_id);
            config.channels.voice.say(&ctx, msg_builder).await?;
            Ok(())
        })
    }
}

#[serenity_utils::main(ipc = "peter::ipc")]
async fn main() -> Result<serenity_utils::Builder, Error> {
    let config = Config::new().await?;
    Ok(serenity_utils::builder(365936493539229699, config.peter.bot_token.clone()).await?
        .error_notifier(ErrorNotifier::User(FENHL))
        .event_handler(serenity_utils::handler::user_list_exporter::<peter::user_list::Exporter>())
        .event_handler(serenity_utils::handler::voice_state_exporter::<VoiceStateExporter>())
        .slash_command(GEFOLGE, "team", CommandPermissions::from(MENSCH) | GUEST,
            |cmd| cmd
                .description("In ein Team wechseln, z.B. für ein Quiz")
                .create_option(|opt| {
                    opt.name("team").description("Die Teamnummer").kind(ApplicationCommandOptionType::Integer).default_option(true).required(true);
                    for i in 1..=6 { opt.add_int_choice(i.to_string(), i); }
                    opt
                }),
            |ctx, interaction| Box::pin(async move {
                const TEAMS: [RoleId; 6] = [
                    RoleId(828431321586991104),
                    RoleId(828431500747735100),
                    RoleId(828431624759935016),
                    RoleId(828431736194072606),
                    RoleId(828431741332750407),
                    RoleId(828431913738960956),
                ];

                #[derive(Debug)]
                enum TeamCommandError {
                    ParseOptions,
                    TeamNumberRange,
                }

                impl fmt::Display for TeamCommandError {
                    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                        match self {
                            Self::ParseOptions => write!(f, "unexpected options"),
                            Self::TeamNumberRange => write!(f, "team number was not in 1..=6"),
                        }
                    }
                }

                impl std::error::Error for TeamCommandError {}

                if let Some(mut member) = interaction.member {
                    let team_idx = if let [ApplicationCommandInteractionDataOption { name, resolved: Some(ApplicationCommandInteractionDataOptionValue::Integer(team_number)), .. }] = &interaction.data.options[..] { //TODO add structopt-style combined slash command argument definition/parsing to serenity-utils to make this less verbose?
                        if name == "team" {
                            if (1..=6).contains(team_number) {
                                (team_number - 1) as usize
                            } else {
                                return Err(Box::new(TeamCommandError::TeamNumberRange) as Box<dyn std::error::Error + Send + Sync>)
                            }
                        } else {
                            return Err(Box::new(TeamCommandError::ParseOptions))
                        }
                    } else {
                        return Err(Box::new(TeamCommandError::ParseOptions))
                    };
                    member.remove_roles(&ctx, &TEAMS.iter().enumerate().filter_map(|(idx, &role_id)| (idx != team_idx).then(|| role_id)).collect_vec()).await?;
                    member.add_role(ctx, TEAMS[team_idx]).await?;
                } else {
                    interaction.create_interaction_response(ctx, |resp| resp
                        .interaction_response_data(|data| data
                            .content("Dieses command funktioniert nur im Gefolge.")
                            .flags(InteractionApplicationCommandCallbackDataFlags::EPHEMERAL)
                        )
                    ).await?;
                }
                Ok(())
            }),
        )
        .message_commands(Some("!"), &commands::GROUP)
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
