"""The combat engine: one assault at a time, and nothing else.

Pure: no database, no Flask, no clock. It is handed the state of a fight and
gives back the state after one assault, so a seeded `random.Random` makes a whole
combat reproducible and every rule below is testable on its own.

The rules, from `regles-du-jeu-spj1`:

- Force d'Attaque — "jetez deux dés. Ajoutez au résultat obtenu votre total de
  Force du moment."
- Damage — "retirez la Force d'Attaque la plus faible de la Force d'Attaque la
  plus élevée. Le chiffre obtenu constitue le nombre de points de Vie de dommages
  infligé." Damage is the *gap*, not a fixed toll, so an assault won by one point
  barely stings and one won by ten is close to lethal.
- Divine judgement — "Si vous tirez un double 6, vous avez instantanément tué
  votre adversaire. Si, en revanche, votre adversaire tire un double 1, c'est lui
  qui a mis fin à vos jours."

Two things the rules leave open, decided here and not read off the book:

- **Equal Forces d'Attaque cost nobody anything.** The rules only ever describe
  damage as the gap between the two, and the gap is zero.
- **A hero's double 6 outranks an adversary's double 1 in the same assault.** An
  adversary killed outright does not get to land its own blow. The book never
  puts the two together, so this is a choice; it is at least the one that reads
  the way the assault is written down, hero first.

`FIGHTS_WITH_SPECIAL_RULES` lists the pages this engine knowingly does not
model; `TODO.md` says what each of them adds.
"""

import random
from dataclasses import dataclass, field, replace

from haute_tension.core.dice import Roll, roll_2d6

SINGLE = "single"
SEQUENTIAL = "sequential"
SIMULTANEOUS = "simultaneous"

ONGOING = "ongoing"
VICTORY = "victory"
DEFEAT = "defeat"

# Pages whose text adds a rule this engine does not implement: a fight that ends
# on the first assault instead of on a death (133), a limited number of assaults
# and a Force halved under water (335, 396, 429, 595), a branch on the first
# wound taken (74) or on dropping below a Vie threshold (425, 627), a Force that
# changes mid-fight on every 6 rolled (335, 396, 595), or a flight allowed only
# once one adversary is down (609).
#
# They resolve here as plain fights, which is wrong for them and right for the
# other 34. Handle them before trusting a result on these pages.
FIGHTS_WITH_SPECIAL_RULES = frozenset(
    {"40", "74", "133", "241", "324", "335", "396", "425", "429", "540", "595",
     "609", "627"}
)


@dataclass(frozen=True)
class Combatant:
    """One side of a fight, hero or adversary."""

    name: str
    force: int
    vie_max: int
    vie_actuelle: int
    damage_adjustment: int = 0

    @property
    def is_alive(self) -> bool:
        """Whether this combatant still has Vie left."""
        return self.vie_actuelle > 0

    def wounded_by(self, damage: int) -> "Combatant":
        """Return this combatant with `damage` taken off, never below zero."""
        return replace(self, vie_actuelle=max(0, self.vie_actuelle - damage))

    def slain(self) -> "Combatant":
        """Return this combatant killed outright, whatever its Vie was."""
        return replace(self, vie_actuelle=0)


@dataclass(frozen=True)
class Exchange:
    """What happened between the hero and one adversary in one assault."""

    enemy_name: str
    enemy_force: int
    enemy_roll: Roll
    enemy_attack_force: int
    winner: str | None
    damage: int
    divine_judgement: bool = False


@dataclass(frozen=True)
class Assault:
    """One full assault: the hero's roll, and every exchange it settled."""

    number: int
    hero_roll: Roll
    hero_attack_force: int
    exchanges: list[Exchange] = field(default_factory=list)


HERO = "hero"
ENEMY = "enemy"


def attack_force(force: int, rng: random.Random | None = None) -> tuple[int, Roll]:
    """Roll a Force d'Attaque.

    Args:
        force: The combatant's Force of the moment.
        rng: Source of chance.

    Returns:
        The Force d'Attaque and the roll behind it.
    """
    roll = roll_2d6(rng)
    return force + roll.total, roll


def engaged_indexes(enemies: list[Combatant], fight_type: str) -> list[int]:
    """Which adversaries the hero faces this assault.

    A `sequential` fight is a queue: the hero meets the next adversary only once
    the one before it is down. Anything else puts every survivor in front of him
    at once, which is what `single` (one adversary) and `simultaneous` both mean.

    Args:
        enemies: Every adversary of the fight, dead ones included.
        fight_type: How the book stages them.

    Returns:
        Indexes into `enemies`, in order.
    """
    alive = [index for index, enemy in enumerate(enemies) if enemy.is_alive]
    if fight_type == SEQUENTIAL:
        return alive[:1]
    return alive


def resolve_assault(
    hero: Combatant,
    enemies: list[Combatant],
    fight_type: str,
    number: int = 1,
    rng: random.Random | None = None,
) -> tuple[Combatant, list[Combatant], Assault]:
    """Play one assault and report it.

    The hero rolls **once** and that single Force d'Attaque is compared to each
    engaged adversary's own, which is the rule the book prints on the page:
    "lancez deux dés pour vous et deux dés pour chacun de vos adversaires".

    Args:
        hero: The hero as he stands.
        enemies: Every adversary of the fight, dead ones included.
        fight_type: How the book stages them.
        number: Which assault this is, for the log.
        rng: Source of chance.

    Returns:
        The hero after the assault, the adversaries after it, and the record of
        what happened.
    """
    hero_force_attaque, hero_roll = attack_force(hero.force, rng)
    assault = Assault(
        number=number, hero_roll=hero_roll, hero_attack_force=hero_force_attaque
    )
    survivors = list(enemies)

    for index in engaged_indexes(enemies, fight_type):
        enemy = survivors[index]
        enemy_force_attaque, enemy_roll = attack_force(enemy.force, rng)
        exchange, hero, survivors[index] = _resolve_exchange(
            hero, enemy, hero_roll, hero_force_attaque, enemy_roll,
            enemy_force_attaque,
        )
        assault.exchanges.append(exchange)
        if not hero.is_alive:
            break

    return hero, survivors, assault


def combat_status(hero: Combatant, enemies: list[Combatant]) -> str:
    """Say whether the fight is over, and for whom.

    Args:
        hero: The hero as he stands.
        enemies: Every adversary of the fight.

    Returns:
        `ONGOING`, `VICTORY` when every adversary is down, or `DEFEAT`.
    """
    if not hero.is_alive:
        return DEFEAT
    if all(not enemy.is_alive for enemy in enemies):
        return VICTORY
    return ONGOING


def _resolve_exchange(
    hero: Combatant,
    enemy: Combatant,
    hero_roll: Roll,
    hero_force_attaque: int,
    enemy_roll: Roll,
    enemy_force_attaque: int,
) -> tuple[Exchange, Combatant, Combatant]:
    """Settle the hero against one adversary.

    A divine judgement short-circuits the comparison entirely: no gap is
    computed and no adjustment applies, the loser simply dies.

    Args:
        hero: The hero as he stands.
        enemy: The adversary he faces.
        hero_roll: The hero's dice, shared by every exchange of the assault.
        hero_force_attaque: The hero's Force d'Attaque.
        enemy_roll: This adversary's dice.
        enemy_force_attaque: This adversary's Force d'Attaque.

    Returns:
        The record of the exchange, the hero after it, and the adversary after
        it.
    """
    if hero_roll.is_double_six:
        return (
            _judgement(enemy, enemy_roll, enemy_force_attaque, HERO),
            hero,
            enemy.slain(),
        )
    if enemy_roll.is_double_one:
        return (
            _judgement(enemy, enemy_roll, enemy_force_attaque, ENEMY),
            hero.slain(),
            enemy,
        )

    gap = hero_force_attaque - enemy_force_attaque
    if gap > 0:
        damage = gap + hero.damage_adjustment
        return (
            Exchange(
                enemy.name, enemy.force, enemy_roll, enemy_force_attaque, HERO,
                damage,
            ),
            hero,
            enemy.wounded_by(damage),
        )
    if gap < 0:
        damage = -gap + enemy.damage_adjustment
        return (
            Exchange(
                enemy.name, enemy.force, enemy_roll, enemy_force_attaque, ENEMY,
                damage,
            ),
            hero.wounded_by(damage),
            enemy,
        )
    return (
        Exchange(enemy.name, enemy.force, enemy_roll, enemy_force_attaque, None, 0),
        hero,
        enemy,
    )


def _judgement(
    enemy: Combatant, enemy_roll: Roll, enemy_force_attaque: int, winner: str
) -> Exchange:
    """Build the record of a kill by divine judgement."""
    return Exchange(
        enemy_name=enemy.name,
        enemy_force=enemy.force,
        enemy_roll=enemy_roll,
        enemy_attack_force=enemy_force_attaque,
        winner=winner,
        damage=0,
        divine_judgement=True,
    )
