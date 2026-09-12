//! Tests for rolling up Prêtre Jean, by the book and the easy way.

#[path = "common/mod.rs"]
mod common;

use common::{fixed, seeded};
use haute_tension::core::character::{
    damage_adjustment, generate_character, named_mode, Mode, Throw, DEFAULT_MODE, EASY, MODES,
    NORMAL,
};

/// The two ways of rolling a hero up, for the tests that check both.
const BOTH: [Mode; 2] = [NORMAL, EASY];

/// A characteristic: a base, and the dice added to it.
mod throw {
    use super::*;

    #[test]
    fn it_writes_its_dice_the_way_a_reader_does() {
        assert_eq!(Throw::new(6, 2, 6).notation(), "2D6");
        assert_eq!(Throw::new(26, 3, 4).notation(), "3D4");
    }

    #[test]
    fn its_floor_is_every_die_at_one() {
        assert_eq!(Throw::new(6, 2, 6).lowest(), 8);
        assert_eq!(Throw::new(26, 3, 4).lowest(), 29);
    }

    #[test]
    fn its_ceiling_is_every_die_at_its_top_face() {
        assert_eq!(Throw::new(6, 2, 6).highest(), 18);
        assert_eq!(Throw::new(26, 3, 4).highest(), 38);
    }

    #[test]
    fn rolling_adds_the_base_to_the_dice() {
        let (total, roll) = Throw::new(12, 2, 4).roll(Some(&fixed(&[3, 4])));

        assert_eq!(roll.dice(), [3, 4]);
        assert_eq!(total, 19);
    }

    #[test]
    fn it_throws_as_many_dice_as_it_says() {
        let (_, roll) = Throw::new(26, 3, 4).roll(Some(&seeded(1)));

        assert_eq!(roll.dice().len(), 3);
    }
}

/// The two ways of rolling a hero up.
mod the_modes {
    use super::*;

    #[test]
    fn the_book_is_the_default() {
        assert_eq!(DEFAULT_MODE, NORMAL);
    }

    #[test]
    fn the_normal_mode_is_the_rules_as_quoted() {
        assert_eq!((NORMAL.force.base, NORMAL.force.notation()), (6, "2D6".to_string()));
        assert_eq!((NORMAL.vie.base, NORMAL.vie.notation()), (18, "2D6".to_string()));
    }

    #[test]
    fn the_easy_mode_is_the_house_rule() {
        assert_eq!((EASY.force.base, EASY.force.notation()), (12, "2D4".to_string()));
        assert_eq!((EASY.vie.base, EASY.vie.notation()), (26, "3D4".to_string()));
    }

    #[test]
    fn both_are_offered() {
        let names: Vec<&str> = MODES.iter().map(|mode| mode.name).collect();

        assert_eq!(names, ["normal", "easy"]);
    }

    /// Its floor is above the book's average, which is the whole point.
    #[test]
    fn the_easy_mode_cannot_roll_a_weakling() {
        let average_force = (NORMAL.force.lowest() + NORMAL.force.highest()) as f64 / 2.0;
        let average_vie = (NORMAL.vie.lowest() + NORMAL.vie.highest()) as f64 / 2.0;

        assert!(EASY.force.lowest() as f64 > average_force);
        assert!(EASY.vie.lowest() as f64 > average_vie);
    }
}

/// Reading a mode off a form.
mod reading_a_named_mode {
    use super::*;

    #[test]
    fn a_known_name_is_found() {
        for name in ["normal", "easy"] {
            assert_eq!(named_mode(Some(name)).name, name);
        }
    }

    #[test]
    fn case_and_padding_are_ignored() {
        assert_eq!(named_mode(Some("  EASY ")), EASY);
    }

    /// A posted value is whatever was posted; it must not refuse to roll.
    #[test]
    fn an_unknown_name_rolls_by_the_book() {
        assert_eq!(named_mode(Some("impossible")), NORMAL);
    }

    #[test]
    fn nothing_at_all_rolls_by_the_book() {
        assert_eq!(named_mode(None), NORMAL);
        assert_eq!(named_mode(Some("")), NORMAL);
    }
}

/// The throws each mode asks for.
mod generation {
    use super::*;

    #[test]
    fn the_book_hero_is_six_plus_two_dice() {
        let character = generate_character(NORMAL, Some(&fixed(&[3, 4, 5, 2])));

        assert_eq!(character.force_roll.dice(), [3, 4]);
        assert_eq!(character.force, 13);
        assert_eq!(character.vie_max, 25);
    }

    #[test]
    fn the_easy_hero_is_twelve_plus_two_dice() {
        let character = generate_character(EASY, Some(&fixed(&[3, 4, 2, 3, 1])));

        assert_eq!(character.force_roll.dice(), [3, 4]);
        assert_eq!(character.force, 19);
    }

    #[test]
    fn the_easy_hero_rolls_three_dice_for_vie() {
        let character = generate_character(EASY, Some(&fixed(&[1, 1, 2, 3, 4])));

        assert_eq!(character.vie_roll.dice(), [2, 3, 4]);
        assert_eq!(character.vie_max, 35);
    }

    #[test]
    fn a_mode_never_leaves_its_range() {
        for mode in BOTH {
            let heroes: Vec<_> = (0..400)
                .map(|seed| generate_character(mode, Some(&seeded(seed))))
                .collect();

            assert!(heroes.iter().all(|hero| hero.force >= mode.force.lowest()));
            assert!(heroes.iter().all(|hero| hero.force <= mode.force.highest()));
            assert!(heroes.iter().all(|hero| hero.vie_max >= mode.vie.lowest()));
            assert!(heroes.iter().all(|hero| hero.vie_max <= mode.vie.highest()));
        }
    }

    #[test]
    fn the_whole_range_is_reachable() {
        for mode in BOTH {
            let forces: Vec<i64> = (0..600)
                .map(|seed| generate_character(mode, Some(&seeded(seed))).force)
                .collect();

            assert_eq!(forces.iter().min().copied(), Some(mode.force.lowest()));
            assert_eq!(forces.iter().max().copied(), Some(mode.force.highest()));
        }
    }

    #[test]
    fn he_starts_at_full_vie() {
        for mode in BOTH {
            let character = generate_character(mode, Some(&seeded(1)));

            assert_eq!(character.vie_actuelle, character.vie_max);
        }
    }

    #[test]
    fn he_remembers_how_he_was_rolled() {
        for mode in BOTH {
            assert_eq!(generate_character(mode, Some(&seeded(1))).mode, mode);
        }
    }

    #[test]
    fn the_book_is_used_when_no_mode_is_named() {
        assert_eq!(
            generate_character(DEFAULT_MODE, Some(&seeded(1))).mode,
            NORMAL
        );
    }

    #[test]
    fn force_and_vie_use_different_throws() {
        let character = generate_character(NORMAL, Some(&fixed(&[1, 1, 6, 6])));

        assert_eq!(character.force, 8);
        assert_eq!(character.vie_max, 30);
    }

    #[test]
    fn a_seed_makes_the_hero_reproducible() {
        for mode in BOTH {
            assert_eq!(
                generate_character(mode, Some(&seeded(9))),
                generate_character(mode, Some(&seeded(9)))
            );
        }
    }
}

/// What a high Force adds to the damage its blows do.
mod the_damage_adjustment {
    use super::*;

    #[test]
    fn an_ordinary_force_earns_nothing() {
        assert_eq!(damage_adjustment(10), 0);
        assert_eq!(damage_adjustment(16), 0);
    }

    #[test]
    fn seventeen_adds_one() {
        assert_eq!(damage_adjustment(17), 1);
    }

    #[test]
    fn eighteen_adds_two() {
        assert_eq!(damage_adjustment(18), 2);
    }

    /// The easy mode reaches 20; rolling higher must not be worse than 18.
    #[test]
    fn beyond_the_table_keeps_the_top_step() {
        for force in [19, 20] {
            assert_eq!(damage_adjustment(force), damage_adjustment(18));
            assert_eq!(damage_adjustment(force), 2);
        }
    }

    #[test]
    fn the_hero_reads_it_off_his_force() {
        assert_eq!(
            generate_character(NORMAL, Some(&fixed(&[6, 6, 1, 1]))).damage_adjustment(),
            2
        );
        assert_eq!(
            generate_character(NORMAL, Some(&fixed(&[3, 4, 1, 1]))).damage_adjustment(),
            0
        );
    }
}
