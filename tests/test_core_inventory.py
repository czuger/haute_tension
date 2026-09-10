"""What a gain or a loss does to a hero."""

import random

import pytest

from haute_tension.core.inventory import (
    GOLD,
    LIFE,
    STRENGTH,
    Change,
    Holdings,
    pluralised,
    read_changes,
    starting_gold,
)
from tests.test_core_dice import FixedDice


def holdings(force=12, vie_max=25, vie_actuelle=25, gold=10, **items):
    """A hero carrying whatever the test cares about."""
    return Holdings(
        force=force,
        vie_max=vie_max,
        vie_actuelle=vie_actuelle,
        gold=gold,
        items={name: (name, count) for name, count in items.items()},
    )


def gain(element, amount=1, **extra):
    return Change(element, element, amount, sign=1, **extra)


def loss(element, amount=1, **extra):
    return Change(element, element, amount, sign=-1, **extra)


class TestReadingChanges:
    """Every gain and loss a choice carries."""

    def test_gains_and_losses_are_both_read(self):
        changes = read_changes(
            {
                "gains": [{"element": "key", "label_fr": "clé", "amount": 1}],
                "losses": [{"element": GOLD, "label_fr": "pièce d'or", "amount": 3}],
            }
        )

        assert [c.element for c in changes] == ["key", GOLD]
        assert [c.sign for c in changes] == [1, -1]

    def test_a_choice_carrying_nothing_gives_nothing(self):
        assert read_changes({"goto": "2"}) == []
        assert read_changes({"gains": None, "losses": []}) == []

    def test_the_french_label_is_kept(self):
        [change] = read_changes(
            {"gains": [{"element": "key", "label_fr": "clé", "amount": 1}]}
        )

        assert change.label == "clé"

    def test_a_missing_label_falls_back_to_the_element(self):
        [change] = read_changes({"gains": [{"element": "key", "amount": 1}]})

        assert change.label == "key"

    def test_a_malformed_entry_is_dropped(self):
        assert read_changes({"gains": ["clé"]}) == []

    def test_a_condition_is_kept(self):
        [change] = read_changes(
            {
                "losses": [
                    {
                        "element": "ration",
                        "label_fr": "ration",
                        "amount": 1,
                        "condition": "Si vous avez des provisions",
                    }
                ]
            }
        )

        assert change.condition == "Si vous avez des provisions"
        assert change.is_conditional

    def test_a_blank_condition_is_no_condition(self):
        [change] = read_changes(
            {"gains": [{"element": "key", "amount": 1, "condition": "  "}]}
        )

        assert change.condition is None
        assert not change.is_conditional

    def test_a_rolled_amount_is_conditional_too(self):
        """`note: "dice"` means the page's text says how many dice."""
        [change] = read_changes(
            {"losses": [{"element": LIFE, "amount": None, "note": "dice"}]}
        )

        assert change.amount is None
        assert change.is_conditional

    def test_taking_everything_is_not_conditional(self):
        """`note: "all"` is the one note the code can act on."""
        [change] = read_changes(
            {"losses": [{"element": GOLD, "amount": None, "note": "all"}]}
        )

        assert change.takes_everything
        assert not change.is_conditional


class TestCharacteristics:
    """The three elements that are the hero rather than his bag."""

    def test_a_life_gain_heals(self):
        after = holdings(vie_actuelle=10).with_change(gain(LIFE, 4))

        assert after.vie_actuelle == 14

    def test_a_life_gain_never_passes_the_starting_total(self):
        """"votre total de Vie de départ est immuable […] Vous ne pourrez le
        dépasser en aucun cas"."""
        after = holdings(vie_max=25, vie_actuelle=22).with_change(gain(LIFE, 40))

        assert after.vie_actuelle == 25

    def test_a_life_loss_wounds(self):
        assert holdings(vie_actuelle=20).with_change(loss(LIFE, 6)).vie_actuelle == 14

    def test_life_never_goes_below_zero(self):
        assert holdings(vie_actuelle=2).with_change(loss(LIFE, 9)).vie_actuelle == 0

    def test_the_starting_total_never_moves(self):
        after = holdings(vie_max=25, vie_actuelle=25).with_change(loss(LIFE, 5))

        assert after.vie_max == 25

    def test_strength_moves_both_ways(self):
        assert holdings(force=12).with_change(gain(STRENGTH, 2)).force == 14
        assert holdings(force=12).with_change(loss(STRENGTH, 3)).force == 9

    def test_strength_never_goes_below_zero(self):
        assert holdings(force=2).with_change(loss(STRENGTH, 9)).force == 0

    def test_gold_moves_both_ways(self):
        assert holdings(gold=10).with_change(gain(GOLD, 5)).gold == 15
        assert holdings(gold=10).with_change(loss(GOLD, 4)).gold == 6

    def test_gold_never_goes_below_zero(self):
        """A hero cannot pay what he has not got."""
        assert holdings(gold=3).with_change(loss(GOLD, 9)).gold == 0

    def test_losing_all_the_gold_empties_the_purse(self):
        after = holdings(gold=17).with_change(
            Change(GOLD, "pièce d'or", None, note="all", sign=-1)
        )

        assert after.gold == 0

    def test_a_characteristic_never_reaches_the_bag(self):
        assert holdings().with_change(gain(LIFE, 2)).items == {}


class TestTheBag:
    """Everything that is a thing."""

    def test_a_gain_puts_something_in(self):
        after = holdings().with_change(gain("key", 1))

        assert after.items == {"key": ("key", 1)}

    def test_a_second_one_stacks(self):
        after = holdings(key=1).with_change(gain("key", 2))

        assert after.items["key"] == ("key", 3)

    def test_a_loss_takes_some_out(self):
        after = holdings(ration=4).with_change(loss("ration", 1))

        assert after.items["ration"] == ("ration", 3)

    def test_the_last_one_leaves_the_bag(self):
        after = holdings(ration=1).with_change(loss("ration", 1))

        assert "ration" not in after.items

    def test_losing_more_than_is_held_leaves_the_bag(self):
        after = holdings(ration=2).with_change(loss("ration", 9))

        assert "ration" not in after.items

    def test_losing_what_is_not_held_changes_nothing(self):
        after = holdings(ration=4).with_change(loss("key", 1))

        assert after.items == {"ration": ("ration", 4)}

    def test_losing_all_of_something_empties_it(self):
        after = holdings(ration=4).with_change(
            Change("ration", "ration", None, note="all", sign=-1)
        )

        assert "ration" not in after.items

    def test_the_first_label_is_the_one_kept(self):
        after = holdings().with_change(Change("key", "clé", 1))

        assert after.items["key"] == ("clé", 1)

    def test_the_original_holdings_are_left_alone(self):
        before = holdings(ration=4)
        before.with_change(loss("ration", 4))

        assert before.items["ration"] == ("ration", 4)


class TestStartingGold:
    """"lancez deux dés deux fois de suite"."""

    def test_it_is_two_throws_of_two_dice(self):
        total, throws = starting_gold(FixedDice(3, 4, 5, 2))

        assert [throw.dice for throw in throws] == [(3, 4), (5, 2)]
        assert total == 14

    def test_it_never_leaves_four_to_twenty_four(self):
        totals = [starting_gold(random.Random(s))[0] for s in range(400)]

        assert min(totals) >= 4
        assert max(totals) <= 24

    def test_the_floor_is_four_dice_at_one(self):
        """Seeding for it would be a lottery: a total of 4 is one run in 1296."""
        assert starting_gold(FixedDice(1, 1, 1, 1))[0] == 4

    def test_the_ceiling_is_four_dice_at_six(self):
        assert starting_gold(FixedDice(6, 6, 6, 6))[0] == 24


class TestDescribing:
    """A change as a reader reads it."""

    def test_a_gain_is_written_with_a_plus(self):
        assert Change("key", "clé", 2).described() == "+2 clés"

    def test_a_loss_is_written_with_a_minus(self):
        assert Change("key", "clé", 1, sign=-1).described() == "-1 clé"

    def test_taking_everything_says_so(self):
        written = Change(GOLD, "pièce d'or", None, note="all", sign=-1).described()

        assert "tout" in written

    def test_a_rolled_amount_says_it_is_unknown(self):
        assert Change(LIFE, "point de Vie", None, note="dice", sign=-1).described() == (
            "-? point de Vie"
        )

    @pytest.mark.parametrize(
        "label,count,expected",
        [
            ("point de Vie", 2, "points de Vie"),
            ("pièce d'or", 5, "pièces d'or"),
            ("ration", 3, "rations"),
            ("ration", 1, "ration"),
            ("gantelets magiques", 2, "gantelets magiques"),
            ("", 3, ""),
        ],
    )
    def test_french_marks_the_plural_on_the_head_noun(self, label, count, expected):
        assert pluralised(label, count) == expected


class TestTheWholeBook:
    """Every change the book actually carries goes through this."""

    def test_every_change_is_readable(self, book_changes):
        assert len(book_changes) == 275

    def test_most_of_them_need_nobody(self, book_changes):
        conditional = [c for c in book_changes if c.is_conditional]

        assert len(conditional) == 65
        assert len(book_changes) - len(conditional) == 210

    def test_applying_every_unconditional_one_never_raises(self, book_changes):
        """Whatever order the book puts them in, none of it breaks the hero."""
        state = holdings(force=12, vie_max=25, vie_actuelle=25, gold=10, ration=4)

        for change in book_changes:
            state = state.with_change(change)

        assert state.vie_actuelle >= 0
        assert state.force >= 0
        assert state.gold >= 0
        assert all(count > 0 for _, count in state.items.values())


@pytest.fixture(scope="module")
def book_changes():
    """Every gain and loss of the packaged book."""
    import json

    from haute_tension.application.factory import BOOK, BOOKS_PATH
    from haute_tension.core.story import PAGES_FILE

    pages = json.loads(
        (BOOKS_PATH / BOOK / PAGES_FILE).read_text(encoding="utf-8")
    )
    return [
        change
        for page in pages
        for choice in page.get("choices") or []
        for change in read_changes(choice)
    ]


class TestTakingEverythingOfACharacteristic:
    """`note: "all"` on the hero himself.

    The book only ever writes it for gold, rations and equipment, but the rule
    is the rule: whatever it names, all of it goes.
    """

    def test_all_the_life_leaves_the_hero_for_dead(self):
        after = holdings(vie_actuelle=18).with_change(
            Change(LIFE, "point de Vie", None, note="all", sign=-1)
        )

        assert after.vie_actuelle == 0

    def test_all_the_strength_leaves_nothing(self):
        after = holdings(force=14).with_change(
            Change(STRENGTH, "point de Force", None, note="all", sign=-1)
        )

        assert after.force == 0

    def test_gaining_all_of_something_gains_nothing(self):
        """"All" is a loss's word; as a gain it names no amount to add."""
        after = holdings(gold=10).with_change(
            Change(GOLD, "pièce d'or", None, note="all", sign=1)
        )

        assert after.gold == 10
