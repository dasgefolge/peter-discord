//! Implements of all of the bot's commands.

#![allow(missing_docs)]

use {
    std::iter,
    futures::{
        pin_mut,
        stream::TryStreamExt as _,
    },
    itertools::Itertools as _,
    rand::{
        Rng as _,
        thread_rng,
    },
    serenity::{
        framework::standard::{
            Args,
            CommandResult,
            macros::{
                command,
                group,
            },
        },
        model::prelude::*,
        prelude::*,
    },
    serenity_utils::{
        shut_down,
        slash::*,
    },
    crate::{
        ADMIN,
        GEFOLGE,
        GUEST,
        MENSCH,
        QUIZMASTER,
        config::Config,
        emoji,
        werewolf::{
            COMMAND_DAY_COMMAND,
            COMMAND_IN_COMMAND,
            COMMAND_NIGHT_COMMAND,
            COMMAND_OUT_COMMAND,
        },
    },
};
pub use self::MAIN_GROUP as GROUP;

const TEAMS: [RoleId; 6] = [
    RoleId(828431321586991104),
    RoleId(828431500747735100),
    RoleId(828431624759935016),
    RoleId(828431736194072606),
    RoleId(828431741332750407),
    RoleId(828431913738960956),
];

#[serenity_utils::slash_command(GEFOLGE, allow(MENSCH, GUEST))]
/// Dir eine selbstzuweisbare Rolle zuweisen
pub async fn iam(ctx: &Context, member: &mut Member, #[serenity_utils(description = "die Rolle, die du haben möchtest")] role: Role) -> serenity::Result<&'static str> {
    if !ctx.data.read().await.get::<Config>().expect("missing self-assignable roles list").peter.self_assignable_roles.contains(&role.id) {
        return Ok("diese Rolle ist nicht selbstzuweisbar") //TODO submit role list on command creation
    }
    if member.roles.contains(&role.id) {
        return Ok("du hast diese Rolle schon")
    }
    member.add_role(&ctx, role).await?;
    Ok("✅")
}

#[serenity_utils::slash_command(GEFOLGE, allow(MENSCH, GUEST))]
/// Eine selbstzuweisbare Rolle von dir entfernen
pub async fn iamn(ctx: &Context, member: &mut Member, #[serenity_utils(description = "die Rolle, die du loswerden möchtest")] role: Role) -> serenity::Result<&'static str> {
    if !ctx.data.read().await.get::<Config>().expect("missing self-assignable roles list").peter.self_assignable_roles.contains(&role.id) {
        return Ok("diese Rolle ist nicht selbstzuweisbar") //TODO submit role list on command creation
    }
    if !member.roles.contains(&role.id) {
        return Ok("du hast diese Rolle sowieso nicht")
    }
    member.remove_role(&ctx, role).await?;
    Ok("✅")
}

#[serenity_utils::slash_command(GEFOLGE, allow_all)]
/// Testen, ob Peter online ist
pub fn ping() -> String {
    let mut rng = thread_rng();
    if rng.gen_bool(0.01) {
        format!("BWO{}{}G", "R".repeat(rng.gen_range(3..20)), "N".repeat(rng.gen_range(1..5)))
    } else {
        format!("pong")
    }
}

#[command] //TODO replace with message context menu command once supported on mobile
pub async fn poll(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let mut emoji_iter = emoji::Iter::new(msg).peekable();
    if emoji_iter.peek().is_some() {
        for emoji in emoji_iter {
            msg.react(&ctx, emoji).await?;
        }
    } else if let Ok(num_reactions) = args.single::<u8>() {
        for i in 0..num_reactions.min(26) {
            msg.react(&ctx, emoji::nth_letter(i)).await?;
        }
    } else {
        msg.react(&ctx, '👍').await?;
        msg.react(&ctx, '👎').await?;
    }
    Ok(())
}

#[serenity_utils::slash_command(GEFOLGE, allow(ADMIN))]
/// Peter abschalten
pub async fn quit(ctx: &Context, interaction: &ApplicationCommandInteraction) -> serenity::Result<NoResponse> {
    interaction.create_interaction_response(ctx, |builder| builder.interaction_response_data(|data| data.content("shutting down…"))).await?;
    shut_down(&ctx).await;
    Ok(NoResponse)
}

#[serenity_utils::slash_command(GEFOLGE, allow(ADMIN))]
/// Die Rollen und Nicknames für Quizmaster und Teams aufräumen
pub async fn reset_quiz(ctx: &Context, guild_id: GuildId) -> serenity::Result<&'static str> {
    let members = guild_id.members_iter(ctx);
    pin_mut!(members);
    while let Some(mut member) = members.try_next().await? {
        member.remove_roles(&ctx, &iter::once(QUIZMASTER).chain(TEAMS).collect_vec()).await?;
        //TODO adjust nickname
    }
    Ok("Teams aufgeräumt")
}

pub async fn roll(_: &Context, _: &Message, _: Args) -> CommandResult {
    unimplemented!(); //TODO
}

pub async fn shuffle(_: &Context, _: &Message, _: Args) -> CommandResult {
    unimplemented!(); //TODO
}

#[serenity_utils::slash_command(GEFOLGE, allow(MENSCH, GUEST))]
/// In ein Team wechseln, z.B. für ein Quiz
pub async fn team(ctx: &Context, member: &mut Member, #[serenity_utils(range = 1..=6, description = "Die Teamnummer")] team: i64) -> serenity::Result<String> {
    let team_idx = (team - 1) as usize;
    member.remove_roles(&ctx, &TEAMS.iter().enumerate().filter_map(|(idx, &role_id)| (idx != team_idx).then(|| role_id)).collect_vec()).await?;
    member.add_role(ctx, TEAMS[team_idx]).await?;
    //TODO adjust nickname
    Ok(format!("du bist jetzt in Team {}", team))
}

#[group]
#[commands(
    command_day,
    command_in,
    command_night,
    command_out,
    poll,
)]
struct Main;
