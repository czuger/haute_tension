//! Tests for the combat engine — every rule on its own, with fixed dice.

#[path = "common/mod.rs"]
mod common;

use common::{fixed, seeded};
use haute_tension::core::combat::{
    attack_force, combat_status, engaged_indexes, resolve_assault, Combatant, DEFEAT, ENEMY, HERO,
    ONGOING, SEQUENTIAL, SIMULTANEOUS, SINGLE, VICTORY,
};

/// A hero at full Vie unless a test says otherwise.
fn hero(force: i64, vie: i64, adjustment: i64) -> Combatant {
    Combatant::new("Prêtre Jean", force, vie, vie, adjustment)
}

/// The hero the tests that care about nothing else use.
fn plain_hero() -> Combatant {
    hero(10, 20, 0)
}

/// One adversary at full Vie.
fn enemy(name: &str, force: i64, vie: i64, adjustment: i64) -> Combatant {
    Combatant::new(name, force, vie, vie, adjustment)
}

/// The adversary the tests that care about nothing else use.
fn plain_enemy() -> Combatant {
    enemy("orc", 6, 10, 0)
}

/// The state each side carries.
mod combatant {
    use super::*;

    #[test]
    fn a_wound_comes_off_the_vie() {
        assert_eq!(enemy("orc", 6, 10, 0).wounded_by(3).vie_actuelle, 7);
    }

    #[test]
    fn vie_never_goes_below_zero() {
        assert_eq!(enemy("orc", 6, 2, 0).wounded_by(9).vie_actuelle, 0);
    }

    #[test]
    fn a_combatant_with_vie_is_alive() {
        assert!(enemy("orc", 6, 1, 0).is_alive());
        assert!(!enemy("orc", 6, 1, 0).wounded_by(1).is_alive());
    }

    #[test]
    fn being_slain_empties_the_vie_whatever_it_was() {
        assert_eq!(enemy("orc", 6, 40, 0).slain().vie_actuelle, 0);
    }

    #[test]
    fn wounding_leaves_the_original_alone() {
        let original = enemy("orc", 6, 10, 0);
        original.wounded_by(4);

        assert_eq!(original.vie_actuelle, 10);
    }
}

/// "jetez deux dés. Ajoutez au résultat votre total de Force du moment".
mod the_attack_force {
    use super::*;

    #[test]
    fn it_is_the_force_plus_two_dice() {
        let (total, roll) = attack_force(15, Some(&fixed(&[4, 5])));

        assert_eq!(roll.dice(), [4, 5]);
        assert_eq!(total, 24);
    }
}

/// Who the hero faces this assault.
mod who_is_engaged {
    use super::*;

    #[test]
    fn a_single_fight_engages_the_one_adversary() {
        assert_eq!(engaged_indexes(&[plain_enemy()], SINGLE), [0]);
    }

    #[test]
    fn a_sequential_fight_engages_one_at_a_time() {
        assert_eq!(
            engaged_indexes(&[plain_enemy(), plain_enemy()], SEQUENTIAL),
            [0]
        );
    }

    #[test]
    fn a_sequential_fight_moves_on_once_the_first_is_down() {
        let fallen = plain_enemy().slain();

        assert_eq!(engaged_indexes(&[fallen, plain_enemy()], SEQUENTIAL), [1]);
    }

    #[test]
    fn a_simultaneous_fight_engages_every_survivor() {
        assert_eq!(
            engaged_indexes(&[plain_enemy(), plain_enemy()], SIMULTANEOUS),
            [0, 1]
        );
    }

    #[test]
    fn the_dead_are_never_engaged() {
        assert_eq!(
            engaged_indexes(&[plain_enemy().slain(), plain_enemy()], SIMULTANEOUS),
            [1]
        );
    }

    #[test]
    fn nothing_is_engaged_once_all_are_down() {
        assert!(engaged_indexes(&[plain_enemy().slain()], SIMULTANEOUS).is_empty());
    }
}

/// "retirez la Force d'Attaque la plus faible de la plus élevée".
mod damage {
    use super::*;

    #[test]
    fn the_gap_is_the_damage() {
        // Hero 10 + 9 = 19, enemy 6 + 5 = 11, gap 8.
        let (_, enemies, assault) = resolve_assault(
            &hero(10, 20, 0),
            &[enemy("orc", 6, 20, 0)],
            SINGLE,
            1,
            Some(&fixed(&[4, 5, 2, 3])),
        );

        assert_eq!(assault.hero_attack_force, 19);
        assert_eq!(assault.exchanges[0].enemy_attack_force, 11);
        assert_eq!(assault.exchanges[0].winner, Some(HERO));
        assert_eq!(assault.exchanges[0].damage, 8);
        assert_eq!(enemies[0].vie_actuelle, 12);
    }

    #[test]
    fn a_gap_of_one_barely_stings() {
        let (_, enemies, assault) = resolve_assault(
            &hero(10, 20, 0),
            &[enemy("orc", 10, 20, 0)],
            SINGLE,
            1,
            Some(&fixed(&[4, 5, 4, 4])),
        );

        assert_eq!(assault.exchanges[0].damage, 1);
        assert_eq!(enemies[0].vie_actuelle, 19);
    }

    #[test]
    fn losing_the_assault_wounds_the_hero() {
        let (survivor, enemies, assault) = resolve_assault(
            &hero(6, 20, 0),
            &[enemy("orc", 10, 10, 0)],
            SINGLE,
            1,
            Some(&fixed(&[2, 3, 4, 5])),
        );

        assert_eq!(assault.exchanges[0].winner, Some(ENEMY));
        assert_eq!(assault.exchanges[0].damage, 8);
        assert_eq!(survivor.vie_actuelle, 12);
        assert_eq!(enemies[0].vie_actuelle, 10);
    }

    #[test]
    fn equal_forces_cost_nobody_anything() {
        let (survivor, enemies, assault) = resolve_assault(
            &hero(10, 20, 0),
            &[enemy("orc", 10, 10, 0)],
            SINGLE,
            1,
            Some(&fixed(&[3, 4, 3, 4])),
        );

        assert_eq!(assault.exchanges[0].winner, None);
        assert_eq!(assault.exchanges[0].damage, 0);
        assert_eq!(survivor.vie_actuelle, 20);
        assert_eq!(enemies[0].vie_actuelle, 10);
    }

    #[test]
    fn the_hero_adjustment_adds_to_his_blows() {
        let (_, _, assault) = resolve_assault(
            &hero(10, 20, 2),
            &[plain_enemy()],
            SINGLE,
            1,
            Some(&fixed(&[4, 5, 2, 3])),
        );

        assert_eq!(assault.exchanges[0].damage, 10);
    }

    #[test]
    fn the_adversary_adjustment_adds_to_its_own() {
        let (survivor, _, assault) = resolve_assault(
            &hero(6, 20, 0),
            &[enemy("orc", 10, 10, 1)],
            SINGLE,
            1,
            Some(&fixed(&[2, 3, 4, 5])),
        );

        assert_eq!(assault.exchanges[0].damage, 9);
        assert_eq!(survivor.vie_actuelle, 11);
    }

    /// The hero's bonus is for the blows he lands, not the ones he takes.
    #[test]
    fn an_adjustment_never_applies_to_a_loss() {
        let (_, _, assault) = resolve_assault(
            &hero(6, 20, 2),
            &[enemy("orc", 10, 10, 0)],
            SINGLE,
            1,
            Some(&fixed(&[2, 3, 4, 5])),
        );

        assert_eq!(assault.exchanges[0].damage, 8);
    }
}

/// "Si vous tirez un double 6, vous avez instantanément tué votre adversaire".
mod divine_judgement {
    use super::*;

    #[test]
    fn a_hero_double_six_kills_outright() {
        let (_, enemies, assault) = resolve_assault(
            &plain_hero(),
            &[enemy("orc", 6, 40, 0)],
            SINGLE,
            1,
            Some(&fixed(&[6, 6, 1, 2])),
        );

        assert!(assault.exchanges[0].divine_judgement);
        assert_eq!(assault.exchanges[0].winner, Some(HERO));
        assert_eq!(enemies[0].vie_actuelle, 0);
    }

    #[test]
    fn an_adversary_double_one_kills_the_hero() {
        let (survivor, enemies, assault) = resolve_assault(
            &hero(10, 40, 0),
            &[plain_enemy()],
            SINGLE,
            1,
            Some(&fixed(&[3, 4, 1, 1])),
        );

        assert!(assault.exchanges[0].divine_judgement);
        assert_eq!(assault.exchanges[0].winner, Some(ENEMY));
        assert_eq!(survivor.vie_actuelle, 0);
        assert_eq!(enemies[0].vie_actuelle, 10);
    }

    /// A double 6 losing on points still kills: no gap is computed.
    #[test]
    fn a_judgement_ignores_the_gap_entirely() {
        let (_, enemies, assault) = resolve_assault(
            &hero(1, 20, 0),
            &[enemy("orc", 100, 40, 0)],
            SINGLE,
            1,
            Some(&fixed(&[6, 6, 6, 6])),
        );

        assert_eq!(assault.exchanges[0].damage, 0);
        assert_eq!(enemies[0].vie_actuelle, 0);
    }

    #[test]
    fn a_judgement_ignores_the_adjustment() {
        let (_, _, assault) = resolve_assault(
            &hero(10, 20, 2),
            &[plain_enemy()],
            SINGLE,
            1,
            Some(&fixed(&[6, 6, 1, 2])),
        );

        assert_eq!(assault.exchanges[0].damage, 0);
    }

    /// Decided here, not in the book: an adversary killed outright cannot strike.
    #[test]
    fn the_hero_judgement_outranks_the_adversary_one() {
        let (survivor, enemies, assault) = resolve_assault(
            &hero(10, 20, 0),
            &[plain_enemy()],
            SINGLE,
            1,
            Some(&fixed(&[6, 6, 1, 1])),
        );

        assert_eq!(assault.exchanges[0].winner, Some(HERO));
        assert_eq!(survivor.vie_actuelle, 20);
        assert_eq!(enemies[0].vie_actuelle, 0);
    }
}

/// "lancez deux dés pour vous et deux dés pour chacun de vos adversaires".
mod melee {
    use super::*;

    /// One throw for the hero, one for each adversary — six faces in all.
    #[test]
    fn one_hero_roll_is_compared_to_each_adversary() {
        let (_, _, assault) = resolve_assault(
            &hero(10, 20, 0),
            &[enemy("a", 6, 10, 0), enemy("b", 6, 10, 0)],
            SIMULTANEOUS,
            1,
            Some(&fixed(&[4, 5, 1, 2, 2, 3])),
        );

        assert_eq!(assault.exchanges.len(), 2);
        assert_eq!(assault.hero_attack_force, 19);
        let names: Vec<&str> = assault
            .exchanges
            .iter()
            .map(|exchange| exchange.enemy_name.as_str())
            .collect();
        assert_eq!(names, ["a", "b"]);
        let forces: Vec<i64> = assault
            .exchanges
            .iter()
            .map(|exchange| exchange.enemy_attack_force)
            .collect();
        assert_eq!(forces, [9, 11]);
    }

    #[test]
    fn beating_both_hits_both() {
        let (_, enemies, assault) = resolve_assault(
            &hero(10, 20, 0),
            &[enemy("a", 6, 10, 0), enemy("b", 6, 10, 0)],
            SIMULTANEOUS,
            1,
            Some(&fixed(&[4, 5, 2, 2, 2, 3])),
        );

        let winners: Vec<Option<&str>> = assault
            .exchanges
            .iter()
            .map(|exchange| exchange.winner)
            .collect();
        assert_eq!(winners, [Some(HERO), Some(HERO)]);
        let left: Vec<i64> = enemies.iter().map(|enemy| enemy.vie_actuelle).collect();
        assert_eq!(left, [1, 2]);
    }

    #[test]
    fn beating_one_and_losing_to_the_other_cuts_both_ways() {
        let (survivor, enemies, assault) = resolve_assault(
            &hero(10, 20, 0),
            &[enemy("a", 6, 10, 0), enemy("b", 20, 10, 0)],
            SIMULTANEOUS,
            1,
            Some(&fixed(&[4, 5, 2, 2, 2, 2])),
        );

        let winners: Vec<Option<&str>> = assault
            .exchanges
            .iter()
            .map(|exchange| exchange.winner)
            .collect();
        assert_eq!(winners, [Some(HERO), Some(ENEMY)]);
        assert!(enemies[0].vie_actuelle < 10);
        assert!(survivor.vie_actuelle < 20);
    }

    #[test]
    fn losing_to_both_is_wounded_twice() {
        // Hero 1 + (1+2) = 4; each adversary 20 + (3+3) = 26, a gap of 22 twice.
        let (survivor, _, assault) = resolve_assault(
            &hero(1, 60, 0),
            &[enemy("a", 20, 10, 0), enemy("b", 20, 10, 0)],
            SIMULTANEOUS,
            1,
            Some(&fixed(&[1, 2, 3, 3, 3, 3])),
        );

        let winners: Vec<Option<&str>> = assault
            .exchanges
            .iter()
            .map(|exchange| exchange.winner)
            .collect();
        assert_eq!(winners, [Some(ENEMY), Some(ENEMY)]);
        let damage: Vec<i64> = assault
            .exchanges
            .iter()
            .map(|exchange| exchange.damage)
            .collect();
        assert_eq!(damage, [22, 22]);
        assert_eq!(survivor.vie_actuelle, 60 - 22 - 22);
    }

    /// No adversary strikes a corpse: the rest of the melee is not rolled.
    #[test]
    fn a_hero_killed_by_the_first_stops_the_assault() {
        let (survivor, _, assault) = resolve_assault(
            &hero(10, 2, 0),
            &[enemy("a", 100, 10, 0), enemy("b", 6, 10, 0)],
            SIMULTANEOUS,
            1,
            Some(&fixed(&[1, 1, 6, 6])),
        );

        assert!(!survivor.is_alive());
        assert_eq!(assault.exchanges.len(), 1);
    }

    #[test]
    fn a_sequential_fight_meets_one_adversary_at_a_time() {
        let (_, _, assault) = resolve_assault(
            &plain_hero(),
            &[enemy("a", 6, 10, 0), enemy("b", 6, 10, 0)],
            SEQUENTIAL,
            1,
            Some(&fixed(&[4, 5, 1, 1])),
        );

        let names: Vec<&str> = assault
            .exchanges
            .iter()
            .map(|exchange| exchange.enemy_name.as_str())
            .collect();
        assert_eq!(names, ["a"]);
    }
}

/// When the fight is over, and for whom.
mod status {
    use super::*;

    #[test]
    fn a_fight_with_both_standing_goes_on() {
        assert_eq!(combat_status(&plain_hero(), &[plain_enemy()]), ONGOING);
    }

    #[test]
    fn every_adversary_down_is_a_victory() {
        assert_eq!(
            combat_status(&plain_hero(), &[plain_enemy().slain()]),
            VICTORY
        );
    }

    #[test]
    fn one_adversary_left_standing_is_not() {
        assert_eq!(
            combat_status(&plain_hero(), &[plain_enemy().slain(), plain_enemy()]),
            ONGOING
        );
    }

    #[test]
    fn a_fallen_hero_is_a_defeat() {
        assert_eq!(
            combat_status(&plain_hero().slain(), &[plain_enemy()]),
            DEFEAT
        );
    }

    /// A double 1 that kills the last adversary's killer is still a defeat.
    #[test]
    fn a_defeat_outranks_a_victory() {
        assert_eq!(
            combat_status(&plain_hero().slain(), &[plain_enemy().slain()]),
            DEFEAT
        );
    }
}

/// The loop the routes drive, run to its end.
mod the_whole_fight {
    use super::*;

    #[test]
    fn a_fight_always_ends() {
        for seed in 0..60 {
            let rng = seeded(seed);
            let mut fighter = hero(12, 26, 0);
            let mut enemies = vec![enemy("orc", 8, 12, 0)];
            for number in 1..200 {
                if combat_status(&fighter, &enemies) != ONGOING {
                    break;
                }
                let played =
                    resolve_assault(&fighter, &enemies, SINGLE, number, Some(&rng));
                fighter = played.0;
                enemies = played.1;
            }

            assert_ne!(combat_status(&fighter, &enemies), ONGOING);
        }
    }

    #[test]
    fn a_sequential_fight_works_through_the_queue() {
        let rng = seeded(3);
        let mut fighter = hero(18, 30, 0);
        let mut enemies = vec![
            enemy("a", 4, 6, 0),
            enemy("b", 4, 6, 0),
            enemy("c", 4, 6, 0),
        ];
        for number in 1..100 {
            if combat_status(&fighter, &enemies) != ONGOING {
                break;
            }
            let played =
                resolve_assault(&fighter, &enemies, SEQUENTIAL, number, Some(&rng));
            fighter = played.0;
            enemies = played.1;
        }

        assert_eq!(combat_status(&fighter, &enemies), VICTORY);
        assert!(enemies.iter().all(|enemy| !enemy.is_alive()));
    }
}
