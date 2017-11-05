//! Implements of all of the bot's commands.

#![allow(missing_docs)]

use rand::{Rng, thread_rng};

use serenity::prelude::*;
use serenity::framework::standard::{Args, CommandError};
use serenity::model::Message;

pub fn ping(_: &mut Context, msg: &Message, _: Args) -> Result<(), CommandError> {
    let mut rng = thread_rng();
    let pingception = format!("BWO{}{}G", "R".repeat(rng.gen_range(3, 20)), "N".repeat(rng.gen_range(1, 5)));
    msg.reply(if rng.gen_weighted_bool(1024) { &pingception } else { "pong" })?;
    Ok(())
}

//pub fn poll(_: &mut Context, msg: &Message, _: Args) -> Result<(), CommandError> {
//    unimplemented!(); //TODO
//    Ok(())
//}

pub fn quit(ctx: &mut Context, _: &Message, _: Args) -> Result<(), CommandError> {
    ctx.quit()?;
    Ok(())
}

pub fn test(&mut _: &mut Context, msg: &Message, args: Args) -> Result<(), CommandError> {
    println!("[ ** ] test(&mut _, &{:?}, {:?})", *msg, args);
    Ok(())
}
