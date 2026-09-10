"""Following a choice through the browser: what it costs, and what waits."""

import re

BOOK = "pretre_jean/forteresse_alamuth"


def text_of(response):
    return response.get_data(as_text=True)


def stored(fake_db):
    return fake_db["games"].docs[0]


def waiting(fake_db):
    """What is waiting on the reader — absent from the document when empty."""
    return stored(fake_db).get("pending", [])


class TestTheHoldingsBanner:
    """What the reader carries, on every page."""

    def test_it_shows_force_vie_and_gold(self, hero, fake_db):
        game = stored(fake_db)
        body = text_of(hero.get("/book/1"))

        assert f"<strong>{game['force']}</strong>" in body
        assert f"<strong>{game['gold']}</strong>" in body

    def test_it_says_how_much_is_in_the_bag(self, hero):
        """What is *in* it is a link away; the banner only counts it."""
        body = text_of(hero.get("/book/1"))
        banner = re.search(r'<p class="holdings">.*?</p>', body, re.S).group(0)

        assert "Feuille de personnage" in banner
        assert "6" in banner

    def test_a_reader_without_a_hero_sees_none_of_it(self, client):
        assert 'class="holdings"' not in text_of(client.get("/book/1"))


class TestTheCostOnAChoice:
    """What a choice will cost, before it is taken."""

    def test_it_is_shown_next_to_the_choice(self, hero):
        body = text_of(hero.get("/book/1"))

        assert "+1 clé" in body
        assert "-3 pièces d&#39;or" in body

    def test_a_choice_costing_nothing_shows_nothing(self, hero):
        """The class name lives in base.html's stylesheet; the element must not."""
        body = text_of(hero.get("/book/2"))

        assert '<span class="choice-cost">' not in body


class TestFollowingAChoice:
    """Taking one, and paying for it."""

    def test_a_choice_is_a_form_when_there_is_a_hero(self, hero):
        """Following a choice spends things, so it is a POST and not a link."""
        body = text_of(hero.get("/book/1"))

        assert 'action="/book/1/choice/0"' in body

    def test_a_choice_is_a_plain_link_without_a_hero(self, client):
        body = text_of(client.get("/book/1"))

        assert 'href="/book/2"' in body
        assert "/choice/0" not in body

    def test_taking_it_pays_and_moves_on(self, hero, fake_db):
        before = stored(fake_db)["gold"]

        response = hero.post("/book/1/choice/0")

        assert response.headers["Location"] == "/book/2"
        assert stored(fake_db)["gold"] == before - 3

    def test_taking_it_picks_up_what_it_gives(self, hero, fake_db):
        hero.post("/book/1/choice/0")

        assert any(i["element"] == "key" for i in stored(fake_db)["items"])

    def test_the_redirect_keeps_a_reload_from_paying_twice(self, hero, fake_db):
        hero.post("/book/1/choice/0")
        paid_once = stored(fake_db)["gold"]

        for _ in range(3):
            hero.get("/book/2")

        assert stored(fake_db)["gold"] == paid_once

    def test_an_unknown_page_takes_nothing(self, hero):
        assert hero.post("/book/999/choice/0").status_code == 404

    def test_an_index_naming_no_choice_takes_nothing(self, hero):
        assert hero.post("/book/1/choice/7").status_code == 404

    def test_a_reader_without_a_hero_is_still_moved_on(self, fighting_client):
        response = fighting_client.post("/book/1/choice/0")

        assert response.headers["Location"] == "/book/2"


class TestWaitingChanges:
    """The ones only the reader can rule on."""

    def test_a_conditional_choice_leaves_something_waiting(self, hero, fake_db):
        hero.post("/book/1/choice/1")

        assert len(waiting(fake_db)) == 1

    def test_it_is_not_applied_on_its_own(self, hero, fake_db):
        before = stored(fake_db)["force"]
        hero.post("/book/1/choice/1")

        assert stored(fake_db)["force"] == before

    def test_the_page_shows_it_with_its_condition(self, hero):
        hero.post("/book/1/choice/1")
        body = text_of(hero.get("/book/4"))

        assert "À vous de voir" in body
        assert "+1 point de Force" in body
        assert "porterez" in body

    def test_applying_it_moves_the_hero(self, hero, fake_db):
        before = stored(fake_db)["force"]
        hero.post("/book/1/choice/1")

        hero.post("/game/pending/0/apply", data={"page": "4"})

        assert stored(fake_db)["force"] == before + 1
        assert waiting(fake_db) == []

    def test_dismissing_it_leaves_the_hero_alone(self, hero, fake_db):
        before = stored(fake_db)["force"]
        hero.post("/book/1/choice/1")

        hero.post("/game/pending/0/dismiss", data={"page": "4"})

        assert stored(fake_db)["force"] == before
        assert waiting(fake_db) == []

    def test_dismissing_everything_clears_the_lot(self, hero, fake_db):
        hero.post("/book/1/choice/1")

        hero.post("/game/pending/dismiss", data={"page": "4"})

        assert waiting(fake_db) == []

    def test_the_answer_goes_back_to_the_page_it_was_posted_from(self, hero):
        hero.post("/book/1/choice/1")

        response = hero.post("/game/pending/0/apply", data={"page": "4"})

        assert response.headers["Location"] == "/book/4"

    def test_a_form_naming_no_page_goes_to_the_sheet(self, hero):
        hero.post("/book/1/choice/1")

        response = hero.post("/game/pending/0/apply")

        assert response.headers["Location"].endswith("/game")

    def test_a_form_naming_nonsense_goes_to_the_sheet(self, hero):
        hero.post("/book/1/choice/1")

        response = hero.post("/game/pending/0/apply", data={"page": "ailleurs"})

        assert response.headers["Location"].endswith("/game")

    def test_ruling_without_a_hero_is_harmless(self, fighting_client):
        assert fighting_client.post("/game/pending/0/apply").status_code == 302
        assert fighting_client.post("/game/pending/0/dismiss").status_code == 302
        assert fighting_client.post("/game/pending/dismiss").status_code == 302

    def test_a_page_with_nothing_waiting_shows_no_panel(self, hero):
        assert "À vous de voir" not in text_of(hero.get("/book/1"))


class TestTheSheet:
    """The bag and the purse on the character sheet."""

    def test_the_purse_shows_its_throws(self, hero, fake_db):
        body = text_of(hero.get("/game"))

        assert "2D6 deux fois" in body
        assert f"<strong>{stored(fake_db)['gold']}</strong>" in body

    def test_the_bag_is_listed(self, hero):
        body = text_of(hero.get("/game"))

        assert "Équipement" in body
        assert re.search(r"épée", body)
        assert "×4" in body

    def test_an_emptied_bag_says_so(self, hero, fake_db):
        game = stored(fake_db)
        game["items"] = []
        fake_db["games"].docs = [game]

        assert "Votre sac est vide" in text_of(hero.get("/game"))


class TestTheSheetLink:
    """Getting from the story to the sheet, at any moment."""

    def test_the_banner_links_to_the_sheet(self, hero):
        assert 'href="/game"' in text_of(hero.get("/book/1"))

    def test_the_link_counts_what_is_carried(self, hero):
        """A sword, a bag and four rations: six things."""
        link = re.search(
            r'class="bag-link".*?</a>', text_of(hero.get("/book/1")), re.S
        ).group(0)

        assert "6 objets" in re.sub(r"\s+", " ", link)

    def test_one_thing_is_not_written_plural(self, hero, fake_db):
        game = stored(fake_db)
        game["items"] = [{"element": "sword", "label": "épée", "count": 1}]
        fake_db["games"].docs = [game]

        link = re.search(
            r'class="bag-link".*?</a>', text_of(hero.get("/book/1")), re.S
        ).group(0)

        assert "1 objet)" in re.sub(r"\s+", " ", link)

    def test_an_empty_bag_says_so_rather_than_counting(self, hero, fake_db):
        game = stored(fake_db)
        game["items"] = []
        fake_db["games"].docs = [game]

        assert "sac vide" in text_of(hero.get("/book/1"))

    def test_the_banner_does_not_list_every_item(self, hero):
        """It grew past what a line on every page can hold."""
        banner = re.search(
            r'<p class="holdings">.*?</p>', text_of(hero.get("/book/1")), re.S
        ).group(0)

        assert "ration" not in banner

    def test_every_page_carries_the_link_in_its_header(self, hero):
        """Reachable at any moment, hero or not, page or fight."""
        for path in ("/", "/book/1", "/book/2", "/game", "/heroes"):
            assert 'class="site-sheet" href="/game"' in text_of(hero.get(path))

    def test_a_reader_without_a_hero_gets_no_banner_link(self, client):
        assert 'class="bag-link"' not in text_of(client.get("/book/1"))


class TestTheSheet:
    """Everything about the hero, on one page."""

    def test_it_shows_vie_out_of_its_maximum(self, hero, fake_db):
        game = stored(fake_db)
        body = text_of(hero.get("/game"))

        assert f"<strong>{game['vie_actuelle']}</strong>" in body
        assert f"/{game['vie_max']}" in body

    def test_it_shows_force_and_gold(self, hero, fake_db):
        game = stored(fake_db)
        body = text_of(hero.get("/game"))

        assert f"<strong>{game['force']}</strong>" in body
        assert f"<strong>{game['gold']}</strong>" in body

    def test_it_shows_the_throw_behind_each_one(self, hero):
        body = text_of(hero.get("/game"))

        assert re.search(r"\d \+ \d \(2D6\)\s*\+ 6 = \d+", body)
        assert re.search(r"\d \+ \d \(2D6\)\s*\+ 18 = \d+", body)
        assert "2D6 deux fois" in body

    def test_the_vie_gauge_matches_the_vie(self, hero, fake_db):
        game = stored(fake_db)
        game["vie_actuelle"] = game["vie_max"] // 2
        fake_db["games"].docs = [game]

        gauge = re.search(r'<div class="gauge".*?</div>', text_of(hero.get("/game")), re.S)

        assert "width: 50%" in gauge.group(0)

    def test_a_full_gauge_is_full(self, hero):
        gauge = re.search(r'<div class="gauge".*?</div>', text_of(hero.get("/game")), re.S)

        assert "width: 100%" in gauge.group(0)

    def test_it_lists_the_bag(self, hero):
        body = text_of(hero.get("/game"))

        assert "Équipement" in body
        assert "épée" in body
        assert "×4" in body

    def test_an_empty_bag_says_so(self, hero, fake_db):
        game = stored(fake_db)
        game["items"] = []
        fake_db["games"].docs = [game]

        # Literal template text, so Jinja leaves the apostrophe alone.
        assert "Vous n'avez plus rien" in text_of(hero.get("/game"))

    def test_it_offers_the_way_back_to_the_story(self, hero):
        hero.get("/book/2")

        assert 'href="/book/2"' in text_of(hero.get("/game"))

    def test_the_way_back_is_the_last_page_read(self, hero):
        hero.get("/book/2")
        hero.get("/book/1")

        assert 'href="/book/1"' in text_of(hero.get("/game"))

    def test_the_sheet_itself_is_not_a_page_read(self, hero, fake_db):
        """Otherwise the way back would point at the sheet."""
        hero.get("/book/2")
        hero.get("/game")

        assert [v["page"] for v in fake_db["page_views"].docs][-1] == "2"

    def test_nothing_read_yet_offers_the_opening_page(self, hero):
        body = text_of(hero.get("/game"))

        assert "Commencer l'aventure" in body

    def test_an_open_fight_is_shown(self, hero):
        hero.get("/combat/1")

        body = text_of(hero.get("/game"))

        assert "Combat en cours" in body
        assert "Reprendre le combat" in body

    def test_it_links_to_the_fallen(self, hero):
        assert 'href="/heroes"' in text_of(hero.get("/game"))
