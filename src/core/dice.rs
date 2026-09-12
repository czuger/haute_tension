//! Dice, and what the rules read off them.
//!
//! The bottom of the game rules: everything else in `core` that needs chance
//! goes through here, so a test hands over dice of its own and the whole run
//! becomes deterministic.
//!
//! A roll is kept face by face rather than as a total, because the rules read
//! both: `8` says nothing, but `4 + 4` and `6 + 2` are the same total and only
//! one of them can be a double.
//!
//! Combat is always two six-sided dice ([`roll_2d6`]), which is the throw the
//! rules describe. [`roll_dice`] is the general one, and character creation is
//! what needs it: the easy mode rolls four-sided dice, and three of them for
//! Vie.

use std::fmt;
use std::sync::{Mutex, OnceLock};

use rand::rngs::StdRng;
use rand::{Rng as _, SeedableRng};

pub const DICE_PER_ROLL: usize = 2;
pub const DIE_FACES: u32 = 6;

/// Where a throw gets its faces.
///
/// The one seam chance comes through: a rule takes one of these rather than
/// reaching for a generator of its own, so a test can hand it faces chosen for
/// the rule it is checking.
pub trait Rng: Send + Sync {
    /// One face, between `low` and `high` inclusive.
    fn randint(&self, low: u32, high: u32) -> u32;
}

/// A seeded generator: the same seed gives the same run, twice over.
pub struct SeededRng(Mutex<StdRng>);

impl SeededRng {
    /// Build a generator on a seed.
    pub fn new(seed: u64) -> Self {
        Self(Mutex::new(StdRng::seed_from_u64(seed)))
    }
}

impl Rng for SeededRng {
    fn randint(&self, low: u32, high: u32) -> u32 {
        self.0
            .lock()
            .expect("the dice are not shared across a panic")
            .random_range(low..=high)
    }
}

/// The generator a call that names none falls back to.
fn default_rng() -> &'static SeededRng {
    static DEFAULT: OnceLock<SeededRng> = OnceLock::new();
    DEFAULT.get_or_init(|| SeededRng(Mutex::new(StdRng::from_os_rng())))
}

/// One throw, kept face by face.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Roll {
    dice: Vec<u32>,
}

impl Roll {
    /// Build a roll from the faces that came up.
    pub fn new(dice: impl Into<Vec<u32>>) -> Self {
        Self { dice: dice.into() }
    }

    /// The faces, in the order they were thrown.
    pub fn dice(&self) -> &[u32] {
        &self.dice
    }

    /// The sum of the faces.
    pub fn total(&self) -> u32 {
        self.dice.iter().sum()
    }

    /// Whether every face came up six — the hero's divine judgement.
    ///
    /// Read on combat throws only, which are always two six-sided dice. It is
    /// not asked of a creation throw, whose dice may have four faces and could
    /// never answer yes.
    pub fn is_double_six(&self) -> bool {
        self.dice.iter().all(|&die| die == DIE_FACES)
    }

    /// Whether every face came up one — the adversary's divine judgement.
    ///
    /// Read on combat throws only. Three four-sided dice can all come up one,
    /// and that throw rolls up a hero rather than settling an assault.
    pub fn is_double_one(&self) -> bool {
        self.dice.iter().all(|&die| die == 1)
    }
}

impl fmt::Display for Roll {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let faces: Vec<String> = self.dice.iter().map(|die| die.to_string()).collect();
        write!(formatter, "{} = {}", faces.join(" + "), self.total())
    }
}

/// Throw `count` dice of `faces` faces each.
///
/// # Arguments
///
/// * `count` - How many dice.
/// * `faces` - How many faces each of them has.
/// * `rng` - Source of chance. Seed one to make a run reproducible; `None` is a
///   process-wide generator.
pub fn roll_dice(count: usize, faces: u32, rng: Option<&dyn Rng>) -> Roll {
    let generator: &dyn Rng = rng.unwrap_or_else(|| default_rng());
    Roll::new(
        (0..count)
            .map(|_| generator.randint(1, faces))
            .collect::<Vec<u32>>(),
    )
}

/// Throw two six-sided dice — the throw every rule of combat is written on.
pub fn roll_2d6(rng: Option<&dyn Rng>) -> Roll {
    roll_dice(DICE_PER_ROLL, DIE_FACES, rng)
}
