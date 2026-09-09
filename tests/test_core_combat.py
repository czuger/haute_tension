"""Tests for the combat engine — every rule on its own, with fixed dice."""

import random

from haute_tension.core.combat import (
    DEFEAT,
    ENEMY,
    HERO,
    ONGOING,
    SEQUENTIAL,
    SIMULTANEOUS,
    SINGLE,
    VICTORY,
    Combatant,
    attack_force,
    combat_status,
    engaged_indexes,
    resolve_assault,
)
from tests.test_core_dice import FixedDice


def hero(force=10, vie=20, adjustment=0):
    """A hero at full Vie unless a test says otherwise."""
    return Combatant("Prêtre Jean", force, vie, vie, adjustment)


def enemy(name="orc", force=6, vie=10, adjustment=0):
    """One adversary at full Vie."""
    return Combatant(name, force, vie, vie, adjustment)


class TestCombatant:
    """The state each side carries."""

    def test_a_wound_comes_off_the_vie(self):
        assert enemy(vie=10).wounded_by(3).vie_actuelle == 7

    def test_vie_never_goes_below_zero(self):
        assert enemy(vie=2).wounded_by(9).vie_actuelle == 0

    def test_a_combatant_with_vie_is_alive(self):
        assert enemy(vie=1).is_alive
        assert not enemy(vie=1).wounded_by(1).is_alive

    def test_being_slain_empties_the_vie_whatever_it_was(self):
        assert enemy(vie=40).slain().vie_actuelle == 0

    def test_wounding_leaves_the_original_alone(self):
        original = enemy(vie=10)
        original.wounded_by(4)

        assert original.vie_actuelle == 10


class TestAttackForce:
    """"jetez deux dés. Ajoutez au résultat votre total de Force du moment"."""

    def test_it_is_the_force_plus_two_dice(self):
        total, roll = attack_force(15, FixedDice(4, 5))

        assert roll.dice == (4, 5)
        assert total == 24


class TestEngagedIndexes:
    """Who the hero faces this assault."""

    def test_a_single_fight_engages_the_one_adversary(self):
        assert engaged_indexes([enemy()], SINGLE) == [0]

    def test_a_sequential_fight_engages_one_at_a_time(self):
        assert engaged_indexes([enemy(), enemy()], SEQUENTIAL) == [0]

    def test_a_sequential_fight_moves_on_once_the_first_is_down(self):
        fallen = enemy().slain()

        assert engaged_indexes([fallen, enemy()], SEQUENTIAL) == [1]

    def test_a_simultaneous_fight_engages_every_survivor(self):
        assert engaged_indexes([enemy(), enemy()], SIMULTANEOUS) == [0, 1]

    def test_the_dead_are_never_engaged(self):
        assert engaged_indexes([enemy().slain(), enemy()], SIMULTANEOUS) == [1]

    def test_nothing_is_engaged_once_all_are_down(self):
        assert engaged_indexes([enemy().slain()], SIMULTANEOUS) == []


class TestDamage:
    """"retirez la Force d'Attaque la plus faible de la plus élevée"."""

    def test_the_gap_is_the_damage(self):
        # Hero 10 + 9 = 19, enemy 6 + 5 = 11, gap 8.
        survivor, enemies, assault = resolve_assault(
            hero(force=10), [enemy(force=6, vie=20)], SINGLE,
            rng=FixedDice(4, 5, 2, 3),
        )

        assert assault.hero_attack_force == 19
        assert assault.exchanges[0].enemy_attack_force == 11
        assert assault.exchanges[0].winner == HERO
        assert assault.exchanges[0].damage == 8
        assert enemies[0].vie_actuelle == 12

    def test_a_gap_of_one_barely_stings(self):
        _, enemies, assault = resolve_assault(
            hero(force=10), [enemy(force=10, vie=20)], SINGLE,
            rng=FixedDice(4, 5, 4, 4),
        )

        assert assault.exchanges[0].damage == 1
        assert enemies[0].vie_actuelle == 19

    def test_losing_the_assault_wounds_the_hero(self):
        survivor, enemies, assault = resolve_assault(
            hero(force=6, vie=20), [enemy(force=10)], SINGLE,
            rng=FixedDice(2, 3, 4, 5),
        )

        assert assault.exchanges[0].winner == ENEMY
        assert assault.exchanges[0].damage == 8
        assert survivor.vie_actuelle == 12
        assert enemies[0].vie_actuelle == 10

    def test_equal_forces_cost_nobody_anything(self):
        survivor, enemies, assault = resolve_assault(
            hero(force=10), [enemy(force=10)], SINGLE,
            rng=FixedDice(3, 4, 3, 4),
        )

        assert assault.exchanges[0].winner is None
        assert assault.exchanges[0].damage == 0
        assert survivor.vie_actuelle == 20
        assert enemies[0].vie_actuelle == 10

    def test_the_hero_adjustment_adds_to_his_blows(self):
        _, _, assault = resolve_assault(
            hero(force=10, adjustment=2), [enemy(force=6)], SINGLE,
            rng=FixedDice(4, 5, 2, 3),
        )

        assert assault.exchanges[0].damage == 10

    def test_the_adversary_adjustment_adds_to_its_own(self):
        survivor, _, assault = resolve_assault(
            hero(force=6, vie=20), [enemy(force=10, adjustment=1)], SINGLE,
            rng=FixedDice(2, 3, 4, 5),
        )

        assert assault.exchanges[0].damage == 9
        assert survivor.vie_actuelle == 11

    def test_an_adjustment_never_applies_to_a_loss(self):
        """The hero's bonus is for the blows he lands, not the ones he takes."""
        survivor, _, assault = resolve_assault(
            hero(force=6, vie=20, adjustment=2), [enemy(force=10)], SINGLE,
            rng=FixedDice(2, 3, 4, 5),
        )

        assert assault.exchanges[0].damage == 8


class TestDivineJudgement:
    """"Si vous tirez un double 6, vous avez instantanément tué votre adversaire"."""

    def test_a_hero_double_six_kills_outright(self):
        _, enemies, assault = resolve_assault(
            hero(), [enemy(vie=40)], SINGLE, rng=FixedDice(6, 6, 1, 2)
        )

        assert assault.exchanges[0].divine_judgement
        assert assault.exchanges[0].winner == HERO
        assert enemies[0].vie_actuelle == 0

    def test_an_adversary_double_one_kills_the_hero(self):
        survivor, enemies, assault = resolve_assault(
            hero(vie=40), [enemy()], SINGLE, rng=FixedDice(3, 4, 1, 1)
        )

        assert assault.exchanges[0].divine_judgement
        assert assault.exchanges[0].winner == ENEMY
        assert survivor.vie_actuelle == 0
        assert enemies[0].vie_actuelle == 10

    def test_a_judgement_ignores_the_gap_entirely(self):
        """A double 6 losing on points still kills: no gap is computed."""
        _, enemies, assault = resolve_assault(
            hero(force=1), [enemy(force=100, vie=40)], SINGLE,
            rng=FixedDice(6, 6, 6, 6),
        )

        assert assault.exchanges[0].damage == 0
        assert enemies[0].vie_actuelle == 0

    def test_a_judgement_ignores_the_adjustment(self):
        _, _, assault = resolve_assault(
            hero(adjustment=2), [enemy()], SINGLE, rng=FixedDice(6, 6, 1, 2)
        )

        assert assault.exchanges[0].damage == 0

    def test_the_hero_judgement_outranks_the_adversary_one(self):
        """Decided here, not in the book: an adversary killed outright cannot strike."""
        survivor, enemies, assault = resolve_assault(
            hero(vie=20), [enemy()], SINGLE, rng=FixedDice(6, 6, 1, 1)
        )

        assert assault.exchanges[0].winner == HERO
        assert survivor.vie_actuelle == 20
        assert enemies[0].vie_actuelle == 0


class TestMelee:
    """"lancez deux dés pour vous et deux dés pour chacun de vos adversaires"."""

    def test_one_hero_roll_is_compared_to_each_adversary(self):
        """One throw for the hero, one for each adversary — six faces in all."""
        _, _, assault = resolve_assault(
            hero(force=10), [enemy("a", force=6), enemy("b", force=6)],
            SIMULTANEOUS, rng=FixedDice(4, 5, 1, 2, 2, 3),
        )

        assert len(assault.exchanges) == 2
        assert assault.hero_attack_force == 19
        assert [e.enemy_name for e in assault.exchanges] == ["a", "b"]
        assert [e.enemy_attack_force for e in assault.exchanges] == [9, 11]

    def test_beating_both_hits_both(self):
        _, enemies, assault = resolve_assault(
            hero(force=10), [enemy("a", force=6), enemy("b", force=6)],
            SIMULTANEOUS, rng=FixedDice(4, 5, 2, 2, 2, 3),
        )

        assert [e.winner for e in assault.exchanges] == [HERO, HERO]
        assert [e.vie_actuelle for e in enemies] == [1, 2]

    def test_beating_one_and_losing_to_the_other_cuts_both_ways(self):
        survivor, enemies, assault = resolve_assault(
            hero(force=10, vie=20), [enemy("a", force=6), enemy("b", force=20)],
            SIMULTANEOUS, rng=FixedDice(4, 5, 2, 2, 2, 2),
        )

        assert [e.winner for e in assault.exchanges] == [HERO, ENEMY]
        assert enemies[0].vie_actuelle < 10
        assert survivor.vie_actuelle < 20

    def test_losing_to_both_is_wounded_twice(self):
        # Hero 1 + (1+2) = 4; each adversary 20 + (3+3) = 26, a gap of 22 twice.
        survivor, _, assault = resolve_assault(
            hero(force=1, vie=60), [enemy("a", force=20), enemy("b", force=20)],
            SIMULTANEOUS, rng=FixedDice(1, 2, 3, 3, 3, 3),
        )

        assert [e.winner for e in assault.exchanges] == [ENEMY, ENEMY]
        assert [e.damage for e in assault.exchanges] == [22, 22]
        assert survivor.vie_actuelle == 60 - 22 - 22

    def test_a_hero_killed_by_the_first_stops_the_assault(self):
        """No adversary strikes a corpse: the rest of the melee is not rolled."""
        survivor, _, assault = resolve_assault(
            hero(vie=2), [enemy("a", force=100), enemy("b")], SIMULTANEOUS,
            rng=FixedDice(1, 1, 6, 6),
        )

        assert not survivor.is_alive
        assert len(assault.exchanges) == 1

    def test_a_sequential_fight_meets_one_adversary_at_a_time(self):
        _, _, assault = resolve_assault(
            hero(), [enemy("a"), enemy("b")], SEQUENTIAL,
            rng=FixedDice(4, 5, 1, 1),
        )

        assert [e.enemy_name for e in assault.exchanges] == ["a"]


class TestCombatStatus:
    """When the fight is over, and for whom."""

    def test_a_fight_with_both_standing_goes_on(self):
        assert combat_status(hero(), [enemy()]) == ONGOING

    def test_every_adversary_down_is_a_victory(self):
        assert combat_status(hero(), [enemy().slain()]) == VICTORY

    def test_one_adversary_left_standing_is_not(self):
        assert combat_status(hero(), [enemy().slain(), enemy()]) == ONGOING

    def test_a_fallen_hero_is_a_defeat(self):
        assert combat_status(hero().slain(), [enemy()]) == DEFEAT

    def test_a_defeat_outranks_a_victory(self):
        """A double 1 that kills the last adversary's killer is still a defeat."""
        assert combat_status(hero().slain(), [enemy().slain()]) == DEFEAT


class TestWholeFight:
    """The loop the routes drive, run to its end."""

    def test_a_fight_always_ends(self):
        for seed in range(60):
            rng = random.Random(seed)
            fighter, enemies = hero(force=12, vie=26), [enemy(force=8, vie=12)]
            for number in range(1, 200):
                if combat_status(fighter, enemies) != ONGOING:
                    break
                fighter, enemies, _ = resolve_assault(
                    fighter, enemies, SINGLE, number, rng
                )

            assert combat_status(fighter, enemies) != ONGOING

    def test_a_sequential_fight_works_through_the_queue(self):
        rng = random.Random(3)
        fighter = hero(force=18, vie=30)
        enemies = [enemy("a", 4, 6), enemy("b", 4, 6), enemy("c", 4, 6)]
        for number in range(1, 100):
            if combat_status(fighter, enemies) != ONGOING:
                break
            fighter, enemies, _ = resolve_assault(
                fighter, enemies, SEQUENTIAL, number, rng
            )

        assert combat_status(fighter, enemies) == VICTORY
        assert all(not e.is_alive for e in enemies)
