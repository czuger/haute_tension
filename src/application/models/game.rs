//! The shape of a play-through, as the routes and templates read it.
//!
//! The dict side of `core::models::game`: `core::db` converts on the way out, so
//! nothing above it holds a row. Same role `StoryPage` plays for the book.

/// What the session cookie carries, and all it carries: the id of the
/// play-through. Read by the routes that drive a game and by the request trace,
/// which is why it sits here rather than in either.
pub const SESSION_KEY: &str = "game_id";

/// What happened between the hero and one adversary in one assault.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exchange {
    pub enemy_name: String,
    pub enemy_force: i64,
    pub enemy_dice: Vec<i64>,
    pub enemy_attack_force: i64,
    /// `"hero"`, `"enemy"`, or nothing when the two Forces d'Attaque were equal.
    pub winner: Option<String>,
    pub damage: i64,
    pub divine_judgement: bool,
}

/// One assault: the hero's single roll, and every exchange it settled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assault {
    pub number: i64,
    pub hero_dice: Vec<i64>,
    pub hero_attack_force: i64,
    pub exchanges: Vec<Exchange>,
}

/// One adversary of the current fight, as it stands.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Enemy {
    pub name: String,
    pub force: i64,
    pub vie_max: i64,
    pub vie_actuelle: i64,
    pub damage_adjustment: i64,
}

/// The fight the hero is in, and everything it has cost so far.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Combat {
    pub page: String,
    pub fight_type: String,
    pub status: String,
    pub on_victory: Option<String>,
    pub on_defeat: Option<String>,
    pub on_flee: Option<String>,
    pub has_special_rules: bool,
    pub enemies: Vec<Enemy>,
    pub assaults: Vec<Assault>,
}

/// One thing in the hero's bag, and how many of it he has.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    pub element: String,
    pub label: String,
    pub count: i64,
}

/// A gain or loss waiting on the reader's word.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pending {
    pub index: usize,
    pub element: String,
    pub label: String,
    pub amount: Option<i64>,
    pub condition: Option<String>,
    pub note: Option<String>,
    pub sign: i64,
    pub page: Option<String>,
    pub described: String,
}

/// A hero, rolled once, and the fight he is in the middle of.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameDict {
    pub id: i64,
    pub book: String,
    pub mode: String,
    pub mode_label: String,
    pub force_throw: String,
    pub vie_throw: String,
    pub force_base: i64,
    pub vie_base: i64,
    pub force: i64,
    pub vie_max: i64,
    pub vie_actuelle: i64,
    pub force_dice: Vec<i64>,
    pub vie_dice: Vec<i64>,
    pub damage_adjustment: i64,
    pub gold: i64,
    pub gold_dice: Vec<i64>,
    /// Named `bag` and not `items`, the way the dict was: what the hero carries
    /// is one thing, and the rows it is kept in are another.
    pub bag: Vec<Item>,
    pub pending: Vec<Pending>,
    pub combat: Option<Combat>,
    pub is_dead: bool,
    pub died_on_page: Option<String>,
    pub died_of: Option<String>,
}

impl GameDict {
    /// How many things the hero carries, all lines counted together.
    pub fn carried(&self) -> i64 {
        self.bag.iter().map(|item| item.count).sum()
    }
}

/// One hero who did not come back, as the memorial lists him.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FallenHero {
    pub id: i64,
    pub mode_label: String,
    pub force: i64,
    pub vie_max: i64,
    pub gold: i64,
    pub bag: Vec<Item>,
    pub died_on_page: Option<String>,
    pub died_of: Option<String>,
    pub died_at: Option<String>,
}
