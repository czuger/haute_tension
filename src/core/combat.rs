//! The combat engine: one assault at a time, and nothing else.
//!
//! Pure: no database, no HTTP, no clock. It is handed the state of a fight and
//! gives back the state after one assault, so a seeded generator makes a whole
//! combat reproducible and every rule below is testable on its own.
//!
//! The rules, from `regles-du-jeu-spj1`:
//!
//! - Force d'Attaque — "jetez deux dés. Ajoutez au résultat obtenu votre total
//!   de Force du moment."
//! - Damage — "retirez la Force d'Attaque la plus faible de la Force d'Attaque
//!   la plus élevée. Le chiffre obtenu constitue le nombre de points de Vie de
//!   dommages infligé." Damage is the *gap*, not a fixed toll, so an assault won
//!   by one point barely stings and one won by ten is close to lethal.
//! - Divine judgement — "Si vous tirez un double 6, vous avez instantanément tué
//!   votre adversaire. Si, en revanche, votre adversaire tire un double 1, c'est
//!   lui qui a mis fin à vos jours."
//!
//! Two things the rules leave open, decided here and not read off the book:
//!
//! - **Equal Forces d'Attaque cost nobody anything.** The rules only ever
//!   describe damage as the gap between the two, and the gap is zero.
//! - **A hero's double 6 outranks an adversary's double 1 in the same assault.**
//!   An adversary killed outright does not get to land its own blow. The book
//!   never puts the two together, so this is a choice; it is at least the one
//!   that reads the way the assault is written down, hero first.
//!
//! [`FIGHTS_WITH_SPECIAL_RULES`] lists the pages this engine knowingly does not
//! model; `TODO.md` says what each of them adds.

use crate::core::dice::{roll_2d6, Rng, Roll};

pub const SINGLE: &str = "single";
pub const SEQUENTIAL: &str = "sequential";
pub const SIMULTANEOUS: &str = "simultaneous";

pub const ONGOING: &str = "ongoing";
pub const VICTORY: &str = "victory";
pub const DEFEAT: &str = "defeat";

pub const HERO: &str = "hero";
pub const ENEMY: &str = "enemy";

// Pages whose text adds a rule this engine does not implement: a fight that ends
// on the first assault instead of on a death (133), a limited number of assaults
// and a Force halved under water (335, 396, 429, 595), a branch on the first
// wound taken (74) or on dropping below a Vie threshold (425, 627), a Force that
// changes mid-fight on every 6 rolled (335, 396, 595), or a flight allowed only
// once one adversary is down (609).
//
// They resolve here as plain fights, which is wrong for them and right for the
// other 34. Handle them before trusting a result on these pages.
pub const FIGHTS_WITH_SPECIAL_RULES: [&str; 13] = [
    "40", "74", "133", "241", "324", "335", "396", "425", "429", "540", "595", "609", "627",
];

/// Whether a page's fight adds a rule the engine does not model.
pub fn has_special_rules(page: &str) -> bool {
    FIGHTS_WITH_SPECIAL_RULES.contains(&page)
}

/// One side of a fight, hero or adversary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Combatant {
    pub name: String,
    pub force: i64,
    pub vie_max: i64,
    pub vie_actuelle: i64,
    pub damage_adjustment: i64,
}

impl Combatant {
    /// Build a combatant.
    pub fn new(
        name: impl Into<String>,
        force: i64,
        vie_max: i64,
        vie_actuelle: i64,
        damage_adjustment: i64,
    ) -> Self {
        Self {
            name: name.into(),
            force,
            vie_max,
            vie_actuelle,
            damage_adjustment,
        }
    }

    /// Whether this combatant still has Vie left.
    pub fn is_alive(&self) -> bool {
        self.vie_actuelle > 0
    }

    /// Return this combatant with `damage` taken off, never below zero.
    pub fn wounded_by(&self, damage: i64) -> Self {
        Self {
            vie_actuelle: (self.vie_actuelle - damage).max(0),
            ..self.clone()
        }
    }

    /// Return this combatant killed outright, whatever its Vie was.
    pub fn slain(&self) -> Self {
        Self {
            vie_actuelle: 0,
            ..self.clone()
        }
    }
}

/// What happened between the hero and one adversary in one assault.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exchange {
    pub enemy_name: String,
    pub enemy_force: i64,
    pub enemy_roll: Roll,
    pub enemy_attack_force: i64,
    /// [`HERO`], [`ENEMY`], or nothing when the two Forces d'Attaque were equal.
    pub winner: Option<&'static str>,
    pub damage: i64,
    pub divine_judgement: bool,
}

/// One full assault: the hero's roll, and every exchange it settled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assault {
    pub number: i64,
    pub hero_roll: Roll,
    pub hero_attack_force: i64,
    pub exchanges: Vec<Exchange>,
}

/// Roll a Force d'Attaque.
///
/// # Returns
///
/// The Force d'Attaque and the roll behind it.
pub fn attack_force(force: i64, rng: Option<&dyn Rng>) -> (i64, Roll) {
    let roll = roll_2d6(rng);
    (force + roll.total() as i64, roll)
}

/// Which adversaries the hero faces this assault.
///
/// A `sequential` fight is a queue: the hero meets the next adversary only once
/// the one before it is down. Anything else puts every survivor in front of him
/// at once, which is what `single` (one adversary) and `simultaneous` both mean.
///
/// # Returns
///
/// Indexes into `enemies`, in order.
pub fn engaged_indexes(enemies: &[Combatant], fight_type: &str) -> Vec<usize> {
    let alive: Vec<usize> = enemies
        .iter()
        .enumerate()
        .filter(|(_, enemy)| enemy.is_alive())
        .map(|(index, _)| index)
        .collect();
    if fight_type == SEQUENTIAL {
        return alive.into_iter().take(1).collect();
    }
    alive
}

/// Play one assault and report it.
///
/// The hero rolls **once** and that single Force d'Attaque is compared to each
/// engaged adversary's own, which is the rule the book prints on the page:
/// "lancez deux dés pour vous et deux dés pour chacun de vos adversaires".
///
/// # Returns
///
/// The hero after the assault, the adversaries after it, and the record of what
/// happened.
pub fn resolve_assault(
    hero: &Combatant,
    enemies: &[Combatant],
    fight_type: &str,
    number: i64,
    rng: Option<&dyn Rng>,
) -> (Combatant, Vec<Combatant>, Assault) {
    let (hero_force_attaque, hero_roll) = attack_force(hero.force, rng);
    let mut assault = Assault {
        number,
        hero_roll: hero_roll.clone(),
        hero_attack_force: hero_force_attaque,
        exchanges: Vec::new(),
    };
    let mut survivors = enemies.to_vec();
    let mut standing = hero.clone();

    for index in engaged_indexes(enemies, fight_type) {
        let enemy = survivors[index].clone();
        let (enemy_force_attaque, enemy_roll) = attack_force(enemy.force, rng);
        let (exchange, after_hero, after_enemy) = resolve_exchange(
            &standing,
            &enemy,
            &hero_roll,
            hero_force_attaque,
            enemy_roll,
            enemy_force_attaque,
        );
        standing = after_hero;
        survivors[index] = after_enemy;
        assault.exchanges.push(exchange);
        if !standing.is_alive() {
            break;
        }
    }

    (standing, survivors, assault)
}

/// Say whether the fight is over, and for whom.
///
/// # Returns
///
/// [`ONGOING`], [`VICTORY`] when every adversary is down, or [`DEFEAT`].
pub fn combat_status(hero: &Combatant, enemies: &[Combatant]) -> &'static str {
    if !hero.is_alive() {
        return DEFEAT;
    }
    if enemies.iter().all(|enemy| !enemy.is_alive()) {
        return VICTORY;
    }
    ONGOING
}

/// Settle the hero against one adversary.
///
/// A divine judgement short-circuits the comparison entirely: no gap is computed
/// and no adjustment applies, the loser simply dies.
///
/// # Returns
///
/// The record of the exchange, the hero after it, and the adversary after it.
fn resolve_exchange(
    hero: &Combatant,
    enemy: &Combatant,
    hero_roll: &Roll,
    hero_force_attaque: i64,
    enemy_roll: Roll,
    enemy_force_attaque: i64,
) -> (Exchange, Combatant, Combatant) {
    if hero_roll.is_double_six() {
        return (
            judgement(enemy, enemy_roll, enemy_force_attaque, HERO),
            hero.clone(),
            enemy.slain(),
        );
    }
    if enemy_roll.is_double_one() {
        return (
            judgement(enemy, enemy_roll, enemy_force_attaque, ENEMY),
            hero.slain(),
            enemy.clone(),
        );
    }

    let gap = hero_force_attaque - enemy_force_attaque;
    if gap > 0 {
        let damage = gap + hero.damage_adjustment;
        return (
            Exchange {
                enemy_name: enemy.name.clone(),
                enemy_force: enemy.force,
                enemy_roll,
                enemy_attack_force: enemy_force_attaque,
                winner: Some(HERO),
                damage,
                divine_judgement: false,
            },
            hero.clone(),
            enemy.wounded_by(damage),
        );
    }
    if gap < 0 {
        let damage = -gap + enemy.damage_adjustment;
        return (
            Exchange {
                enemy_name: enemy.name.clone(),
                enemy_force: enemy.force,
                enemy_roll,
                enemy_attack_force: enemy_force_attaque,
                winner: Some(ENEMY),
                damage,
                divine_judgement: false,
            },
            hero.wounded_by(damage),
            enemy.clone(),
        );
    }
    (
        Exchange {
            enemy_name: enemy.name.clone(),
            enemy_force: enemy.force,
            enemy_roll,
            enemy_attack_force: enemy_force_attaque,
            winner: None,
            damage: 0,
            divine_judgement: false,
        },
        hero.clone(),
        enemy.clone(),
    )
}

/// Build the record of a kill by divine judgement.
fn judgement(
    enemy: &Combatant,
    enemy_roll: Roll,
    enemy_force_attaque: i64,
    winner: &'static str,
) -> Exchange {
    Exchange {
        enemy_name: enemy.name.clone(),
        enemy_force: enemy.force,
        enemy_roll,
        enemy_attack_force: enemy_force_attaque,
        winner: Some(winner),
        damage: 0,
        divine_judgement: true,
    }
}
