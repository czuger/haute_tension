# Combats à revoir — La Forteresse d'Alamuth

Les combats que le moteur (`src/core/combat.rs`) joue mal ou pas du tout, d'après le texte de `books/pretre_jean/forteresse_alamuth/pages.json`. Pour chacun : ce qui ne va pas, les données actuelles du `fight`, le texte de la page, et une place pour vos commentaires.

## Combats décidés au premier assaut

### Page 40

**Problème :**

- Le combat s'arrête au premier assaut : gagné → 408, perdu → 598.
- Dans `FIGHTS_WITH_SPECIAL_RULES` : résolu comme un combat ordinaire, avec un avertissement.

**Données actuelles :** `single` · adversaires : DEMON DU STYX F12 V20 · victoire → `"408"` · défaite → `"598"` · fuite → `null`

**Texte :**

> « Au nom du Christ ! » lancez-vous, retrouvant toute votre fougue de croisé, comme lorsque vous combattiez au côté du roi Richard Coeur de Lion ! Vous savez que cette épreuve peut être décisive dans votre quête.
>
> Le Démon dénude ses crocs acérés.
>
> DEMON DU STYX FORCE: 12 VIE : 20
>
> Si vous remportez le premier assaut. 408
>
> Si le Démon remporte le premier assaut. 598

**Commentaire :**

> 

---

### Page 133

**Problème :**

- Le combat s'arrête au premier assaut : gagné → 419, perdu → 70.
- Dans `FIGHTS_WITH_SPECIAL_RULES` : résolu comme un combat ordinaire, avec un avertissement.

**Données actuelles :** `single` · adversaires : DJINN F16 V20 dégâts+1 · victoire → `"419"` · défaite → `"70"` · fuite → `null`

**Texte :**

> Vous tirez votre épée de son fourreau. Qui est donc ce Djinn peu courtois qui ose vous toiser de si haut ? Votre bouillant sang de croisé ne fait qu'un tour et vous vous jetez sur lui. « Par le Christ et la Croix ! » lancez-vous haut et clair en avançant sur le Djinn. Celui-ci se contente d'éclater de rire et vous distinguez une lueur cruelle dans ses yeux en amande. Préparez-vous à combattre.
>
> DJINN FORCE: 16 VIE: 20
>
> AJUSTEMENT DOMMAGES : + 1
>
> Si vous remportez le premier assaut. 419
>
> Si vous perdez le premier assaut. 70

**Commentaire :**

> 

---

### Page 540

**Problème :**

- Le combat s'arrête au premier assaut : gagné → 505, perdu → 477.
- Dans `FIGHTS_WITH_SPECIAL_RULES` : résolu comme un combat ordinaire, avec un avertissement.

**Données actuelles :** `single` · adversaires : GARDE F6 V12 · victoire → `"505"` · défaite → `"477"` · fuite → `null`

**Texte :**

> Vous quittez rapidement le caravansérail. La nuit est tombée et seul un pâle clair de lune jette sa lueur sur la cour intérieure de la Forteresse. Vous suivez les quatre soldats en vous dissimulant dans l'ombre. Mais alors que vous passez dans une zone éclairée par une torche fixée à la muraille, le dernier garde, tout à fait soûl, arrive derrière vous et vous interpelle. Profitant de son état d'ébriété avancé, vous tirez votre épée.
>
> GARDE FORCE : 6 VIE : 12
>
> Si vous remportez le premier assaut. 505
>
> Si le garde remporte le premier assaut. 477

**Commentaire :**

> 

---

### Page 324

**Problème :**

- Un seul assaut, puis on va en 83 quoi qu'il arrive.
- `outcome` entièrement vide : rien ne dit où aller à la fin du combat.
- Dans `FIGHTS_WITH_SPECIAL_RULES` : résolu comme un combat ordinaire, avec un avertissement.

**Données actuelles :** `single` · adversaires : GARDE DE LA FORTERESSE F8 V12 · victoire → `null` · défaite → `null` · fuite → `null`

**Texte :**

> Vous tirez votre propre épée et vous vous préparez à lancer un défi au gigantesque garde, mais vous n'en avez pas le temps. Celui-ci se rue sur vous en hurlant, les yeux fous !
>
> GARDE DE LA FORTERESSE FORCE : 8 VIE : 12
>
> Après le premier assaut. 83

**Commentaire :**

> 

---

### Page 241

**Problème :**

- Le héros **perd automatiquement le premier assaut** (« Son agilité vous a fait perdre toute chance de remporter le premier assaut »), puis le combat continue normalement.
- `TODO.md` le range parmi les combats « décidés au premier assaut », ce que le texte ne dit pas.
- Dans `FIGHTS_WITH_SPECIAL_RULES` : résolu comme un combat ordinaire, avec un avertissement.

**Données actuelles :** `single` · adversaires : FAUCON TUEUR F10 V7 · victoire → `"484"` · défaite → `"death"` · fuite → `null`

**Texte :**

> Espérant avoir l'avantage de la surprise, vous dégainez votre épée et attaquez l'Oiseleur. D'un battement d'ailes, il prend ses distances et se prépare au combat. Son agilité vous a fait perdre toute chance de remporter le premier assaut. Bien pire, l'Homme- Oiseau tire sur les cordons de cuir qui retiennent le capuchon de son faucon dressé et libère le rapace. Le faucon tueur se jette sur vous en lançant un cri rageur.
>
> FAUCON TUEUR FORCE : 10 VIE : 7
>
> Si vous gagnez ce combat. 484

**Commentaire :**

> 

---

## Combats sous l'eau (la pieuvre)

### Page 335

**Problème :**

- Force du héros divisée par deux (ou −2 en venant de la page 531).
- Chaque 6 lancé ôte 2 points de **Force** à la pieuvre.
- Mort par noyade si pas de victoire en **3 assauts**.
- Dans `FIGHTS_WITH_SPECIAL_RULES` : résolu comme un combat ordinaire, avec un avertissement.

**Données actuelles :** `single` · adversaires : PIEUVRE VENIMEUSE F16 V6 dégâts+1 · victoire → `"243"` · défaite → `"death"` · fuite → `null`

**Texte :**

> Vous êtes déséquilibré par ce soudain retournement de situation et chancelez. Vous tombez dans le bassin et la pieuvre tente de vous attirer vers son bec venimeux.
>
> Si vous avez une dague ou un poignard. 531
>
> Si vous pensez qu'un des objets parmi votre équipement peut vous aider. 18
>
> Sans quoi, il vous faudra affronter la pieuvre en divisant par deux vos points de Force, car vous combattez sous l'eau.
>
> PIEUVRE VENIMEUSE FORCE : 16 VIE : 6
>
> AJUSTEMENT DOMMAGES : + 1
>
> Pour chaque 6 que vous lancez au cours du combat, ôtez immédiatement 2 points de Force au poulpe : vous avez réussi à arracher l'un de ses tentacules. Cependant, si vous ne triomphez pas de ce monstre en trois assauts au plus, vos poumons cèdent sous le manque d'oxygène et vous mourez noyé. La pieuvre dévorera votre cadavre. Si vous remportez ce combat. 243 (

**Commentaire :**

> 

---

### Page 396

**Problème :**

- Force du héros −1. Un double tue la pieuvre d'un coup.
- Chaque 6 lancé ôte 2 points de **Force** à la pieuvre.
- Mort par noyade si pas de victoire en **2 assauts**.
- Dans `FIGHTS_WITH_SPECIAL_RULES` : résolu comme un combat ordinaire, avec un avertissement.

**Données actuelles :** `single` · adversaires : PIEUVRE VENIMEUSE F16 V6 dégâts+1 · victoire → `"243"` · défaite → `"death"` · fuite → `null`

**Texte :**

> Vous plongez, l'épée à la main, pour affronter le monstre. Le choc est rude : la pieuvre était aussi préparée que vous ! Mais vous connaissez son point faible ! Son oeil unique : vous parviendrez à le transpercer, au cours du combat, si vous réussissez un double, portant un coup mortel à la bête. Vous mènerez ce combat sous-marin à 1 point de Force au-dessous de votre total du moment, à cause de l'eau bien sûr. Pour chaque 6 que vous tirerez, ôtez 2 points de Force à la pieuvre : vous aurez tranché l'un de ses tentacules. Cependant si vous ne remportez pas ce duel en deux assauts au plus, vos poumons cèdent sous le manque d'oxygène et vous mourez noyé avant d'être dévoré par le poulpe.
>
> PIEUVRE VENIMEUSE FORCE : 16 VIE : 6 AJUSTEMENT DOMMAGES : + 1
>
> Si vous triomphez. 243

**Commentaire :**

> 

---

### Page 429

**Problème :**

- Chaque 6 lancé ôte 2 points de **Vie** (et non de Force) à la pieuvre.
- Mort par noyade si pas de victoire en **2 assauts**.
- Dans `FIGHTS_WITH_SPECIAL_RULES` : résolu comme un combat ordinaire, avec un avertissement.

**Données actuelles :** `single` · adversaires : PIEUVRE VENIMEUSE F16 V6 dégâts+1 · victoire → `"243"` · défaite → `"death"` · fuite → `null`

**Texte :**

> Vous n'hésitez pas une seconde et plongez dans le bassin pour en remonter cet objet brillant qui pourrait bien être une pièce de l'armure magique dont a parlé Abdul Al-Azhred. Vous nagez jusqu'au fond du bassin, mais à ce moment un tentacule visqueux agrippe votre jambe et vous attire vers un bec acéré ! Un poulpe géant sommeillait dans le bassin, chargé de garder précieusement l'objet que vous convoitiez ! Vous devez le combattre en divisant par deux vos points de Force car vous êtes sous l'eau et n'avez pas le temps de tirer votre épée.
>
> PIEUVRE VENIMEUSE FORCE: 16 VIE : 6
>
> AJUSTEMENT DOMMAGES : + 1
>
> A chaque 6 que vous lancerez au cours du combat, vous lui enlevez automatiquement 2 points de Vie car vous aurez réussi à trancher l'un de ses tentacules. Cependant, si vous ne triomphez pas de ce monstre en deux assauts au plus, vos poumons cèdent sous le manque d'oxygène et vous mourez noyé avant d'être dévoré par la pieuvre.
>
> Si vous remportez ce combat. 243

**Commentaire :**

> 

---

### Page 595

**Problème :**

- Force du héros −2. Un double tue la pieuvre d'un coup.
- Chaque 6 lancé ôte 2 points de **Force** à la pieuvre.
- Mort par noyade si pas de victoire en **4 assauts**.
- Dans `FIGHTS_WITH_SPECIAL_RULES` : résolu comme un combat ordinaire, avec un avertissement.

**Données actuelles :** `single` · adversaires : PIEUVRE VENIMEUSE F16 V6 dégâts+1 · victoire → `"243"` · défaite → `"death"` · fuite → `null`

**Texte :**

> Vous plongez, le poignard à la main, prêt à affronter le monstre. Le choc est rude : la pieuvre était aussi préparée que vous ! Mais vous connaissez son point faible : son oeil unique. Vous le transpercerez, tuant ainsi la bête sur le coup, si, au cours du combat, vous réussissez un double. A cause de l'eau qui vous entoure, vous mènerez ce combat en retirant 2 points de votre total de Force du moment. Pour chaque 6 que vous réussirez, ôtez 2 points de Force à la pieuvre : vous aurez tranché l'un de ses tentacules. Cependant si vous ne remportez pas ce duel en quatre assauts au plus, vos poumons cèdent sous le manque d'oxygène et vous mourez noyé. Le poulpe vous dévorera.
>
> PIEUVRE VENIMEUSE FORCE : 16 VIE : 6 AJUSTEMENT DEGATS : + 1
>
> Si vous triomphez. 243

**Commentaire :**

> 

---

## Branchements en cours de combat

### Page 74

**Problème :**

- À la **première blessure** reçue, aller en 330 avant de continuer le combat.
- Le seul `goto` 330 apparaît comme un choix ordinaire.
- Dans `FIGHTS_WITH_SPECIAL_RULES` : résolu comme un combat ordinaire, avec un avertissement.

**Données actuelles :** `simultaneous` · adversaires : LEPREUX F6 V6 ×2 · victoire → `"10"` · défaite → `"death"` · fuite → `null`

**Texte :**

> Deux lépreux plus hardis que les autres (ou peut-être simplement moins malades) se jettent sur vous en vous voyant sortir votre épée. Ils vous prennent sans doute pour un de leurs tortionnaires.
>
> LEPREUX FORCE : 6 VIE : 6
>
> Vous devez les combattre ensemble. Pour chaque assaut, lancez deux dés pour vous et deux dés pour chacun de vos adversaires. Si votre Force d'Attaque est supérieure à celle de chacun de vos deux adversaires, vous les touchez tous les deux. Si elle est supérieure à celle de l'un d'eux et inférieure à celle de l'autre, vous touchez le premier mais le second vous blesse. Si votre Force d'Attaque est inférieure à celle de vos deux adversaires, ils vous touchent tous les deux.
>
> A la première blessure que les lépreux vous infligent, rendez-vous ici avant de continuer le combat. 330
>
> Si vous tuez vos deux adversaires. 10

**Commentaire :**

> 

---

### Page 611

**Problème :**

- À la **première blessure** reçue, aller en 591 ; victoire en 327 seulement si l'araignée ne vous a jamais touché.
- **Absent de `FIGHTS_WITH_SPECIAL_RULES`** : le moteur le joue comme un combat ordinaire, sans avertissement.

**Données actuelles :** `single` · adversaires : ARAIGNEE GEANTE F12 V12 dégâts+1 · victoire → `"327"` · défaite → `"death"` · fuite → `null`

**Texte :**

> Vous avez touché le fil ! Le choc le fait vibrer imperceptiblement à vos yeux, mais suffisamment pour que ce signal se répercute jusqu'au faîte des majestueux conifères de la forêt. Là, une toile finement élaborée vibre et une grosse boule noire et velue, brusquement tirée de sa léthargie, se glisse entre les branches en tissant un fil solide. Puis, ayant repéré sa future victime, elle se laisse tomber sur elle : en l'occurrence vous ! Ayant mal calculé sa trajectoire, l'Araignée atterrit en souplesse devant vous. Elle déplie ses huit longues pattes tranchantes comme des couteaux et avance sur vous.
>
> Si vous possédez la potion de contrôle des animaux. 365
>
> Aucun autre philtre ou objet magique n'est efficace ! Vous n'avez alors que votre épée et votre courage à opposer au monstre :
>
> ARAIGNEE GEANTE FORCE : 12 VIE : 12
>
> AJUSTEMENT DEGATS : + 1
>
> A la première piqûre ou morsure qu'elle vous inflige. 591
>
> Si vous parvenez à tuer l'arachnide sans qu'elle vous touche. 327

**Commentaire :**

> 

---

### Page 425

**Problème :**

- Le héros **gagne automatiquement le premier assaut** et ôte 2 points de Vie au bourreau.
- Sous **2** points de Vie (et non 3 comme le dit `TODO.md`), aller en 499 : `on_defeat` vaut 499, mais le moteur ne l'atteint qu'à la mort.
- Dans `FIGHTS_WITH_SPECIAL_RULES` : résolu comme un combat ordinaire, avec un avertissement.

**Données actuelles :** `single` · adversaires : BOURREAU F13 V12 · victoire → `"62"` · défaite → `"499"` · fuite → `null`

**Texte :**

> « Vil traître ! Je viens mettre fin à ton règne de terreur, au nom du Christ ! » Lancez-vous en crachant au visage de l'Orque bestial. L'effet de surprise est de votre côté et vous remportez automatiquement le premier assaut. Otez-lui 2 Points de Vie. Cependant il vous faudra compter avec la force exceptionnelle du bourreau !
>
> BOURREAU FORCE: 13 VIE: 12
>
> Si vous tuez l'Orque. 62
>
> Si vous descendez au-dessous de 2 points de Vie. 499

**Commentaire :**

> 

---

### Page 627

**Problème :**

- Sous **2** points de Vie (et non 3 comme le dit `TODO.md`), aller en 206.
- Dans `FIGHTS_WITH_SPECIAL_RULES` : résolu comme un combat ordinaire, avec un avertissement.

**Données actuelles :** `sequential` · adversaires : PREMIER HASCHICHIN F8 V12, DEUXIEME HASCHICHIN F8 V10 · victoire → `"296"` · défaite → `"206"` · fuite → `null`

**Texte :**

> Le Marchand ventripotent vous entraîne tout au fond de son échoppe, dans une salle chichement éclairée. Il vous tend une petite lampe à huile. «Je dois faire attention, tout pourrait facilement flamber dans ma pauvre boutique ! Gémit-il. Voici le tapis que vous cherchez, seigneur ! » Vous vous penchez au-dessus du tapis qu'il désigne et le regardez de près. Vous ne distinguez rien de bien extraordinaire. Vous vous retournez pour demander des explications, juste à temps pour apercevoir deux Haschichins complices du Marchand qui s'apprêtaient à vous faire un mauvais parti. Vous lancez un coup de pied dans la poitrine du premier et tirez votre épée. Vous les combattrez séparément car l'exiguïté de l'arrière-boutique ne leur permet pas de vous attaquer simultanément.
>
> PREMIER HASCHICHIN FORCE : 8 VIE : 12
>
> DEUXIEME HASCHICHIN FORCE: 8 VIE: 10
>
> Si vous parvenez à vaincre ces deux canailles. 296
>
> Si, au cours du combat, vous tombez au- dessous de 2 points de Vie. 206

**Commentaire :**

> 

---

### Page 637

**Problème :**

- Fuite possible seulement **après avoir perdu 6 points de Vie ou plus** ; le moteur propose la fuite (286) dès le départ.
- **Absent de `FIGHTS_WITH_SPECIAL_RULES`.**
- Nom d'adversaire « chaque yllerlon » : le mot « CHAQUE » du livre est resté dans le nom.

**Données actuelles :** `sequential` · adversaires : CHAQUE YLLERLON F10 V8 ×2 · victoire → `"135"` · défaite → `"death"` · fuite → `"286"`

**Texte :**

> Du haut du monticule, vous apercevez un vieillard allongé sur le sol caillouteux. Il s'est brisé la jambe en tombant et tente avec difficulté de repousser de sa canne les assauts d'un oiseau gigantesque.
>
> Pour en avoir entendu plusieurs fois la description par des conteurs, vous reconnaissez l'Yllerlon, un oiseau de feu dont les ailes sont tranchantes comme des rasoirs. Vous vous apprêtez à descendre le combattre lorsque deux autres volatiles viennent rejoindre le premier agresseur. Le vieillard tient l'un des oiseaux en respect tandis que les deux autres se ruent sur vous. Vous devez les affronter séparément :
>
> CHAQUE YLLERLON FORCE : 10 VIE : 8
>
> Après avoir perdu 6 points de Vie ou plus, vous avez la possibilité de fuir en abandonnant le vieillard et en prenant le premier chemin sur votre gauche. 286
>
> Si vous décidez de continuer le combat et anéantissez les Yllerlons. 135

**Commentaire :**

> 

---

### Page 609

**Problème :**

- Fuite (72) permise seulement une fois un des deux Trolls abattu, et seulement si le héros a encore son cheval.
- Dans `FIGHTS_WITH_SPECIAL_RULES` : résolu comme un combat ordinaire, avec un avertissement.

**Données actuelles :** `simultaneous` · adversaires : LEY F14 V12, Bus F12 V14 · victoire → `"394"` · défaite → `"death"` · fuite → `"72"`

**Texte :**

> Vous vous élancez bravement, décidé à forcer le passage. Mais les deux Trolls s'attendaient à votre tentative courageuse quoique peu habile. Ils tournent leurs piques vers vous et vous devez vous résoudre à les combattre. « Tu t'es cru malin en essayant d'échapper à Ley et Bus, les gardiens du pont d'Alazar, pauvre fou ! Tu vas voir ce qu'il t'en coûtera! Nous prendrons tout l'or que nous voudrons sur ton cadavre et nous te laisserons pourrir au soleil ! Ha ha ha ! » Cela dit, les deux énormes Trolls se ruent sur vous. Vous devrez les combattre ensemble. Pour chaque assaut, lancez deux dés pour vous et deux dés pour chacun de vos adversaires. Si votre Force d'Attaque est supérieure à celles de vos deux adversaires, vous les touchez tous les deux. Si elle est supérieure à celle de l'un d'eux et inférieure à celle de l'autre, vous touchez le premier, mais le second vous blesse. Si votre Force d'Attaque est inférieure à celles de vos deux adversaires, ils vous touchent tous les deux.
>
> LEY FORCE : 14 VIE : 12
>
> Bus FORCE: 12 VIE: 14
>
> Si vous sortez vainqueur de ce combat. 394
>
> Vous pouvez également choisir de fuir en sautant du pont et en vous jetant dans l'eau avec votre cheval (si vous ne l'avez pas donné aux Trolls), mais seulement une fois que vous aurez tué l'un de vos adversaires. Si vous choisissez la fuite. 72

**Commentaire :**

> 

---

## Règles particulières non signalées

### Page 120

**Problème :**

- Les coups du héros ne portent pas : il **perd automatiquement deux assauts**, puis va en 642.
- `outcome` entièrement vide, et le combat est jouable comme un combat ordinaire.
- **Absent de `FIGHTS_WITH_SPECIAL_RULES`.**

**Données actuelles :** `single` · adversaires : ELEMENTAL DE L'AIR F8 V8 · victoire → `null` · défaite → `null` · fuite → `null`

**Texte :**

> Vous dégainez votre épée et tentez de frapper l'Elémental de l'air :
>
> ELEMENTAL DE L'AIR FORCE : 8 VIE : 8
>
> Mais vous vous apercevez très vite que vos coups ne portent pas ! Ils traversent la créature des airs sans lui faire le moindre mal, comme un souffle de vent. Par contre, les coups de poing que vous lance l'Elémental font mouche ! Vous perdez donc automatiquement deux assauts. 642

**Commentaire :**

> 

---

### Page 368

**Problème :**

- Au début, un dé : autant de fléaux tranchés, chacun ôte 2 points de Vie à l'escargot ; puis chaque 6 lancé en tranche un de plus.
- Victoire dès que l'escargot tombe à **3 points de Vie ou moins** (663) ; 634 est une autre issue.
- **Absent de `FIGHTS_WITH_SPECIAL_RULES`.**

**Données actuelles :** `single` · adversaires : ESCARGOT MOISSONNEUR F13 V15 dégâts+1 · victoire → `"663"` · défaite → `"death"` · fuite → `null`

**Texte :**

> D'un mouvement tournant, l'Escargot vous a acculé contre le tronc d'unarbre. Vous n'avez pas la possibilité de fuir et vous devez attaquer le monstre, armé de votre seule épée.
>
> ESCARGOT MOISSONNEUR
>
> FORCE: 13 VIE: 15
>
> AJUSTEMENT DEGATS : + 1
>
> Lancez un dé pour déterminer le nombre de fléaux que vous réussissez à lui trancher dès le début du combat. Au cours de l'affrontement, considérez que vous venez de couper un fléau à chaque 6 que vous réussissez. A chaque appendice inutilisable, l'Escargot perd 2 points de Vie.
>
> Lorsque vous aurez réduit l'Escargot à 3 points de Vie ou moins. 663
>
> Si vous l'avez déjà tué ou si vous préférez abréger ses souffrances, vous regagnez le chemin dont vous vous êtes écarté. 634

**Commentaire :**

> 

---

### Page 564

**Problème :**

- Le double a **la Force et la Vie du héros** : pas de bloc de caractéristiques, donc `enemies` vide et pas de bouton « Engager le combat ».
- Le combat final du livre n'est pas jouable.

**Données actuelles :** `single` · adversaires : aucun · victoire → `"625"` · défaite → `"death"` · fuite → `null`

**Texte :**

> L'escalier est plus large que le précédent. Un épais tapis moelleux en couvre les marches, de riches tentures en fil d'or cachent la brique nue des murailles. Vous continuez votre montée qui s'achève sur un palier plongé dans une obscurité totale. Tâtonnant du bout des doigts, vous parvenez à découvrir une porte et à l'ouvrir. Vous pénétrez dans une petite pièce blanche violemment éclairée. Bougies, torches, lampes à huile, globes iridescents sont accrochés aux murs ou suspendus au plafond.
>
> Pas un seul recoin de la salle n'est dans l'ombre, et l'éclat aveuglant de ces centaines de points lumineux blesse vos yeux accoutumés aux ténèbres des souterrains et des sous-bois. Vous vous demandez l'utilité d'une telle pièce quand, de la porte par laquelle vous êtes entré, surgit un guerrier vêtu de noir. Vous l'observez avec attention : son apparence vous est familière... Vous êtes face à votre double magique, l'ultime épreuve qu'Hassan Sabba a placée sur votre chemin. Pour être digne de rencontrer le Vieux de la Montagne, vous devez combattre vos mauvais instincts et vos pensées impies incarnées dans cette enveloppe charnelle à votre image. Attention ! Votre double bénéficie du même total de points de Force et de points de Vie que vous. Ce détail excepté, l'affrontement se déroule comme à l'accoutumée...
>
> Si vous parvenez à détruire le Mal qui vous habitait. 625

**Commentaire :**

> 

---

### Page 635

**Problème :**

- « Vos quatre adversaires » mais le livre n'en donne que trois à tuer (le lieutenant et deux gardes) ; vérifier le `count`.
- Nom d'adversaire « chaque garde » : le mot « CHAQUE » du livre est resté dans le nom.

**Données actuelles :** `sequential` · adversaires : LIEUTENANT DES GARDES F7 V10, CHAQUE GARDE F6 V10 ×2 · victoire → `"137"` · défaite → `"death"` · fuite → `null`

**Texte :**

> Ils semblent se réjouir d'avoir enfin trouvé une occasion de se distraire. Ce village est tellement paisible que les gardes chargés de maintenir l'ordre n'ont pas souvent la possibilité de tester leur adresse au combat. Vos quatre adversaires se tiennent devant vous. Vous devez les affronter séparément :
>
> LIEUTENANT DES GARDES FORCE : 7 VIE : 10
>
> CHAQUE GARDE FORCE : 6 VIE : 10
>
> Lorsque vous aurez tué le lieutenant et deux gardes. 137

**Commentaire :**

> 

---

## Pages qui modifient le combat suivant

Pas des combats elles-mêmes, mais elles changent les règles du combat où elles mènent, et le moteur n'en sait rien.

### Page 81 → combat de la page 347

**Problème :** Le héros perd automatiquement le premier assaut contre le Dogue.

**Texte :**

> Vous déversez la potion verte sur le Dogue. L'animal semble n'en avoir que faire et aboie avec hargne, comme par défi, avant de bondir sur vous. Vous venez de gaspiller une potion de contrôle des plantes. Retournez combattre le Dogue de l'Enfer à l'épée, mais votre erreur vous a coûté du temps. Vous perdez automatiquement le premier assaut. 347

**Commentaire :**

> 

---

### Page 471 → combat de la page 347

**Problème :** Force du héros −3 pour la durée du combat contre le Dogue.

**Texte :**

> Vous débouchez la fiole bleue, mais au moment où vous allez la lancer sur le monstreux animal, celui-ci vous jette un éclair de rage dans les yeux. La substance bleue se répand sur vous. Il s'agit d'un puissant concentré de glu. Vous voilà collé au sol et gêné dans vos mouvements. Retirez 3 points de Force à votre total et combattez le Dogue de l'Enfer à l'épée. A la fin du combat, la glu se dissoudra et vous retrouverez votre total de Force précédent... si vous survivez ! 347

**Commentaire :**

> 

---

### Page 91 → combat de la page 630

**Problème :** Dommages infligés à la Limace divisés par deux tant qu'elle n'a pas perdu la moitié de ses points de Vie.

**Texte :**

> La niche est très étroite et le moindre geste est difficile. Malheureusement pour vous, la Limace a avancé jusqu'à cette anfractuosité et vous bloque toute possibilité d'en sortir. Vous devez combattre malgré tout, mais vos mouvements sont limités. Divisez par 2 l'importance des blessures que vous infligerez à votre adversaire (en arrondissant à l'entier inférieur) tant qu'il n'aura pas perdu la moitié (ou plus) de ses points de Vie. 630

**Commentaire :**

> 

---

### Page 391 → combat de la page 22

**Problème :** Le héros perd automatiquement le premier assaut contre le Collecteur d'impôts.

**Texte :**

> Vous sortez de votre sac la fiole de potion verte. Vous allez enfin savoir de quoi il s'agit ! Sans laisser au Haschichin le temps de réagir vous la lancez sur lui et elle se brise en petits éclats de verre. Mais rien ne se produit ! C'est une potion de contrôle des plantes que vous venez de gaspiller, et elle n'a aucun effet sur les créatures non végétales. Furieux de s'être laissé tromper, le Collecteur d'impôts se jette sur vous. Vous perdez automatiquement le premier assaut, car le Haschichin vous frappe alors que vous attendez encore les effets de la potion ! 22

**Commentaire :**

> 

---

### Page 404 → combat de la page 22

**Problème :** Dommages reçus du Collecteur d'impôts divisés par deux.

**Texte :**

> Vous sortez de votre sac la fiole bleue. Vous allez enfin savoir ce qu'elle contient ! Avant que le Collecteur d'impôts ne puisse réagir, vous la lancez vers lui d'un geste sec. Il peste avec fureur car le voilà englué dans un concentré de colle liquide très puissant. Vous en profitez pour tirer votre épée et vous ruer sur lui. Allez le combattrez. 22
>
> A chaque assaut que remportera le Haschichin, comptez seulement la moitié des points de dommages qu'il vous causera (arrondissez à l'entier inférieur) : en raison de l'action de la glu, il lui est deux fois plus difficile de se mouvoir et de vous toucher !

**Commentaire :**

> 

---

### Page 550 → combat de la page 368

**Problème :** Le héros perd automatiquement le premier assaut contre l'Escargot.

**Texte :**

> Un jet de bave vous atteint à l'oeil. Vous êtes aveuglé un court instant et déjà l'Escargot vous porte le premier coup. Vous perdez le premier assaut. Découvrez les modalités de l'affrontement. 368

**Commentaire :**

> 

---

### Page 323 → combat de la page 503

**Problème :** À chaque assaut gagné, relancer deux dés : 11 ou 12 enfonce la clef de jade et arrête Thalos, le combat est gagné.

**Texte :**

> Pendant que la statue se met pesamment en marche, vous discernez au milieu de sa poitrine un emplacement taillé dans un bloc de jade massif. Il semble s'agir d'une serrure. Vous comprenez que la clef de jade que vous détenez s'encastre là, probablement pour arrêter la statue. A chaque fois que vous remporterez un assaut au cours du combat, vous déduirez normalement les dommages infligés à votre adversaire. Mais vous lancerez les dés une seconde fois. Si vous jetez un 11 ou un 12, cela signifiera que, profitant d'un coup heureux, vous avez réussi à enfoncer la clef de jade au centre de la poitrine de Thalos. Aussitôt la statue s'immobilisera et vous considérerez le combat comme fini.
>
> Pour le moment, la statue vivante s'avance vers vous à grands pas, ses bras fendant l'air. Livrez bataille ! 503

**Commentaire :**

> 

---

### Page 531 → combat de la page 335

**Problème :** Force du héros −2 au lieu d'être divisée par deux, et un double tue la pieuvre.

**Texte :**

> Vous comprenez que votre seul espoir est de viser l'oeil unique du monstre avec votre poignard ou votre dague. Le duel sous-marin commence, mais au lieu de combattre avec seulement la moitié de vos points de Force, vous n'ôterez que 2 points à votre total pour la durée de ce combat. Si vous réussissez un double durant les assauts, c'est que vous avez touché le monstre à mort. Le combat se termine aussitôt en votre faveur. 335

**Commentaire :**

> 
