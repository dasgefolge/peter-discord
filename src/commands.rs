//! Implements of all of the bot's commands.

#![allow(missing_docs)]

use std::{
    collections::HashSet,
    sync::Arc
};
use rand::{
    Rng,
    thread_rng
};
use serenity::{
    framework::standard::{
        Args,
        Command,
        CommandOptions,
        CommandError
    },
    model::prelude::*,
    prelude::*
};
use crate::{
    GEFOLGE,
    emoji,
    model::{
        Event,
        Transaction,
        UserData
    },
    shut_down
};

const MENSCH: RoleId = RoleId(386753710434287626);

pub struct MigrateGuest;

impl Command for MigrateGuest {
    fn execute(&self, _: &mut Context, msg: &Message, mut args: Args) -> Result<(), CommandError> {
        let user = args.single::<UserId>()?;
        let mut events = Vec::default();
        loop {
            match args.remaining() {
                0 => { break; }
                1 => { return Err("expected groups of two arguments (event and guest ID), found only 1".into()); }
                _ => {
                    events.push((args.single::<Event>()?, args.single::<u8>()?));
                }
            }
        }
        let mut roles = GEFOLGE.member(user)?.roles.into_iter().collect::<HashSet<_>>();
        roles.insert(MENSCH);
        for (event, guest_id) in events {
            let mut event_data = event.data()?;
            roles.insert(event_data.role);
            let via = if let Some(person_data) = event_data.person_data_mut(Err(guest_id)) {
                person_data.id = user.0;
                person_data.name = None;
                person_data.via.take().ok_or(format!("im Gast-Eintrag für {} fehlt das Feld \"via\"", event))?
            } else {
                return Err(format!("das event {} hat keinen Gast mit ID {}", event, guest_id).into());
            };
            for programmpunkt in event_data.programm.values_mut() {
                for signup in &mut programmpunkt.signups {
                    if *signup == guest_id as u64 {
                        *signup = user.0;
                    }
                }
                if let Some(ref mut targets) = programmpunkt.targets {
                    if let Some(target) = targets.remove(&(guest_id as u64)) {
                        targets.insert(user.0, if target == guest_id as u64 { user.0 } else { target });
                    }
                    for target in targets.values_mut() {
                        if *target == guest_id as u64 {
                            *target = user.0;
                        }
                    }
                }
            }
            event.save_data(&event_data)?;
            let mut guest_data = UserData::load(user)?;
            let mut via_data = UserData::load(via)?;
            for transaction in &mut via_data.transactions {
                match transaction {
                    Transaction::EventAnzahlung { event: e, guest: Some(gid), amount, default, time } if *e == event && *gid == guest_id => {
                        let comment = Some(format!("Anzahlung {} (ursprünglich als Gast angemeldet)", event_data.name));
                        guest_data.transactions.push(Transaction::Transfer {
                            amount: -amount.clone(),
                            comment: comment.clone(),
                            mensch: via,
                            time: *time
                        });
                        guest_data.transactions.push(Transaction::EventAnzahlung {
                            amount: amount.clone(),
                            default: default.clone(),
                            event: event.clone(),
                            guest: None,
                            time: *time,
                        });
                        *transaction = Transaction::Transfer {
                            comment,
                            amount: amount.clone(),
                            mensch: user,
                            time: *time
                        };
                    }
                    Transaction::EventAnzahlungReturn { .. } => unimplemented!("migrate-guest for EventAnzahlungReturn"),
                    Transaction::EventAbrechnung { event: e, guest: Some(gid), amount, details, time } if *e == event && *gid == guest_id => {
                        let comment = Some(format!("Abrechnung {} (ursprünglich als Gast angemeldet)", event_data.name));
                        guest_data.transactions.push(Transaction::Transfer {
                            amount: -amount.clone(),
                            comment: comment.clone(),
                            mensch: via,
                            time: *time
                        });
                        guest_data.transactions.push(Transaction::EventAbrechnung {
                            amount: amount.clone(),
                            details: details.clone(),
                            event: event.clone(),
                            guest: None,
                            time: *time,
                        });
                        *transaction = Transaction::Transfer {
                            comment,
                            amount: amount.clone(),
                            mensch: user,
                            time: *time
                        };
                    }
                    _ => {}
                }
            }
            guest_data.transactions.sort_by_key(|transaction| transaction.time());
            via_data.transactions.sort_by_key(|transaction| transaction.time());
            guest_data.save(user)?;
            via_data.save(via)?;
        }
        GEFOLGE.edit_member(user, |m| m.roles(roles))?;
        msg.react("✅")?;
        Ok(())
    }

    fn options(&self) -> Arc<CommandOptions> {
        Arc::new(CommandOptions {
            owners_only: true,
            ..CommandOptions::default()
        })
    }
}

pub fn ping(_: &mut Context, msg: &Message, _: Args) -> Result<(), CommandError> {
    let mut rng = thread_rng();
    let pingception = format!("BWO{}{}G", "R".repeat(rng.gen_range(3, 20)), "N".repeat(rng.gen_range(1, 5)));
    msg.reply(if rng.gen_bool(0.001) { &pingception } else { "pong" })?;
    Ok(())
}

pub fn poll(_: &mut Context, msg: &Message, mut args: Args) -> Result<(), CommandError> {
    let mut emoji_iter = emoji::Iter::new(msg.content.to_owned())?.peekable();
    if emoji_iter.peek().is_some() {
        for emoji in emoji_iter {
            msg.react(emoji)?;
        }
    } else if let Ok(num_reactions) = args.single::<u8>() {
        for i in 0..num_reactions.min(26) {
            msg.react(emoji::nth_letter(i))?;
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

pub fn roll(_: &mut Context, _: &Message, _: Args) -> Result<(), CommandError> {
    unimplemented!(); //TODO
}

pub fn shuffle(_: &mut Context, _: &Message, _: Args) -> Result<(), CommandError> {
    unimplemented!(); //TODO
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
