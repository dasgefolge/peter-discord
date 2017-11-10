//! Implements of all of the bot's commands.

#![allow(missing_docs)]

use rand::{Rng, thread_rng};

use serenity::prelude::*;
use serenity::framework::standard::{Args, CommandError};
use serenity::model::{Message, ReactionType};

pub fn ping(_: &mut Context, msg: &Message, _: Args) -> Result<(), CommandError> {
    let mut rng = thread_rng();
    let pingception = format!("BWO{}{}G", "R".repeat(rng.gen_range(3, 20)), "N".repeat(rng.gen_range(1, 5)));
    msg.reply(if rng.gen_weighted_bool(1024) { &pingception } else { "pong" })?;
    Ok(())
}

pub fn poll(_: &mut Context, msg: &Message, mut args: Args) -> Result<(), CommandError> {
    let mut emoji_iter = ::emoji::Iter::new(msg.content.to_owned())?.peekable();
    if emoji_iter.peek().is_some() {
        for emoji in emoji_iter {
            msg.react(emoji)?;
        }
    } else if let Ok(num_reactions) = args.single::<u8>() {
        for i in 0..num_reactions.min(26) {
            msg.react(::emoji::nth_letter(i))?;
        }
    } else {
        msg.react(ReactionType::Unicode("👍".to_owned()))?;
        msg.react(ReactionType::Unicode("👎".to_owned()))?;
    }
    Ok(())
}

pub fn quit(ctx: &mut Context, _: &Message, _: Args) -> Result<(), CommandError> {
    ctx.quit()?;
    Ok(())
}

pub fn test(&mut _: &mut Context, msg: &Message, args: Args) -> Result<(), CommandError> {
    println!("[ ** ] test(&mut _, &{:?}, {:?})", *msg, args);
    Ok(())
}
