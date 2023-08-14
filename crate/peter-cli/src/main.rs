#![deny(rust_2018_idioms, unused, unused_import_braces, unused_lifetimes, unused_qualifications, warnings)]
#![forbid(unsafe_code)]

use {
    std::{
        collections::{
            BTreeSet,
            HashMap,
        },
        future::Future,
        iter,
        pin::{
            Pin,
            pin,
        },
        time::{
            Duration,
            Instant,
        },
    },
    futures::stream::TryStreamExt as _,
    itertools::Itertools as _,
    rand::prelude::*,
    serde_json::json,
    serenity::{
        all::{
            CreateCommand,
            CreateCommandPermission,
            CreateCommandOption,
            CreateInteractionResponse,
            CreateInteractionResponseMessage,
            EditCommandPermissions,
        },
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
    },
    sqlx::{
        PgPool,
        postgres::PgConnectOptions,
    },
    tokio::time::sleep,
    wheel::fs,
    peter::{
        ADMIN,
        Database,
        Error,
        FENHL,
        GEFOLGE,
        GUEST,
        MENSCH,
        QUIZMASTER,
        config::Config,
        twitch,
        werewolf,
    },
};

const TEAMS: [RoleId; 6] = [
    RoleId::new(828431321586991104),
    RoleId::new(828431500747735100),
    RoleId::new(828431624759935016),
    RoleId::new(828431736194072606),
    RoleId::new(828431741332750407),
    RoleId::new(828431913738960956),
];

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
            config.channels.voice.say(&ctx, msg_builder.build()).await?;
            Ok(())
        })
    }
}

#[derive(Clone, Copy)]
pub(crate) struct CommandIds {
    iam: Option<CommandId>,
    iamn: Option<CommandId>,
    ping: Option<CommandId>,
    reset_quiz: Option<CommandId>,
    team: Option<CommandId>,
}

impl TypeMapKey for CommandIds {
    type Value = HashMap<GuildId, CommandIds>;
}

#[serenity_utils::main(ipc = "peter::ipc")]
async fn main() -> Result<serenity_utils::Builder, Error> {
    let config = Config::new().await?;
    Ok(serenity_utils::builder(config.peter.bot_token.clone()).await?
        .error_notifier(ErrorNotifier::User(FENHL))
        .event_handler(serenity_utils::handler::user_list_exporter::<peter::user_list::Exporter>())
        .event_handler(serenity_utils::handler::voice_state_exporter::<VoiceStateExporter>())
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
        .on_guild_create(false, |ctx, guild, _| Box::pin(async move {
            let mut commands = Vec::default();
            let iam = (guild.id == GEFOLGE).then(|| {
                let idx = commands.len();
                commands.push(CreateCommand::new("iam")
                    .kind(CommandType::ChatInput)
                    .default_member_permissions(Permissions::ADMINISTRATOR)
                    .dm_permission(false)
                    .description("Dir eine selbstzuweisbare Rolle zuweisen")
                    .add_option(CreateCommandOption::new(
                        CommandOptionType::Role,
                        "role",
                        "die Rolle, die du haben möchtest",
                    ).required(true))
                );
                idx
            });
            let iamn = (guild.id == GEFOLGE).then(|| {
                let idx = commands.len();
                commands.push(CreateCommand::new("iamn")
                    .kind(CommandType::ChatInput)
                    .default_member_permissions(Permissions::ADMINISTRATOR)
                    .dm_permission(false)
                    .description("Eine selbstzuweisbare Rolle von dir entfernen")
                    .add_option(CreateCommandOption::new(
                        CommandOptionType::Role,
                        "role",
                        "die Rolle, die du loswerden möchtest",
                    ).required(true))
                );
                idx
            });
            let ping = (guild.id == GEFOLGE).then(|| {
                let idx = commands.len();
                commands.push(CreateCommand::new("ping")
                    .kind(CommandType::ChatInput)
                    .dm_permission(false)
                    .description("Testen, ob Peter online ist")
                );
                idx
            });
            let reset_quiz = (guild.id == GEFOLGE).then(|| {
                let idx = commands.len();
                commands.push(CreateCommand::new("reset-quiz")
                    .kind(CommandType::ChatInput)
                    .default_member_permissions(Permissions::ADMINISTRATOR)
                    .dm_permission(false)
                    .description("Die Rollen und Nicknames für Quizmaster und Teams aufräumen")
                );
                idx
            });
            let team = (guild.id == GEFOLGE).then(|| {
                let idx = commands.len();
                commands.push(CreateCommand::new("team")
                    .kind(CommandType::ChatInput)
                    .default_member_permissions(Permissions::ADMINISTRATOR)
                    .dm_permission(false)
                    .description("In ein Team wechseln, z.B. für ein Quiz")
                    .add_option(CreateCommandOption::new(
                        CommandOptionType::Integer,
                        "team",
                        "Die Teamnummer",
                    )
                        .required(true)
                        .min_int_value(1)
                        .max_int_value(6)
                    )
                );
                idx
            });
            let commands = guild.set_commands(ctx, commands).await?;
            if let Some(idx) = iam {
                guild.edit_command_permissions(ctx, commands[idx].id, EditCommandPermissions::new(vec![
                    CreateCommandPermission::role(MENSCH, true),
                    CreateCommandPermission::role(GUEST, true),
                ])).await?;
            }
            if let Some(idx) = iamn {
                guild.edit_command_permissions(ctx, commands[idx].id, EditCommandPermissions::new(vec![
                    CreateCommandPermission::role(MENSCH, true),
                    CreateCommandPermission::role(GUEST, true),
                ])).await?;
            }
            if let Some(idx) = reset_quiz {
                guild.edit_command_permissions(ctx, commands[idx].id, EditCommandPermissions::new(vec![
                    CreateCommandPermission::role(ADMIN, true),
                ])).await?;
            }
            if let Some(idx) = team {
                guild.edit_command_permissions(ctx, commands[idx].id, EditCommandPermissions::new(vec![
                    CreateCommandPermission::role(MENSCH, true),
                    CreateCommandPermission::role(GUEST, true),
                ])).await?;
            }
            ctx.data.write().await.entry::<CommandIds>().or_default().insert(guild.id, CommandIds {
                iam: iam.map(|idx| commands[idx].id),
                iamn: iamn.map(|idx| commands[idx].id),
                ping: ping.map(|idx| commands[idx].id),
                reset_quiz: reset_quiz.map(|idx| commands[idx].id),
                team: team.map(|idx| commands[idx].id),
            });
            Ok(())
        }))
        .on_interaction_create(|ctx, interaction| Box::pin(async move {
            match interaction {
                Interaction::Command(interaction) => {
                    let guild_id = interaction.guild_id.expect("Discord slash command called outside of a guild");
                    if let Some(&command_ids) = ctx.data.read().await.get::<CommandIds>().and_then(|command_ids| command_ids.get(&guild_id)) {
                        if Some(interaction.data.id) == command_ids.iam {
                            let mut member = interaction.member.clone().expect("/iam called outside of a guild");
                            let role_id = match interaction.data.options[0].value {
                                CommandDataOptionValue::Role(role) => role,
                                _ => panic!("unexpected slash command option type"),
                            };
                            let response = if !ctx.data.read().await.get::<Config>().expect("missing self-assignable roles list").peter.self_assignable_roles.contains(&role_id) {
                                "diese Rolle ist nicht selbstzuweisbar"
                            } else if member.roles.contains(&role_id) {
                                "du hast diese Rolle schon"
                            } else {
                                member.add_role(&ctx, role_id).await?;
                                "✅"
                            };
                            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new()
                                .ephemeral(true)
                                .content(response)
                            )).await?;
                        } else if Some(interaction.data.id) == command_ids.iamn {
                            let mut member = interaction.member.clone().expect("/iamn called outside of a guild");
                            let role_id = match interaction.data.options[0].value {
                                CommandDataOptionValue::Role(role) => role,
                                _ => panic!("unexpected slash command option type"),
                            };
                            let response = if !ctx.data.read().await.get::<Config>().expect("missing self-assignable roles list").peter.self_assignable_roles.contains(&role_id) {
                                "diese Rolle ist nicht selbstzuweisbar"
                            } else if !member.roles.contains(&role_id) {
                                "du hast diese Rolle sowieso nicht"
                            } else {
                                member.remove_role(&ctx, role_id).await?;
                                "✅"
                            };
                            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new()
                                .ephemeral(true)
                                .content(response)
                            )).await?;
                        } else if Some(interaction.data.id) == command_ids.ping {
                            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new()
                                .ephemeral(true)
                                .content({
                                    let mut rng = thread_rng();
                                    if rng.gen_bool(0.01) {
                                        format!("BWO{}{}G", "R".repeat(rng.gen_range(3..20)), "N".repeat(rng.gen_range(1..5)))
                                    } else {
                                        format!("pong")
                                    }
                                })
                            )).await?;
                        } else if Some(interaction.data.id) == command_ids.reset_quiz {
                            let mut members = pin!(guild_id.members_iter(ctx));
                            while let Some(mut member) = members.try_next().await? {
                                member.remove_roles(&ctx, &iter::once(QUIZMASTER).chain(TEAMS).collect_vec()).await?;
                                //TODO adjust nickname
                            }
                            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new()
                                .ephemeral(true)
                                .content("Teams aufgeräumt")
                            )).await?;
                        } else if Some(interaction.data.id) == command_ids.team {
                            let mut member = interaction.member.clone().expect("/team called outside of a guild");
                            let team = match interaction.data.options[0].value {
                                CommandDataOptionValue::Integer(team) => team,
                                _ => panic!("unexpected slash command option type"),
                            };
                            let team_idx = (team - 1) as usize;
                            member.remove_roles(&ctx, &TEAMS.iter().enumerate().filter_map(|(idx, &role_id)| (idx != team_idx).then(|| role_id)).collect_vec()).await?;
                            member.add_role(ctx, TEAMS[team_idx]).await?;
                            //TODO adjust nickname
                            interaction.create_response(ctx, CreateInteractionResponse::Message(CreateInteractionResponseMessage::new()
                                .ephemeral(true)
                                .content(format!("du bist jetzt in Team {team}"))
                            )).await?;
                        } else {
                            panic!("unexpected slash command")
                        }
                    }
                }
                Interaction::Component(_) => panic!("received message component interaction even though no message components are registered"),
                _ => {}
            }
            Ok(())
        }))
        .data::<Config>(config)
        .data::<Database>(PgPool::connect_with(PgConnectOptions::default().database("gefolge").application_name("peter")).await?)
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
