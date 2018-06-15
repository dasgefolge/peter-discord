//! Implements of all of the bot's commands.

#![allow(missing_docs)]

use std::sync::Arc;

use rand::{
    Rng,
    thread_rng
};

use serenity::{
    prelude::*,
    framework::standard::{
        Args,
        Command,
        CommandOptions,
        CommandError
    },
    model::channel::Message
};

use shut_down;

pub fn ping(_: &mut Context, msg: &Message, _: Args) -> Result<(), CommandError> {
    let mut rng = thread_rng();
    let pingception = format!("BWO{}{}G", "R".repeat(rng.gen_range(3, 20)), "N".repeat(rng.gen_range(1, 5)));
    msg.reply(if rng.gen_bool(0.001) { &pingception } else { "pong" })?;
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
        msg.react("👍")?;
        msg.react("👎")?;
    }
    Ok(())
}

pub struct Quit;

impl Command for Quit {
    fn execute(&self, ctx: &mut Context, _: &Message, _: Args) -> Result<(), CommandError> {
        shut_down(&ctx);
        Ok(())
    }

    fn options(&self) -> Arc<CommandOptions> {
        Arc::new(CommandOptions {
            owners_only: true,
            ..CommandOptions::default()
        })
    }
}

pub struct Test;

impl Command for Test {
    fn execute(&self, _: &mut Context, msg: &Message, args: Args) -> Result<(), CommandError> {
        println!("[ ** ] test(&mut _, &{:?}, {:?})", *msg, args);
        Ok(())
    }

    fn options(&self) -> Arc<CommandOptions> {
        Arc::new(CommandOptions {
            owners_only: true,
            ..CommandOptions::default()
        })
    }
}
