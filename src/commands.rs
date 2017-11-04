//! Implements of all of the bot's commands.

#![allow(missing_docs)]

use rand::{Rng, thread_rng};

use serenity::prelude::*;
use serenity::framework::standard::{Args, CommandError};
use serenity::model::Message;

pub fn ping(_: &mut Context, msg: &Message, _: Args) -> Result<(), CommandError> {
    let mut rng = thread_rng();
    msg.channel_id.say(if rng.gen_weighted_bool(1024) {
        format!("BWO{}{}G", "R".repeat(rng.gen_range(3, 20)), "N".repeat(rng.gen_range(1, 5))) // PINGCEPTION
    } else {
        "pong".to_owned()
    })?;
    Ok(())
}

pub fn quit(ctx: &mut Context, _: &Message, _: Args) -> Result<(), CommandError> {
    ctx.quit()?;
    Ok(())
}
