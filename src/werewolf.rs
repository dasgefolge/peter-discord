//! Implements auto-moderating games of Werewolf.

#![allow(missing_docs)]

use std::{iter, mem, str, thread};
use std::ascii::AsciiExt;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use quantum_werewolf::game::{NightAction, Role};
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
    night_actions: Vec<NightAction<UserId>>,
    votes: HashMap<UserId, Vote>,
    timeouts: Vec<bool>
}

impl GameState {
    fn cancel_all_timeouts(&mut self) {
        self.timeouts = vec![false; self.timeouts.len()];
    }

    fn cancel_timeout(&mut self, timeout_idx: usize) {
        self.timeouts[timeout_idx] = false;
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
                state_ref.cancel_all_timeouts();
                state_ref.state = night.resolve_nar(&state_ref.night_actions);
                state_ref.night_actions = Vec::default();
                handle_game_state(state_ref)?
            } else {
                state_ref.state = State::Night(night);
                Some(Duration::from_secs(180)) // 3 minute night time limit works for XylBot, may need to be adjusted up or down. Collect stats?
            }
        }
        State::Day(day) => {
            let (max_votes, vote_result) = vote_leads(&state_ref);
            if max_votes > day.alive().len() / 2 && vote_result.len() == 1 {
                if let Vote::Player(user_id) = vote_result.into_iter().next().unwrap() {
                    state_ref.cancel_all_timeouts();
                    state_ref.state = day.lynch(user_id);
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
            let msg = MessageBuilder::default()
                .push("das Spiel ist vorbei: ");
            TEXT_CHANNEL.say(match winners.len() {
                0 => msg.push("niemand hat gewonnen"),
                1 => msg.mention(winners.swap_remove(0)).push(" hat gewonnen"),
                _ => {
                    let mut msg = msg.mention(winners.remove(0));
                    for winner in winners {
                        msg = msg.push(" ").mention(winner);
                    }
                    msg.push(" haben gewonnen")
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
        State::Night(night) => {
            let next_state = night.resolve_nar(&state_ref.night_actions);
            state_ref.night_actions = Vec::default();
            next_state
        }
        State::Day(day) => {
            // determine the players and/or game actions with the most votes
            let (_, vote_result) = vote_leads(&state_ref);
            // if the result is a single player, lynch that player
            if vote_result.len() == 1 {
                match vote_result.into_iter().next().unwrap() {
                    Vote::Player(user_id) => day.lynch(user_id),
                    Vote::NoLynch => day.no_lynch()
                }
            } else {
                day.no_lynch()
            }
        }
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

pub fn quantum_role_dm(roles: &[Role], num_players: usize, _ /*secret_id*/: usize) -> String {
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
    let mut role_counts = role_counts.into_iter().collect::<Vec<_>>();
    role_counts.sort_by_key(|&(role, _)| role_name(role, ::lang::Nom, false));
    let builder = MessageBuilder::default()
        .push_safe("Du bist eine Quantenüberlagerung aus ");
    let builder = join(builder, role_counts.into_iter().map(|(role, count)| {
        let card = cardinal(count as u64, ::lang::Dat, role_gender(role));
        if let Role::Werewolf(_) = role {
            format!("{} {}", card, if count == 1 { "Werwolf" } else { "Werwölfen" })
        } else {
            format!("{} {}", card, role_name(role, ::lang::Dat, count != 1))
        }
    }), None);
    builder.push(".").build() //TODO everything after the role name
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
