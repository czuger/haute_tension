//! What a gain or a loss does to a hero.

#[path = "common/mod.rs"]
mod common;

use std::sync::OnceLock;

use common::{fixed, packaged_books, seeded, BOOK};
use haute_tension::core::inventory::{
    pluralised, read_changes, starting_gold, Change, Holdings, GOLD, LIFE, STRENGTH,
};
use indexmap::IndexMap;
use serde_json::{Map, Value};

/// A hero carrying whatever the test cares about.
fn holdings(force: i64, vie_max: i64, vie_actuelle: i64, gold: i64, items: &[(&str, i64)]) -> Holdings {
    Holdings {
        force,
        vie_max,
        vie_actuelle,
        gold,
        items: items
            .iter()
            .map(|(name, count)| ((*name).to_string(), ((*name).to_string(), *count)))
            .collect(),
    }
}

/// The hero the tests that care about nothing else use.
fn plain() -> Holdings {
    holdings(12, 25, 25, 10, &[])
}

fn gain(element: &str, amount: i64) -> Change {
    Change::new(element, element, Some(amount), 1)
}

fn loss(element: &str, amount: i64) -> Change {
    Change::new(element, element, Some(amount), -1)
}

/// One choice, as the book stores it.
fn choice(written: Value) -> Map<String, Value> {
    written.as_object().cloned().unwrap_or_default()
}

/// What the bag reads as, for a comparison against a spelled-out one.
fn bag(items: &[(&str, &str, i64)]) -> IndexMap<String, (String, i64)> {
    items
        .iter()
        .map(|(element, label, count)| {
            ((*element).to_string(), ((*label).to_string(), *count))
        })
        .collect()
}

/// Every gain and loss a choice carries.
mod reading_changes {
    use super::*;

    #[test]
    fn gains_and_losses_are_both_read() {
        let changes = read_changes(&choice(serde_json::json!({
            "gains": [{"element": "key", "label_fr": "clé", "amount": 1}],
            "losses": [{"element": GOLD, "label_fr": "pièce d'or", "amount": 3}],
        })));

        let elements: Vec<&str> = changes.iter().map(|change| change.element.as_str()).collect();
        assert_eq!(elements, ["key", GOLD]);
        let signs: Vec<i64> = changes.iter().map(|change| change.sign).collect();
        assert_eq!(signs, [1, -1]);
    }

    #[test]
    fn a_choice_carrying_nothing_gives_nothing() {
        assert!(read_changes(&choice(serde_json::json!({"goto": "2"}))).is_empty());
        assert!(read_changes(&choice(
            serde_json::json!({"gains": null, "losses": []})
        ))
        .is_empty());
    }

    #[test]
    fn the_french_label_is_kept() {
        let changes = read_changes(&choice(serde_json::json!({
            "gains": [{"element": "key", "label_fr": "clé", "amount": 1}]
        })));

        assert_eq!(changes[0].label, "clé");
    }

    #[test]
    fn a_missing_label_falls_back_to_the_element() {
        let changes = read_changes(&choice(serde_json::json!({
            "gains": [{"element": "key", "amount": 1}]
        })));

        assert_eq!(changes[0].label, "key");
    }

    #[test]
    fn a_malformed_entry_is_dropped() {
        assert!(read_changes(&choice(serde_json::json!({"gains": ["clé"]}))).is_empty());
    }

    #[test]
    fn a_condition_is_kept() {
        let changes = read_changes(&choice(serde_json::json!({
            "losses": [{
                "element": "ration",
                "label_fr": "ration",
                "amount": 1,
                "condition": "Si vous avez des provisions",
            }]
        })));

        assert_eq!(
            changes[0].condition.as_deref(),
            Some("Si vous avez des provisions")
        );
        assert!(changes[0].is_conditional());
    }

    #[test]
    fn a_blank_condition_is_no_condition() {
        let changes = read_changes(&choice(serde_json::json!({
            "gains": [{"element": "key", "amount": 1, "condition": "  "}]
        })));

        assert_eq!(changes[0].condition, None);
        assert!(!changes[0].is_conditional());
    }

    /// `note: "dice"` means the page's text says how many dice.
    #[test]
    fn a_rolled_amount_is_conditional_too() {
        let changes = read_changes(&choice(serde_json::json!({
            "losses": [{"element": LIFE, "amount": null, "note": "dice"}]
        })));

        assert_eq!(changes[0].amount, None);
        assert!(changes[0].is_conditional());
    }

    /// `note: "all"` is the one note the code can act on.
    #[test]
    fn taking_everything_is_not_conditional() {
        let changes = read_changes(&choice(serde_json::json!({
            "losses": [{"element": GOLD, "amount": null, "note": "all"}]
        })));

        assert!(changes[0].takes_everything());
        assert!(!changes[0].is_conditional());
    }
}

/// The three elements that are the hero rather than his bag.
mod characteristics {
    use super::*;

    #[test]
    fn a_life_gain_heals() {
        let after = holdings(12, 25, 10, 10, &[]).with_change(&gain(LIFE, 4));

        assert_eq!(after.vie_actuelle, 14);
    }

    /// "votre total de Vie de départ est immuable […] Vous ne pourrez le
    /// dépasser en aucun cas".
    #[test]
    fn a_life_gain_never_passes_the_starting_total() {
        let after = holdings(12, 25, 22, 10, &[]).with_change(&gain(LIFE, 40));

        assert_eq!(after.vie_actuelle, 25);
    }

    #[test]
    fn a_life_loss_wounds() {
        let after = holdings(12, 25, 20, 10, &[]).with_change(&loss(LIFE, 6));

        assert_eq!(after.vie_actuelle, 14);
    }

    #[test]
    fn life_never_goes_below_zero() {
        let after = holdings(12, 25, 2, 10, &[]).with_change(&loss(LIFE, 9));

        assert_eq!(after.vie_actuelle, 0);
    }

    #[test]
    fn the_starting_total_never_moves() {
        let after = holdings(12, 25, 25, 10, &[]).with_change(&loss(LIFE, 5));

        assert_eq!(after.vie_max, 25);
    }

    #[test]
    fn strength_moves_both_ways() {
        assert_eq!(plain().with_change(&gain(STRENGTH, 2)).force, 14);
        assert_eq!(plain().with_change(&loss(STRENGTH, 3)).force, 9);
    }

    #[test]
    fn strength_never_goes_below_zero() {
        let after = holdings(2, 25, 25, 10, &[]).with_change(&loss(STRENGTH, 9));

        assert_eq!(after.force, 0);
    }

    #[test]
    fn gold_moves_both_ways() {
        assert_eq!(plain().with_change(&gain(GOLD, 5)).gold, 15);
        assert_eq!(plain().with_change(&loss(GOLD, 4)).gold, 6);
    }

    /// A hero cannot pay what he has not got.
    #[test]
    fn gold_never_goes_below_zero() {
        let after = holdings(12, 25, 25, 3, &[]).with_change(&loss(GOLD, 9));

        assert_eq!(after.gold, 0);
    }

    #[test]
    fn losing_all_the_gold_empties_the_purse() {
        let after = holdings(12, 25, 25, 17, &[]).with_change(
            &Change::new(GOLD, "pièce d'or", None, -1).with_note("all"),
        );

        assert_eq!(after.gold, 0);
    }

    #[test]
    fn a_characteristic_never_reaches_the_bag() {
        assert!(plain().with_change(&gain(LIFE, 2)).items.is_empty());
    }
}

/// Everything that is a thing.
mod the_bag {
    use super::*;

    #[test]
    fn a_gain_puts_something_in() {
        let after = plain().with_change(&gain("key", 1));

        assert_eq!(after.items, bag(&[("key", "key", 1)]));
    }

    #[test]
    fn a_second_one_stacks() {
        let after = holdings(12, 25, 25, 10, &[("key", 1)]).with_change(&gain("key", 2));

        assert_eq!(after.items["key"], ("key".to_string(), 3));
    }

    #[test]
    fn a_loss_takes_some_out() {
        let after =
            holdings(12, 25, 25, 10, &[("ration", 4)]).with_change(&loss("ration", 1));

        assert_eq!(after.items["ration"], ("ration".to_string(), 3));
    }

    #[test]
    fn the_last_one_leaves_the_bag() {
        let after =
            holdings(12, 25, 25, 10, &[("ration", 1)]).with_change(&loss("ration", 1));

        assert!(!after.items.contains_key("ration"));
    }

    #[test]
    fn losing_more_than_is_held_leaves_the_bag() {
        let after =
            holdings(12, 25, 25, 10, &[("ration", 2)]).with_change(&loss("ration", 9));

        assert!(!after.items.contains_key("ration"));
    }

    #[test]
    fn losing_what_is_not_held_changes_nothing() {
        let after = holdings(12, 25, 25, 10, &[("ration", 4)]).with_change(&loss("key", 1));

        assert_eq!(after.items, bag(&[("ration", "ration", 4)]));
    }

    #[test]
    fn losing_all_of_something_empties_it() {
        let after = holdings(12, 25, 25, 10, &[("ration", 4)]).with_change(
            &Change::new("ration", "ration", None, -1).with_note("all"),
        );

        assert!(!after.items.contains_key("ration"));
    }

    #[test]
    fn the_first_label_is_the_one_kept() {
        let after = plain().with_change(&Change::new("key", "clé", Some(1), 1));

        assert_eq!(after.items["key"], ("clé".to_string(), 1));
    }

    #[test]
    fn the_original_holdings_are_left_alone() {
        let before = holdings(12, 25, 25, 10, &[("ration", 4)]);
        before.with_change(&loss("ration", 4));

        assert_eq!(before.items["ration"], ("ration".to_string(), 4));
    }
}

/// "lancez deux dés deux fois de suite".
mod the_starting_purse {
    use super::*;

    #[test]
    fn it_is_two_throws_of_two_dice() {
        let (total, throws) = starting_gold(Some(&fixed(&[3, 4, 5, 2])));

        let faces: Vec<Vec<u32>> = throws.iter().map(|throw| throw.dice().to_vec()).collect();
        assert_eq!(faces, [vec![3, 4], vec![5, 2]]);
        assert_eq!(total, 14);
    }

    #[test]
    fn it_never_leaves_four_to_twenty_four() {
        let totals: Vec<i64> = (0..400)
            .map(|seed| starting_gold(Some(&seeded(seed))).0)
            .collect();

        assert!(totals.iter().min().copied().unwrap_or(0) >= 4);
        assert!(totals.iter().max().copied().unwrap_or(0) <= 24);
    }

    /// Seeding for it would be a lottery: a total of 4 is one run in 1296.
    #[test]
    fn the_floor_is_four_dice_at_one() {
        assert_eq!(starting_gold(Some(&fixed(&[1, 1, 1, 1]))).0, 4);
    }

    #[test]
    fn the_ceiling_is_four_dice_at_six() {
        assert_eq!(starting_gold(Some(&fixed(&[6, 6, 6, 6]))).0, 24);
    }
}

/// A change as a reader reads it.
mod describing {
    use super::*;

    #[test]
    fn a_gain_is_written_with_a_plus() {
        assert_eq!(Change::new("key", "clé", Some(2), 1).described(), "+2 clés");
    }

    #[test]
    fn a_loss_is_written_with_a_minus() {
        assert_eq!(Change::new("key", "clé", Some(1), -1).described(), "-1 clé");
    }

    #[test]
    fn taking_everything_says_so() {
        let written = Change::new(GOLD, "pièce d'or", None, -1)
            .with_note("all")
            .described();

        assert!(written.contains("tout"));
    }

    #[test]
    fn a_rolled_amount_says_it_is_unknown() {
        let written = Change::new(LIFE, "point de Vie", None, -1)
            .with_note("dice")
            .described();

        assert_eq!(written, "-? point de Vie");
    }

    #[test]
    fn french_marks_the_plural_on_the_head_noun() {
        for (label, count, expected) in [
            ("point de Vie", 2, "points de Vie"),
            ("pièce d'or", 5, "pièces d'or"),
            ("ration", 3, "rations"),
            ("ration", 1, "ration"),
            ("gantelets magiques", 2, "gantelets magiques"),
            ("", 3, ""),
        ] {
            assert_eq!(pluralised(label, count), expected, "for {label} × {count}");
        }
    }
}

/// Every change the book actually carries goes through this.
mod the_whole_book {
    use super::*;

    /// Every gain and loss of the packaged book.
    fn book_changes() -> &'static Vec<Change> {
        static CHANGES: OnceLock<Vec<Change>> = OnceLock::new();
        CHANGES.get_or_init(|| {
            let pages: Value = serde_json::from_str(
                &std::fs::read_to_string(packaged_books().join(BOOK).join("pages.json"))
                    .expect("the packaged book is on disk"),
            )
            .expect("the packaged book is valid JSON");
            pages
                .as_array()
                .map(|pages| {
                    pages
                        .iter()
                        .filter_map(Value::as_object)
                        .flat_map(|page| {
                            page.get("choices")
                                .and_then(Value::as_array)
                                .cloned()
                                .unwrap_or_default()
                        })
                        .filter_map(|choice| choice.as_object().cloned())
                        .flat_map(|choice| read_changes(&choice))
                        .collect()
                })
                .unwrap_or_default()
        })
    }

    #[test]
    fn every_change_is_readable() {
        assert_eq!(book_changes().len(), 275);
    }

    #[test]
    fn most_of_them_need_nobody() {
        let conditional = book_changes()
            .iter()
            .filter(|change| change.is_conditional())
            .count();

        assert_eq!(conditional, 65);
        assert_eq!(book_changes().len() - conditional, 210);
    }

    /// Whatever order the book puts them in, none of it breaks the hero.
    #[test]
    fn applying_every_unconditional_one_never_raises() {
        let mut state = holdings(12, 25, 25, 10, &[("ration", 4)]);

        for change in book_changes() {
            state = state.with_change(change);
        }

        assert!(state.vie_actuelle >= 0);
        assert!(state.force >= 0);
        assert!(state.gold >= 0);
        assert!(state.items.values().all(|(_, count)| *count > 0));
    }
}

/// `note: "all"` on the hero himself.
///
/// The book only ever writes it for gold, rations and equipment, but the rule is
/// the rule: whatever it names, all of it goes.
mod taking_everything_of_a_characteristic {
    use super::*;

    #[test]
    fn all_the_life_leaves_the_hero_for_dead() {
        let after = holdings(12, 25, 18, 10, &[]).with_change(
            &Change::new(LIFE, "point de Vie", None, -1).with_note("all"),
        );

        assert_eq!(after.vie_actuelle, 0);
    }

    #[test]
    fn all_the_strength_leaves_nothing() {
        let after = holdings(14, 25, 25, 10, &[]).with_change(
            &Change::new(STRENGTH, "point de Force", None, -1).with_note("all"),
        );

        assert_eq!(after.force, 0);
    }

    /// "All" is a loss's word; as a gain it names no amount to add.
    #[test]
    fn gaining_all_of_something_gains_nothing() {
        let after = holdings(12, 25, 25, 10, &[]).with_change(
            &Change::new(GOLD, "pièce d'or", None, 1).with_note("all"),
        );

        assert_eq!(after.gold, 10);
    }
}
