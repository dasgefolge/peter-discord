//! German language utilities.

#![allow(missing_docs)] //TODO remove

use std::fmt;

use num::One;

use quantum_werewolf::game::{Faction, Role};

use serenity::utils::MessageBuilder;

pub enum Gender { M, F, N }
pub enum Case { Nom, Gen, Acc, Dat }

pub use self::Gender::*;
pub use self::Case::*;

pub fn cardinal<N: Eq + One + ToString>(n: N, case: Case, gender: Gender) -> String {
    if n == N::one() {
        match (case, gender) {
            (Nom, M) | (Nom, N) | (Acc, N) => "ein",
            (Nom, F) | (Acc, F) => "eine",
            (Gen, M) | (Gen, N) => "eines",
            (Gen, F) | (Dat, F) => "einer",
            (Acc, M) => "einen",
            (Dat, M) | (Dat, N) => "einem"
        }.to_owned()
    } else {
        n.to_string()
    }
}

pub fn faction_name(faction: Faction, case: Case) -> String {
    match faction {
        Faction::Village => match case {
            Gen => "Dorfes",
            _ => "Dorf"
        }.to_owned(),
        Faction::Werewolves => match case {
            Dat => "Werwölfen",
            _ => "Werwölfe"
        }.to_owned()
    }
}

pub fn faction_name_sg(faction: Faction, case: Case) -> String {
    match faction {
        Faction::Village => match case {
            Gen => "Dorfes",
            _ => "Dorf"
        }.to_owned(),
        Faction::Werewolves => match case {
            Dat => "Werwolf",
            _ => "Werwolf"
        }.to_owned()
    }
}

pub fn join<D: fmt::Display, I: IntoIterator<Item=D>>(builder: MessageBuilder, words: I, empty: Option<D>) -> MessageBuilder {
    let mut words = words.into_iter().map(|word| word.to_string()).collect::<Vec<_>>();
    match words.len() {
        0 => builder.push_safe(empty.expect("tried to join an empty list with no fallback")),
        1 => builder.push_safe(words.swap_remove(0)),
        2 => builder.push_safe(words.swap_remove(0)).push(" und ").push_safe(words.swap_remove(0)),
        _ => {
            let last = words.pop().unwrap();
            let first = words.remove(0);
            words.into_iter()
                .fold(builder.push_safe(first), |builder, word| builder.push(", ").push_safe(word))
                .push(" und ")
                .push_safe(last)
        }
    }
}

pub fn role_gender(role: Role) -> Gender {
    match role {
        Role::Detective => M,
        Role::Healer => M,
        Role::Villager => M,
        Role::Werewolf(_) => M
    }
}

pub fn role_name(role: Role, case: Case, plural: bool) -> String {
    match role {
        Role::Detective => match (case, plural) {
            (Gen, false) => "Detektivs",
            (_, false) => "Detektiv",
            (Dat, true) => "Detektiven",
            (_, true) => "Detektive"
        }.to_owned(),
        Role::Healer => match (case, plural) {
            (Gen, false) => "Heilers",
            (Dat, true) => "Heilern",
            _ => "Heiler"
        }.to_owned(),
        Role::Villager => match (case, plural) {
            (Gen, false) => "Dorfbewohners",
            (Dat, true) => "Dorfbewohnern",
            _ => "Dorfbewohner"
        }.to_owned(),
        Role::Werewolf(rank) => format!("{} (Rollenrang {})", match (case, plural) {
            (Gen, false) => "Werwolfs",
            (_, false) => "Werwolf",
            (Dat, true) => "Werwölfen",
            (_, true) => "Werwölfe"
        }, rank + 1)
    }
}
