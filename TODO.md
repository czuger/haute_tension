# À traiter plus tard

## Combats à règle particulière

Le moteur de `core/combat.py` applique les règles générales de la série
(Force d'Attaque = Force + 2D6, dommages = écart entre les deux FA, double 6 /
double 1). **Treize des quarante-sept combats du livre ajoutent une règle que le
moteur ne modélise pas.** Ils se résolvent aujourd'hui comme des combats
ordinaires, ce qui est faux pour eux et juste pour les trente-quatre autres.

La liste vit dans le code, `combat.FIGHTS_WITH_SPECIAL_RULES`, et l'écran de
combat affiche un avertissement sur ces pages plutôt que de laisser croire au
résultat.

| Pages | Ce que le texte ajoute |
| ----- | ---------------------- |
| 133, 40, 241, 324, 425, 540 | Le combat se décide **au premier assaut**, pas à la mort d'un camp : « Si vous remportez le premier assaut. 419 / Si vous perdez le premier assaut. 70 » |
| 335, 396, 429, 595 | **Nombre d'assauts limité** (« si vous ne triomphez pas en trois assauts au plus, vos poumons cèdent »), et **Force divisée par deux** parce qu'on combat sous l'eau |
| 335, 396, 595 | La Force de l'adversaire **change en cours de combat** : « Pour chaque 6 que vous lancez, ôtez immédiatement 2 points de Force au poulpe » |
| 74 | Branchement **à la première blessure subie** : « A la première blessure que les lépreux vous infligent, rendez-vous ici avant de continuer le combat. 330 » |
| 425, 627 | Branchement sur un **seuil de Vie** : « si, à un moment quelconque du combat, vous tombez au-dessous de 3 points de Vie » |
| 609 | **Fuite conditionnelle** : possible seulement une fois un adversaire abattu |

Deux formes reviennent et couvriraient presque tout :

1. **Un branchement déclenché en cours de combat** — sur l'assaut *n*, sur la
   première blessure, ou sur un seuil de Vie. Une liste de conditions attachée
   au combat, évaluée après chaque assaut, suffirait aux neuf premières pages.
2. **Une Force modifiée**, à l'entrée (divisée par deux) ou pendant (chaque 6
   retire 2 points à l'adversaire).

## Points laissés ouverts

- **Mêlée simultanée** : le moteur suit le livre — un seul jet héros comparé à
  la FA de chaque adversaire (texte des pages 74 et 609). Le TODO d'origine
  disait « jet séparé par adversaire », écarté au profit du texte imprimé.
- **Double 6 en mêlée** : un double 6 du héros tue tous les adversaires engagés
  dans l'assaut, puisqu'il n'y a qu'un jet. Le livre ne traite que le duel.
- **Égalité des FA** : aucun dommage. Les règles ne décrivent les dommages que
  comme l'écart entre les deux FA, et l'écart est nul.
- **Double 6 contre double 1 au même assaut** : le double 6 du héros l'emporte,
  l'adversaire tué net ne frappe pas. Choix, non règle.
- **Inventaire** : les `gains` / `losses` des choix existent dans les données et
  ne sont ni suivis ni appliqués.
- **Vingt et un `fight` fantômes** dans `pages.json` (pages 18, 81, 91, 148…) :
  `enemies` vide et `outcome` nul. Faux positifs du parseur, ignorés.
