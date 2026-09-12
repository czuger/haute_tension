//! The hero: how Prêtre Jean is rolled up, and what the sheet shows.
//!
//! The rules of the series, quoted from `regles-du-jeu-spj1`:
//!
//! - Vie — "lancez deux dés. Ajoutez 18 au chiffre que vous avez obtenu."
//! - Force — "lancez deux dés. Ajoutez 6 au chiffre que vous aurez obtenu."
//!
//! So a hero rolled by the book has a Force of 8–18 and a Vie of 20–30. That is
//! the [`NORMAL`] mode below.
//!
//! [`EASY`] is a house rule and not the book's: Force 12 + 2D4 and Vie 26 + 3D4,
//! which is 14–20 and 29–38. It cannot roll a weakling — the floor of each is
//! above the book's average — and it is there for reading the story rather than
//! surviving it. The mode is chosen once, when the game is created, and never
//! changes.
//!
//! Both are rolled **once**, when the game is created. Nothing here re-rolls:
//! `core::db::start_game` writes the result and every later read loads it back.
//!
//! Vie is kept twice over. `vie_max` is what was rolled and never moves again;
//! `vie_actuelle` is what is left, and is the only thing combat writes.

use crate::core::dice::{roll_dice, Rng, Roll};

/// A characteristic: a fixed base, and the dice added to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Throw {
    pub base: i64,
    pub count: usize,
    pub faces: u32,
}

impl Throw {
    /// Build a throw.
    pub const fn new(base: i64, count: usize, faces: u32) -> Self {
        Self { base, count, faces }
    }

    /// The dice as a reader writes them, `2D6` or `3D4`.
    pub fn notation(&self) -> String {
        format!("{}D{}", self.count, self.faces)
    }

    /// The smallest total this throw can give.
    pub fn lowest(&self) -> i64 {
        self.base + self.count as i64
    }

    /// The largest total this throw can give.
    pub fn highest(&self) -> i64 {
        self.base + self.count as i64 * self.faces as i64
    }

    /// Throw the dice and add the base.
    ///
    /// # Returns
    ///
    /// The total and the roll that produced it.
    pub fn roll(&self, rng: Option<&dyn Rng>) -> (i64, Roll) {
        let thrown = roll_dice(self.count, self.faces, rng);
        (self.base + thrown.total() as i64, thrown)
    }
}

/// One way of rolling a hero up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mode {
    pub name: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub force: Throw,
    pub vie: Throw,
}

pub const NORMAL: Mode = Mode {
    name: "normal",
    label: "Normale",
    description: "Les règles du livre.",
    force: Throw::new(6, 2, 6),
    vie: Throw::new(18, 2, 6),
};

pub const EASY: Mode = Mode {
    name: "easy",
    label: "Facile",
    description: "Un héros plus robuste, pour lire l'histoire sans y laisser la peau.",
    force: Throw::new(12, 2, 4),
    vie: Throw::new(26, 3, 4),
};

/// Every mode on offer, in the order the pages list them.
pub const MODES: [Mode; 2] = [NORMAL, EASY];

pub const DEFAULT_MODE: Mode = NORMAL;

// "Si, en calculant ce total de Force, vous avez obtenu 17, votre total bénéficie
// d'un Ajustement-Force": a high Force adds to the damage its blows do, the same
// way an adversary's own "AJUSTEMENT DOMMAGES" does.
//
// The book's table stops at 18 because its own Force does. The easy mode reaches
// 20, so the top step is read as "18 or more" rather than "18 exactly" — a hero
// who rolled higher than the table foresaw should not be worse off than one who
// rolled 18.
pub const FORCE_DAMAGE_ADJUSTMENTS: [(i64, i64); 2] = [(17, 1), (18, 2)];

/// The last step of the table, which is read as "that Force or more".
pub const TOP_ADJUSTMENT: (i64, i64) = FORCE_DAMAGE_ADJUSTMENTS[FORCE_DAMAGE_ADJUSTMENTS.len() - 1];

/// How much a Force of this size adds to the damage its blows do.
///
/// # Returns
///
/// The adjustment, `0` for a Force the table does not reach.
pub fn damage_adjustment(force: i64) -> i64 {
    let (top_force, top_adjustment) = TOP_ADJUSTMENT;
    if force >= top_force {
        return top_adjustment;
    }
    FORCE_DAMAGE_ADJUSTMENTS
        .iter()
        .find(|(step, _)| *step == force)
        .map_or(0, |(_, adjustment)| *adjustment)
}

/// A freshly rolled hero, and the dice that made him.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Character {
    pub mode: Mode,
    pub force: i64,
    pub vie_max: i64,
    pub vie_actuelle: i64,
    pub force_roll: Roll,
    pub vie_roll: Roll,
}

impl Character {
    /// How much this hero's Force adds to the damage he deals.
    pub fn damage_adjustment(&self) -> i64 {
        damage_adjustment(self.force)
    }
}

/// The mode a name asks for, the normal one for anything else.
///
/// A name comes off a form, so it is whatever was posted: an unknown one rolls a
/// hero by the book rather than refusing to roll one at all.
pub fn named_mode(name: Option<&str>) -> Mode {
    let asked = name.unwrap_or("").trim().to_lowercase();
    MODES
        .iter()
        .find(|mode| mode.name == asked)
        .copied()
        .unwrap_or(DEFAULT_MODE)
}

/// Roll up Prêtre Jean, once, at the start of a game.
///
/// # Returns
///
/// The hero, with full Vie and the two rolls that made him.
pub fn generate_character(mode: Mode, rng: Option<&dyn Rng>) -> Character {
    let (force, force_roll) = mode.force.roll(rng);
    let (vie, vie_roll) = mode.vie.roll(rng);
    Character {
        mode,
        force,
        vie_max: vie,
        vie_actuelle: vie,
        force_roll,
        vie_roll,
    }
}
