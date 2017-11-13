//! Implements auto-moderating games of Werewolf.

#![allow(missing_docs)]

use std::{iter, mem, str, thread};
use std::ascii::AsciiExt;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use quantum_werewolf::game::{NightAction, NightActionResult, Role};
use quantum_werewolf::game::state::*;

use rand::{Rng, thread_rng};

use serenity::prelude::*;
use serenity::framework::standard::{Args, CommandError};
use serenity::model::{ChannelId, Message, UserId};
use serenity::utils::MessageBuilder;

use typemap::Key;

use lang::{cardinal, join, role_gender, role_name};

pub const TEXT_CHANNEL: ChannelId = ChannelId(378848336255516673);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Vote {
    Player(UserId),
    NoLynch
}

#[derive(Debug)]
pub enum Action {
    Night(NightAction<UserId>),
    Vote(UserId, Vote),
    Unvote(UserId)
}

/// The global game state is tracked here. Also serves as `typemap` key for the global game.
#[derive(Debug, Default)]
pub struct GameState {
    state: State<UserId>,
    alive: Option<HashSet<UserId>>,
    night_actions: Vec<NightAction<UserId>>,
    timeouts: Vec<bool>,
    votes: HashMap<UserId, Vote>
}

impl GameState {
    fn announce_deaths(&mut self) -> ::Result<()> {
        self.alive = if let Some(new_alive) = self.state.alive() {
            let new_alive = new_alive.into_iter().cloned().collect();
            if let Some(ref old_alive) = self.alive {
                let mut died = (old_alive - &new_alive).into_iter().map(|user_id| user_id.get()).collect::<::serenity::Result<Vec<_>>>()?;
                if !died.is_empty() {
                    died.sort_by_key(|user| (user.name.clone(), user.discriminator));
                    let mut builder = MessageBuilder::default();
                    for (i, dead_player) in died.into_iter().enumerate() {
                        if i > 0 {
                            builder = builder.push(" ");
                        }
                        builder = builder
                            .mention(dead_player.id)
                            .push(" ist tot");
                        if let Some(role) = self.state.role(&dead_player.id) {
                            builder = builder
                                .push(" und war ")
                                .push_safe(role_name(role, ::lang::Nom, false));
                        }
                        builder = builder.push(".");
                    }
                    TEXT_CHANNEL.say(builder)?;
                }
            }
            Some(new_alive)
        } else {
            None
        };
        //TODO send new role DMs for remaining Quantum States
        Ok(())
    }

    fn cancel_all_timeouts(&mut self) {
        self.timeouts = vec![false; self.timeouts.len()];
    }

    fn cancel_timeout(&mut self, timeout_idx: usize) {
        self.timeouts[timeout_idx] = false;
    }

    fn resolve_day(&mut self, day: Day<UserId>) -> ::Result<State<UserId>> {
        self.cancel_all_timeouts();
        // determine the players and/or game actions with the most votes
        let (_, vote_result) = vote_leads(&self);
        // if the result is a single player, lynch that player
        let result = if vote_result.len() == 1 {
            match vote_result.into_iter().next().unwrap() {
                Vote::Player(user_id) => day.lynch(user_id),
                Vote::NoLynch => day.no_lynch()
            }
        } else {
            day.no_lynch()
        };
        self.votes = HashMap::default();
        self.announce_deaths()?;
        if let State::Night(_) = result {
            TEXT_CHANNEL.say("Es wird Nacht. Bitte schickt mir innerhalb der nächsten 3 Minuten eure Nachtaktionen.")?; //TODO adjust for night timeout changes
        }
        Ok(result)
    }

    fn resolve_night(&mut self, night: Night<UserId>) -> ::Result<State<UserId>> {
        self.cancel_all_timeouts();
        let result = night.resolve_nar(&self.night_actions);
        self.night_actions = Vec::default();
        if let State::Day(ref day) = result {
            // send night action results
            for (player, result) in day.night_action_results() {
                match result {
                    NightActionResult::Investigation(faction) => {
                        let dm = MessageBuilder::default()
                            .push("Ergebnis deiner Nachtaktion: ")
                            .push_safe(::lang::faction_name(faction, ::lang::Nom))
                            .build();
                        player.create_dm_channel()?.say(&dm)?;
                    }
                }
            }
            // announce probability table
            let mut builder = MessageBuilder::default()
                .push("Die aktuelle Wahrscheinlichkeitsverteilung:");
            for (player_idx, probabilities) in day.probability_table().into_iter().enumerate() {
                builder = builder.push_line("").push_safe(match probabilities {
                    Ok((village_ratio, werewolves_ratio, dead_ratio)) => {
                        format!("{}: {}% Dorf, {}% Werwolf, {}% tot", player_idx + 1, (village_ratio * 100.0).round() as u8, (werewolves_ratio * 100.0).round() as u8, (dead_ratio * 100.0).round() as u8)
                    }
                    Err(faction) => {
                        format!("{}: tot (war {})", player_idx + 1, ::lang::faction_name_sg(faction, ::lang::Nom))
                    }
                });
            }
            TEXT_CHANNEL.say(builder)?;
            // open discussion
            let lynch_votes = day.alive().len() / 2 + 1;
            let builder = MessageBuilder::default()
                .push("Es wird Tag. Die Diskussion ist eröffnet. Absolute Mehrheit besteht aus ")
                .push_safe(::lang::cardinal(lynch_votes, ::lang::Dat, ::lang::F))
                .push(if lynch_votes == 1 { " Stimme." } else { " Stimmen." });
            TEXT_CHANNEL.say(builder)?;
        }
        Ok(result)
    }

    fn start_timeout(&mut self) -> usize {
        let idx = self.timeouts.len();
        self.timeouts.push(true);
        idx
    }

    fn timeout_cancelled(&self, timeout_idx: usize) -> bool {
        !self.timeouts[timeout_idx]
    }

    fn timeouts_active(&self) -> bool {
        self.timeouts.iter().any(|&active| active)
    }
}

impl Key for GameState {
    type Value = GameState;
}

pub fn command_in(ctx: &mut Context, msg: &Message, _: Args) -> Result<(), CommandError> {
    {
        let mut data = ctx.data.lock();
        let state = data.get_mut::<GameState>().expect("missing Werewolf game state");
        if let State::Complete(_) = state.state {
            state.state = State::default();
        }
        if let State::Signups(ref mut signups) = state.state {
            if !signups.sign_up(msg.author.id) {
                msg.reply("du bist schon angemeldet")?;
                return Ok(());
            }
            msg.react("✅")?;
        } else {
            msg.reply("bitte warte, bis das aktuelle Spiel vorbei ist")?;
            return Ok(());
        }
    }
    continue_game(ctx)?;
    Ok(())
}

pub fn command_out(ctx: &mut Context, msg: &Message, _: Args) -> Result<(), CommandError> {
    {
        let mut data = ctx.data.lock();
        let state = data.get_mut::<GameState>().expect("missing Werewolf game state");
        if let State::Complete(_) = state.state {
            state.state = State::default();
        }
        if let State::Signups(ref mut signups) = state.state {
            if !signups.remove_player(&msg.author.id) {
                msg.reply("du warst nicht angemeldet")?;
                return Ok(());
            }
            msg.react("✅")?;
        } else {
            msg.reply("bitte warte, bis das aktuelle Spiel vorbei ist")?; //TODO implement forfeiting
            return Ok(());
        }
    }
    continue_game(ctx)?;
    Ok(())
}

fn continue_game(ctx: &mut Context) -> ::Result<()> {
    let (mut timeout_idx, mut sleep_duration) = {
        let mut data = ctx.data.lock();
        let state_ref = data.get_mut::<GameState>().expect("missing Werewolf game state");
        if let Some(duration) = handle_game_state(state_ref)? {
            if state_ref.timeouts_active() { return Ok(()); }
            (state_ref.start_timeout(), duration)
        } else {
            return Ok(());
        }
    };
    loop {
        thread::sleep(sleep_duration);
        let mut data = ctx.data.lock();
        let state_ref = data.get_mut::<GameState>().expect("missing Werewolf game state");
        if state_ref.timeout_cancelled(timeout_idx) { break; }
        state_ref.cancel_timeout(timeout_idx);
        if let Some(duration) = handle_timeout(state_ref)? {
            if state_ref.timeouts_active() { break; }
            timeout_idx = state_ref.start_timeout();
            sleep_duration = duration;
        } else {
            break;
        }
    }
    Ok(())
}

/// Return whether or not the action was recognized.
pub fn handle_action(ctx: &mut Context, action: Action) -> ::Result<bool> {
    {
        let mut data = ctx.data.lock();
        let state_ref = data.get_mut::<GameState>().expect("missing Werewolf game state");
        match state_ref.state {
            State::Night(ref night) => {
                if let Action::Night(night_action) = action {
                    if !night.secret_ids().contains(night_action.src()) { return Ok(false); }
                    state_ref.night_actions.push(night_action);
                } else {
                    return Ok(false);
                }
            }
            State::Day(ref day) => match action {
                Action::Vote(src_id, vote) => {
                    if !day.secret_ids().contains(&src_id) { return Ok(false); }
                    state_ref.votes.insert(src_id, vote);
                }
                Action::Unvote(src_id) => {
                    if !day.secret_ids().contains(&src_id) { return Ok(false); }
                    state_ref.votes.remove(&src_id);
                }
                Action::Night(..) => { return Ok(false); }
            }
            State::Signups(_) | State::Complete(_) => { return Ok(false); }
        }
    }
    continue_game(ctx)?;
    Ok(true)
}

fn handle_game_state(state_ref: &mut GameState) -> ::Result<Option<Duration>> {
    state_ref.announce_deaths()?;
    let state = mem::replace(&mut state_ref.state, State::default());
    Ok(match state {
        State::Signups(signups) => {
            if signups.num_players() < MIN_PLAYERS {
                state_ref.state = State::Signups(signups);
                None
            } else {
                if !state_ref.timeouts_active() {
                    TEXT_CHANNEL.say("das Spiel startet in einer Minute")?;
                }
                state_ref.state = State::Signups(signups);
                Some(Duration::from_secs(60)) // allow more players to sign up
            }
        }
        State::Night(night) => {
            if night.actions_complete(&state_ref.night_actions) {
                state_ref.state = state_ref.resolve_night(night)?;
                handle_game_state(state_ref)?
            } else {
                state_ref.state = State::Night(night);
                Some(Duration::from_secs(180)) // 3 minute night time limit works for XylBot, may need to be adjusted up or down. Collect stats?
            }
        }
        State::Day(day) => {
            let (max_votes, vote_result) = vote_leads(&state_ref);
            if max_votes > day.alive().len() / 2 && vote_result.len() == 1 {
                if let Some(Vote::Player(_)) = vote_result.into_iter().next() {
                    state_ref.state = state_ref.resolve_day(day)?;
                    handle_game_state(state_ref)?
                } else {
                    state_ref.state = State::Day(day);
                    Some(Duration::from_secs(1800)) // Not sure how long the day limit should be. Starting out with half an hour for now to be safe. Collect stats?
                }
            } else {
                state_ref.state = State::Day(day);
                Some(Duration::from_secs(1800)) // Not sure how long the day limit should be. Starting out with half an hour for now to be safe. Collect stats?
            }
        }
        State::Complete(Complete { winners }) => {
            let mut winners = winners.into_iter().map(|user_id| user_id.get()).collect::<::serenity::Result<Vec<_>>>()?;
            winners.sort_by_key(|user| (user.name.clone(), user.discriminator));
            let builder = MessageBuilder::default()
                .push("das Spiel ist vorbei: ");
            TEXT_CHANNEL.say(match winners.len() {
                0 => builder.push("niemand hat gewonnen"),
                1 => builder.mention(winners.swap_remove(0)).push(" hat gewonnen"),
                _ => {
                    let mut builder = builder.mention(winners.remove(0));
                    for winner in winners {
                        builder = builder.push(" ").mention(winner);
                    }
                    builder.push(" haben gewonnen")
                }
            })?;
            state_ref.state = State::default();
            None
        }
    })
}

fn handle_timeout(state_ref: &mut GameState) -> ::Result<Option<Duration>> {
    let state = mem::replace(&mut state_ref.state, State::default());
    state_ref.state = match state {
        State::Signups(signups) => {
            if signups.num_players() < MIN_PLAYERS {
                State::Signups(signups)
            } else {
                // create a random role distribution
                let num_ww = signups.num_players() * 2 / 5;
                let mut roles = (0..num_ww).map(|i| Role::Werewolf(i)).collect::<Vec<_>>();
                roles.push(Role::Detective);
                if signups.num_players() > 4 && thread_rng().gen() { roles.push(Role::Healer); }
                // start the game with that distribution
                let started = signups.start(roles.clone())?;
                for (secret_id, player) in started.secret_ids().expect("failed to get secred player IDs").into_iter().enumerate() {
                    let dm = quantum_role_dm(&roles, started.num_players(), secret_id);
                    player.create_dm_channel()?.say(&dm)?;
                }
                started
            }
        }
        State::Night(night) => state_ref.resolve_night(night)?,
        State::Day(day) => state_ref.resolve_day(day)?,
        State::Complete(_) => { unimplemented!(); } // there shouldn't be any timeouts after the game ends
    };
    handle_game_state(state_ref)
}

pub fn parse_action(ctx: &mut Context, src: UserId, mut msg: &str) -> Option<Action> {
    fn parse_player(ctx: &mut Context, subj: &mut &str) -> Option<UserId> {
        if let Some(user_id) = ::parse::user_mention(subj) {
            if player_in_game(ctx, user_id) { Some(user_id) } else { None }
        } else {
            //TODO parse `username` or `username#1234` syntax, restrict to players in the game
            //TODO parse `@user name#1234` syntax, restrict to players in the game
            None
        }
    }

    // A simple parser for game actions.
    if msg.starts_with('!') { msg = &msg[1..] } // remove leading `!`, if any
    let mut cmd_name = String::default();
    loop {
        let next_char = match msg.chars().next() {
            Some(' ') => {
                msg = &msg[1..];
                break;
            }
            None => { break; }
            Some(c) => {
                msg = &msg[c.len_utf8()..];
                c
            }
        };
        cmd_name.push(next_char);
    }
    match &cmd_name[..] {
        "heal" => {
            match parse_player(ctx, &mut msg) {
                Some(tgt) => Some(Action::Night(NightAction::Heal(src, tgt))),
                None => None
            }
        }
        "inspect" | "investigate" => {
            match parse_player(ctx, &mut msg) {
                Some(tgt) => Some(Action::Night(NightAction::Investigate(src, tgt))),
                None => None
            }
        }
        "kill" => {
            match parse_player(ctx, &mut msg) {
                Some(tgt) => Some(Action::Night(NightAction::Kill(src, tgt))),
                None => None
            }
        }
        "sleep" => unimplemented!(), //TODO if *this player's* mandatory night actions are complete, note that the player is done submitting night actions. otherwise, reply with an error
        "unvote" => Some(Action::Unvote(src)),
        "v" | "vote" => {
            if msg.is_empty() {
                Some(Action::Unvote(src))
            } else {
                if vec!["no lynch", "nolynch", "nl"].into_iter().any(|prefix| msg.to_ascii_lowercase().starts_with(prefix)) {
                    return Some(Action::Vote(src, Vote::NoLynch));
                }
                match parse_player(ctx, &mut msg) {
                    Some(tgt) => Some(Action::Vote(src, Vote::Player(tgt))),
                    None => None
                }
            }
        }
        _ => None
    }
}

pub fn player_in_game(ctx: &mut Context, user_id: UserId) -> bool {
    let mut data = ctx.data.lock();
    let state_ref = data.get_mut::<GameState>().expect("missing Werewolf game state");
    state_ref.state.secret_ids().map_or(false, |secret_ids| secret_ids.contains(&user_id))
}

pub fn quantum_role_dm(roles: &[Role], num_players: usize, secret_id: usize) -> String {
    // Willkommen
    let mut builder = MessageBuilder::default()
        .push_line("Willkommen bei Quantenwerwölfe!"); //TODO Spielname (flavor) oder Variantenname (für normales ww etc)
    // Rollenname
    let mut role_counts = HashMap::<_, usize>::default();
    let extra_villagers = num_players - roles.len();
    if extra_villagers > 0 {
        role_counts.insert(Role::Villager, extra_villagers);
    }
    for &role in roles {
        let normalized_role = if let Role::Werewolf(_) = role {
            Role::Werewolf(0)
        } else {
            role
        };
        *role_counts.entry(normalized_role).or_insert(0) += 1;
    }
    let mut role_count_list = role_counts.clone().into_iter().collect::<Vec<_>>();
    role_count_list.sort_by_key(|&(role, _)| role_name(role, ::lang::Nom, false));
    builder = builder
        .push("Du bist eine Quantenüberlagerung aus ");
    builder = join(builder, role_count_list.into_iter().map(|(role, count)| {
        let card = cardinal(count as u64, ::lang::Dat, role_gender(role));
        if let Role::Werewolf(_) = role {
            format!("{} {}", card, if count == 1 { "Werwolf" } else { "Werwölfen" })
        } else {
            format!("{} {}", card, role_name(role, ::lang::Dat, count != 1))
        }
    }), None);
    builder = builder
        .push(".")
    // Rollenrang
        .push(" Dein Rollenrang ist ")
        .push(secret_id + 1)
        .push(".")
    //TODO Partei (für qww erst relevant, wenn nur noch eine Rolle möglich ist)
    //TODO Dorfname (bei Variante „die Gemeinschaft der Dörfer“)
        .push_line("");
    //TODO Gruppenmitspieler (irrelevant für qww, zB Werwölfe, Freimaurer, Seherinnen/Kekse)
    // Aktionen (Parteiaktionen klar als solche kennzeichnen)
    if *role_counts.get(&Role::Healer).unwrap_or(&0) > 0 {
        builder = builder
            .push("Solange du noch lebst, kannst du jede Nacht einen lebenden Spieler deiner Wahl heilen (")
            .push_mono_safe("heal <player>")
            .push_line("). In allen Universen, in denen du lebst und Heiler bist, kann dieser Spieler in dieser Nacht nicht sterben. Du kannst keinen Spieler heilen, den du schon in der vorherigen Nacht geheilt hast.");
    }
    if *role_counts.get(&Role::Detective).unwrap_or(&0) > 0 {
        builder = builder
            .push("Solange du noch lebst, kannst du jede Nacht einen Spieler deiner Wahl untersuchen (")
            .push_mono_safe("investigate <player>")
            .push_line("). Falls es mindestens ein Universum gibt, in dem du Detektiv bist, erfährst du die Partei dieses Spielers in einem zufälligen solchen Universum. Alle Universen, in denen du Detektiv bist und der Spieler nicht diese Partei hat, werden eliminiert.");
    }
    builder
        .push("Solange du noch lebst, tötest du in jeder Nacht einen lebenden Spieler deiner Wahl (")
        .push_mono_safe("kill <player>")
        .push_line("). In allen Universen, in denen du der Werwolf mit der kleinsten Rangnummer unter den lebenden Werwölfen bist, stirbt dieser Spieler.")
    // sonstige Effekte (Parteieffekte klar als solche kennzeichnen)
        .push_line("Jeden Morgen wird öffentlich aber anonym dein Rollenrang sowie die relativen Häufigkeiten der Universen, in denen du zum Dorf gehörst, derer in denen du zu den Werwölfen gehörst, und derer in denen du tot bist angekündigt.")
        .push_line("Wenn du in allen Universen tot bist, stirbst du.")
        .push_line("Wenn du stirbst oder am Ende des Spiels wird aus den Universen, in denen du bis eben noch gelebt hast, ein zufälliges ausgewählt und du bekommst deine Identität aus diesem Universum. Alle anderen Quantenüberlagerungen verlieren diese Identität aus ihren Überlagerungen, und alle Universen, in denen du nicht diese Identität warst, werden eliminiert.")
    //TODO wincons (für qww erst relevant, wenn nur noch eine Rolle möglich ist)
    //TODO optional: Kurzzusammenfassung der Regeln bzw link zu den vollständigen Regeln
    // Unterschrift
        .push("Viel Spaß!")
        .build()
}

fn vote_leads(state_ref: &GameState) -> (usize, HashSet<Vote>) {
    let mut vote_count = HashMap::<Vote, usize>::default();
    for (_, &vote) in state_ref.votes.iter() {
        *vote_count.entry(vote).or_insert(0) += 1;
    }
    vote_count.into_iter()
        .fold((0, HashSet::default()), |(max_votes, mut voted), (vote, count)|
            match count.cmp(&max_votes) {
                Ordering::Less => (max_votes, voted),
                Ordering::Equal => {
                    voted.insert(vote);
                    (max_votes, voted)
                }
                Ordering::Greater => (count, iter::once(vote).collect())
            }
        )
}
