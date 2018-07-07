//! German language utilities.

#![allow(missing_docs)] //TODO remove

use std::fmt;

use num_traits::One;

use quantum_werewolf::game::{
    Faction,
    Role
};

pub fn join<D: fmt::Display, I: IntoIterator<Item=D>>(empty: Option<D>, words: I) -> String {
    let mut words = words.into_iter().map(|word| word.to_string()).collect::<Vec<_>>();
    match words.len() {
        0 => empty.expect("tried to join an empty list with no fallback").to_string(),
        1 => words.swap_remove(0),
        2 => format!("{} und {}", words.swap_remove(0), words.swap_remove(0)),
        _ => {
            let last = words.pop().unwrap();
            let first = words.remove(0);
            let builder = words.into_iter()
                .fold(first, |builder, word| format!("{}, {}", builder, word));
            format!("{} und {}", builder, last)
        }
    }
}

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
