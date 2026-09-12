//! What a gain or a loss means, and what the hero carries.
//!
//! Pure: no database, no HTTP. It reads the `gains` and `losses` the import
//! pipeline attached to every choice and says what they do to a hero.
//!
//! Three elements are not things in a bag — they are the hero himself:
//!
//! ```text
//! life point      -> Vie, which may never rise above the Vie he started with
//! strength point  -> Force
//! gold coin       -> the purse
//! ```
//!
//! Everything else is an item, counted: two vials are one line saying two.
//!
//! **A change carrying a `condition` is never applied here.** The conditions are
//! free French — "Si vous avez des provisions dans votre sac", "pendant tout le
//! temps où vous porterez cette cuirasse" — a prerequisite or a duration, and
//! neither is anything code can evaluate. They are handed to the reader, who is
//! the one who knows. `note: "dice"` is the same case by another road: the
//! amount is rolled, and how many dice the page's text says.
//!
//! `note: "all"` is the one note that *is* machine-readable: everything of that
//! element goes, however much there was.
//!
//! The starting hero, from `regles-du-jeu-spj1`: "vous êtes équipé de votre épée
//! et d'un sac", "4 rations de provisions", and a purse of two throws of two
//! dice — "vous voilà donc riche d'un montant allant de 4 à 24 pièces d'or".

use indexmap::IndexMap;
use serde_json::Value;

use crate::core::dice::{roll_2d6, Rng, Roll};

pub const LIFE: &str = "life point";
pub const STRENGTH: &str = "strength point";
pub const GOLD: &str = "gold coin";

/// The three elements that change the hero rather than what he carries.
pub const CHARACTERISTICS: [&str; 3] = [LIFE, STRENGTH, GOLD];

/// The one note the code can act on: take all of it, whatever the amount was.
pub const ALL: &str = "all";
/// The note that means the page's text says how many dice to roll for the amount.
pub const DICE: &str = "dice";

pub const GOLD_THROWS: usize = 2;

pub const STARTING_ITEMS: [(&str, &str, i64); 3] = [
    ("sword", "épée", 1),
    ("bag", "sac", 1),
    ("ration", "ration", 4),
];

/// One gain or loss, as a choice carries it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub element: String,
    pub label: String,
    pub amount: Option<i64>,
    pub condition: Option<String>,
    pub note: Option<String>,
    /// A gain adds, a loss takes away. The sign the caller read off the key.
    pub sign: i64,
}

impl Change {
    /// Build a change, the way a test or the pipeline names one.
    pub fn new(
        element: impl Into<String>,
        label: impl Into<String>,
        amount: Option<i64>,
        sign: i64,
    ) -> Self {
        Self {
            element: element.into(),
            label: label.into(),
            amount,
            condition: None,
            note: None,
            sign,
        }
    }

    /// The same change, carrying a condition only the reader can judge.
    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.condition = Some(condition.into());
        self
    }

    /// The same change, carrying one of the two notes the pipeline writes.
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }

    /// Whether only the reader can say if, or how much, this applies.
    ///
    /// A stated condition, or an amount the page's text asks to be rolled: both
    /// need someone who has read the page.
    pub fn is_conditional(&self) -> bool {
        self.condition.is_some() || self.note.as_deref() == Some(DICE)
    }

    /// Whether this empties the element rather than moving it by an amount.
    pub fn takes_everything(&self) -> bool {
        self.note.as_deref() == Some(ALL)
    }

    /// `+` for a gain, `-` for a loss.
    pub fn marker(&self) -> &'static str {
        if self.sign > 0 {
            "+"
        } else {
            "-"
        }
    }

    /// The change as a reader reads it: `-2 points de Vie`, `tout l'or`.
    pub fn described(&self) -> String {
        if self.takes_everything() {
            return format!("{} tout ({})", self.marker(), self.label);
        }
        match self.amount {
            None => format!("{}? {}", self.marker(), self.label),
            Some(amount) => format!(
                "{}{} {}",
                self.marker(),
                amount,
                pluralised(&self.label, amount)
            ),
        }
    }
}

/// What a hero is and carries, as far as gains and losses are concerned.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Holdings {
    pub force: i64,
    pub vie_max: i64,
    pub vie_actuelle: i64,
    pub gold: i64,
    /// element -> (label, count), in the order things were first picked up.
    pub items: IndexMap<String, (String, i64)>,
}

impl Holdings {
    /// Return these holdings with one change applied.
    ///
    /// # Arguments
    ///
    /// * `change` - The gain or loss to apply. A conditional one is applied here
    ///   too — deciding whether it *should* be is the caller's job, and
    ///   `core::db` only ever hands over the ones the reader agreed to.
    pub fn with_change(&self, change: &Change) -> Self {
        if change.element == LIFE {
            return Self {
                vie_actuelle: self.moved_vie(change),
                ..self.clone()
            };
        }
        if change.element == STRENGTH {
            return Self {
                force: (self.force + self.delta(change)).max(0),
                ..self.clone()
            };
        }
        if change.element == GOLD {
            return Self {
                gold: (self.gold + self.delta(change)).max(0),
                ..self.clone()
            };
        }
        Self {
            items: self.moved_items(change),
            ..self.clone()
        }
    }

    /// Vie after a change, floored at zero and capped at the starting total.
    ///
    /// "vous devrez toujours garder trace de votre total de Vie de départ […]
    /// Vous ne pourrez le dépasser en aucun cas": a hero healed past what he
    /// began with keeps only what he began with.
    fn moved_vie(&self, change: &Change) -> i64 {
        let moved = self.vie_actuelle + self.delta(change);
        moved.min(self.vie_max).max(0)
    }

    /// The bag after a change, an item dropped once none of it is left.
    fn moved_items(&self, change: &Change) -> IndexMap<String, (String, i64)> {
        let mut items = self.items.clone();
        let (label, held) = items
            .get(&change.element)
            .cloned()
            .unwrap_or_else(|| (change.label.clone(), 0));
        let left = if change.takes_everything() && change.sign < 0 {
            0
        } else {
            held + self.delta(change)
        };
        if left <= 0 {
            items.shift_remove(&change.element);
        } else {
            items.insert(change.element.clone(), (label, left));
        }
        items
    }

    /// How far a change moves a number, and in which direction.
    ///
    /// `takes_everything` is answered here for a characteristic — losing "all"
    /// of one's gold empties the purse — and by `moved_items` for a thing in the
    /// bag.
    fn delta(&self, change: &Change) -> i64 {
        if change.takes_everything() {
            return if change.sign < 0 {
                -self.held(&change.element)
            } else {
                0
            };
        }
        change.sign * change.amount.unwrap_or(0)
    }

    /// How much of a characteristic the hero has right now.
    ///
    /// Only ever asked of the three: a thing in the bag that is taken away
    /// entirely is emptied by `moved_items`, which sets it to zero without
    /// needing to know how much was there.
    fn held(&self, element: &str) -> i64 {
        match element {
            GOLD => self.gold,
            LIFE => self.vie_actuelle,
            STRENGTH => self.force,
            other => unreachable!("{other} is not one of {CHARACTERISTICS:?}"),
        }
    }
}

/// Every gain and loss a choice carries, gains first.
///
/// # Arguments
///
/// * `choice` - One choice, as the book stores it.
pub fn read_changes(choice: &serde_json::Map<String, Value>) -> Vec<Change> {
    let mut changes = Vec::new();
    for (key, sign) in [("gains", 1_i64), ("losses", -1_i64)] {
        let Some(entries) = choice.get(key).and_then(Value::as_array) else {
            continue;
        };
        for entry in entries {
            if let Some(entry) = entry.as_object() {
                changes.push(read_change(entry, sign));
            }
        }
    }
    changes
}

/// Roll the purse the hero sets out with: two throws of two dice.
///
/// # Returns
///
/// The total, 4 to 24, and the throws that made it.
pub fn starting_gold(rng: Option<&dyn Rng>) -> (i64, Vec<Roll>) {
    let throws: Vec<Roll> = (0..GOLD_THROWS).map(|_| roll_2d6(rng)).collect();
    let total = throws.iter().map(|throw| throw.total() as i64).sum();
    (total, throws)
}

/// Build one change from the dict the pipeline wrote.
fn read_change(entry: &serde_json::Map<String, Value>, sign: i64) -> Change {
    let element = entry
        .get("element")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let label = entry
        .get("label_fr")
        .and_then(Value::as_str)
        .filter(|label| !label.is_empty())
        .map_or_else(|| element.clone(), str::to_string);
    Change {
        element,
        label,
        // Only a whole number is an amount: a float, a string or a null all read
        // as "no amount".
        amount: entry.get("amount").and_then(|amount| match amount {
            Value::Number(number) if number.is_i64() => number.as_i64(),
            _ => None,
        }),
        condition: text(entry.get("condition")),
        note: text(entry.get("note")),
        sign,
    }
}

/// A French label for a count: `2 points de Vie`, not `2 point de Vies`.
///
/// French marks the plural on the head noun, which is the first word here —
/// "pièce d'or" becomes "pièces d'or" and "point de Vie" becomes "points de
/// Vie". Every label in the book is a head noun followed by a complement, so the
/// first word is the one to move.
pub fn pluralised(label: &str, count: i64) -> String {
    if count <= 1 || label.is_empty() {
        return label.to_string();
    }
    let (head, rest) = match label.split_once(' ') {
        Some((head, rest)) => (head, rest),
        None => (label, ""),
    };
    if head.ends_with('s') || head.ends_with('x') {
        return label.to_string();
    }
    format!("{head}s {rest}").trim_end().to_string()
}

/// A field's text, or nothing when it holds nothing worth reading.
fn text(value: Option<&Value>) -> Option<String> {
    let written = match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(text)) => text.trim().to_string(),
        Some(other) => other.to_string().trim().to_string(),
    };
    (!written.is_empty()).then_some(written)
}
