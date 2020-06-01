//! Implements of all of the bot's commands.

#![allow(missing_docs)]

use {
    rand::{
        Rng,
        thread_rng
    },
    serenity::{
        framework::standard::{
            Args,
            CommandResult,
            macros::{
                command,
                group
            }
        },
        model::prelude::*,
        prelude::*
    },
    crate::{
        Config,
        emoji,
        parse,
        shut_down,
        werewolf::{
            COMMAND_IN_COMMAND,
            COMMAND_OUT_COMMAND
        }
    }
};

#[command]
pub fn iam(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let mut sender = if let Some(sender) = msg.member(&ctx) {
        sender
    } else {
        //TODO get from `GEFOLGE` guild instead of erroring
        msg.reply(ctx, "dieser Befehl funktioniert aus technischen Gründen aktuell nicht in Privatnachrichten")?;
        return Ok(());
    };
    let mut cmd = args.message();
    let role = if let Some(role) = parse::eat_role_full(&mut cmd, msg.guild(&ctx)) {
        role
    } else {
        msg.reply(ctx, "diese Rolle existiert nicht")?;
        return Ok(());
    };
    if !ctx.data.read().get::<Config>().expect("missing self-assignable roles list").peter.self_assignable_roles.contains(&role) {
        msg.reply(ctx, "diese Rolle ist nicht selbstzuweisbar")?;
        return Ok(());
    }
    if sender.roles.contains(&role) {
        msg.reply(ctx, "du hast diese Rolle schon")?;
        return Ok(());
    }
    sender.add_role(&ctx, role)?;
    msg.reply(ctx, "Rolle zugewiesen")?;
    Ok(())
}

#[command]
pub fn iamn(ctx: &mut Context, msg: &Message, args: Args) -> CommandResult {
    let mut sender = if let Some(sender) = msg.member(&ctx) {
        sender
    } else {
        //TODO get from `GEFOLGE` guild instead of erroring
        msg.reply(ctx, "dieser Befehl funktioniert aus technischen Gründen aktuell nicht in Privatnachrichten")?;
        return Ok(());
    };
    let mut cmd = args.message();
    let role = if let Some(role) = parse::eat_role_full(&mut cmd, msg.guild(&ctx)) {
        role
    } else {
        msg.reply(ctx, "diese Rolle existiert nicht")?;
        return Ok(());
    };
    if !ctx.data.read().get::<Config>().expect("missing self-assignable roles list").peter.self_assignable_roles.contains(&role) {
        msg.reply(ctx, "diese Rolle ist nicht selbstzuweisbar")?;
        return Ok(());
    }
    if !sender.roles.contains(&role) {
        msg.reply(ctx, "du hast diese Rolle sowieso nicht")?;
        return Ok(());
    }
    sender.remove_role(&ctx, role)?;
    msg.reply(ctx, "Rollenzuweisung entfernt")?;
    Ok(())
}

#[command]
pub fn ping(ctx: &mut Context, msg: &Message, _: Args) -> CommandResult {
    let mut rng = thread_rng();
    let pingception = format!("BWO{}{}G", "R".repeat(rng.gen_range(3, 20)), "N".repeat(rng.gen_range(1, 5)));
    msg.reply(ctx, if rng.gen_bool(0.001) { &pingception } else { "pong" })?;
    Ok(())
}

#[command]
pub fn poll(ctx: &mut Context, msg: &Message, mut args: Args) -> CommandResult {
    let mut emoji_iter = emoji::Iter::new(msg.content.to_owned())?.peekable();
    if emoji_iter.peek().is_some() {
        for emoji in emoji_iter {
            msg.react(&ctx, emoji)?;
        }
    } else if let Ok(num_reactions) = args.single::<u8>() {
        for i in 0..num_reactions.min(26) {
            msg.react(&ctx, emoji::nth_letter(i))?;
        }
    } else {
        msg.react(&ctx, "👍")?;
        msg.react(&ctx, "👎")?;
    }
    Ok(())
}

#[command]
#[owners_only]
pub fn quit(ctx: &mut Context, _: &Message, _: Args) -> CommandResult {
    shut_down(&ctx);
    Ok(())
}

pub fn roll(_: &mut Context, _: &Message, _: Args) -> CommandResult {
    unimplemented!(); //TODO
}

pub fn shuffle(_: &mut Context, _: &Message, _: Args) -> CommandResult {
    unimplemented!(); //TODO
}

#[command]
#[owners_only]
pub fn test(_: &mut Context, msg: &Message, args: Args) -> CommandResult {
    println!("[ ** ] test(&mut _, &{:?}, {:?})", *msg, args);
    Ok(())
}

#[group]
#[commands(
    iam,
    iamn,
    command_in,
    command_out,
    ping,
    poll,
    quit,
    test
)]
struct Main;

pub use self::MAIN_GROUP as GROUP;
