//! Implements of all of the bot's commands.

#![allow(missing_docs)]

use {
    rand::{
        Rng,
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
        model::{
            ModelError,
            prelude::*,
        },
        prelude::*,
    },
    serenity_utils::shut_down,
    crate::{
        config::Config,
        emoji,
        parse,
        werewolf::{
            COMMAND_DAY_COMMAND,
            COMMAND_IN_COMMAND,
            COMMAND_NIGHT_COMMAND,
            COMMAND_OUT_COMMAND,
        },
    },
};

#[command]
pub async fn iam(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut sender = match msg.member(&ctx).await {
        Ok(sender) => sender,
        Err(serenity::Error::Model(ModelError::ItemMissing)) => {
            //TODO get from `GEFOLGE` guild instead of erroring
            msg.reply(ctx, "dieser Befehl funktioniert aus technischen Gründen aktuell nicht in Privatnachrichten").await?;
            return Ok(());
        }
        Err(e) => return Err(Box::new(e) as _),
    };
    let mut cmd = args.message();
    let role = if let Some(role) = parse::eat_role_full(&mut cmd, msg.guild(&ctx).await) {
        role
    } else {
        msg.reply(ctx, "diese Rolle existiert nicht").await?;
        return Ok(());
    };
    if !ctx.data.read().await.get::<Config>().expect("missing self-assignable roles list").peter.self_assignable_roles.contains(&role) {
        msg.reply(ctx, "diese Rolle ist nicht selbstzuweisbar").await?;
        return Ok(());
    }
    if sender.roles.contains(&role) {
        msg.reply(ctx, "du hast diese Rolle schon").await?;
        return Ok(());
    }
    sender.add_role(&ctx, role).await?;
    msg.react(&ctx, '✅').await?;
    Ok(())
}

#[command]
pub async fn iamn(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let mut sender = match msg.member(&ctx).await {
        Ok(sender) => sender,
        Err(serenity::Error::Model(ModelError::ItemMissing)) => {
            //TODO get from `GEFOLGE` guild instead of erroring
            msg.reply(ctx, "dieser Befehl funktioniert aus technischen Gründen aktuell nicht in Privatnachrichten").await?;
            return Ok(());
        }
        Err(e) => return Err(Box::new(e) as _),
    };
    let mut cmd = args.message();
    let role = if let Some(role) = parse::eat_role_full(&mut cmd, msg.guild(&ctx).await) {
        role
    } else {
        msg.reply(ctx, "diese Rolle existiert nicht").await?;
        return Ok(());
    };
    if !ctx.data.read().await.get::<Config>().expect("missing self-assignable roles list").peter.self_assignable_roles.contains(&role) {
        msg.reply(ctx, "diese Rolle ist nicht selbstzuweisbar").await?;
        return Ok(());
    }
    if !sender.roles.contains(&role) {
        msg.reply(ctx, "du hast diese Rolle sowieso nicht").await?;
        return Ok(());
    }
    sender.remove_role(&ctx, role).await?;
    msg.react(&ctx, '✅').await?;
    Ok(())
}

#[command]
pub async fn ping(ctx: &Context, msg: &Message, _: Args) -> CommandResult {
    let reply = {
        let mut rng = thread_rng();
        let pingception = format!("BWO{}{}G", "R".repeat(rng.gen_range(3, 20)), "N".repeat(rng.gen_range(1, 5)));
        if rng.gen_bool(0.01) { pingception } else { format!("pong") }
    };
    msg.reply(ctx, reply).await?;
    Ok(())
}

#[command]
pub async fn poll(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let mut emoji_iter = emoji::Iter::new(msg.content.to_owned())?.peekable();
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

#[command]
#[owners_only]
pub async fn quit(ctx: &Context, _: &Message, _: Args) -> CommandResult {
    shut_down(&ctx).await;
    Ok(())
}

pub async fn roll(_: &Context, _: &Message, _: Args) -> CommandResult {
    unimplemented!(); //TODO
}

pub async fn shuffle(_: &Context, _: &Message, _: Args) -> CommandResult {
    unimplemented!(); //TODO
}

#[command]
#[owners_only]
pub async fn test(_: &Context, msg: &Message, args: Args) -> CommandResult {
    println!("[ ** ] test(&mut _, &{:?}, {:?})", *msg, args);
    Ok(())
}

#[group]
#[commands(
    command_day,
    iam,
    iamn,
    command_in,
    command_night,
    command_out,
    ping,
    poll,
    quit,
    test,
)]
struct Main;

pub use self::MAIN_GROUP as GROUP;
