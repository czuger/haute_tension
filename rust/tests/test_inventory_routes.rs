//! Following a choice through the browser: what it costs, and what waits.
//!
//! Ported from `tests/test_inventory_routes.py`. The first `TestTheSheet` class
//! of the Python file was shadowed by a second class of the same name and never
//! ran; its three tests are ported here under their own names, and the one that
//! asserted text the template does not carry is noted in `MIGRATION.md`.

#[path = "common/mod.rs"]
mod common;

use common::{client, fighting_client, hero, FakeDb, TestClient};
use haute_tension::core::models::hybrid_document::{Field, RowDict};
use haute_tension::row;
use regex::Regex;

/// The one stored game, as it now stands.
async fn stored(db: &FakeDb) -> RowDict {
    db.table("games").one().await
}

/// What is waiting on the reader — absent from the document when empty.
async fn waiting(db: &FakeDb) -> Vec<Field> {
    stored(db)
        .await
        .get("pending")
        .and_then(Field::as_list)
        .map(<[Field]>::to_vec)
        .unwrap_or_default()
}

/// Give the one stored game a bag of exactly these lines.
async fn carry(db: &FakeDb, lines: Vec<RowDict>) {
    let mut game = stored(db).await;
    game.insert(
        "items".to_string(),
        Field::List(lines.into_iter().map(Field::Dict).collect()),
    );
    db.table("games").seed(vec![game]).await;
}

/// One part of a page, matched whole.
async fn part_of(client: &TestClient, path: &str, pattern: &str) -> String {
    let body = client.get(path).await.text();
    Regex::new(pattern)
        .expect("the pattern is written here")
        .find(&body)
        .unwrap_or_else(|| panic!("{pattern} is on {path}"))
        .as_str()
        .to_string()
}

/// The same text with every run of whitespace collapsed.
fn tightened(text: &str) -> String {
    Regex::new(r"\s+")
        .expect("the pattern is written here")
        .replace_all(text, " ")
        .into_owned()
}

/// What the reader carries, on every page.
mod the_holdings_banner {
    use super::*;

    #[tokio::test]
    async fn it_shows_force_vie_and_gold() {
        let (db, client) = hero().await;
        let game = stored(&db).await;
        let body = client.get("/book/1").await.text();

        assert!(body.contains(&format!("<strong>{}</strong>", game["force"])));
        assert!(body.contains(&format!("<strong>{}</strong>", game["gold"])));
    }

    /// What is *in* it is a link away; the banner only counts it.
    #[tokio::test]
    async fn it_says_how_much_is_in_the_bag() {
        let (_db, client) = hero().await;

        let banner = part_of(&client, "/book/1", r#"(?s)<p class="holdings">.*?</p>"#).await;

        assert!(banner.contains("Feuille de personnage"));
        assert!(banner.contains('6'));
    }

    #[tokio::test]
    async fn a_reader_without_a_hero_sees_none_of_it() {
        let (_db, client) = client().await;

        assert!(!client
            .get("/book/1")
            .await
            .text()
            .contains(r#"class="holdings""#));
    }
}

/// What a choice will cost, before it is taken.
mod the_cost_on_a_choice {
    use super::*;

    #[tokio::test]
    async fn it_is_shown_next_to_the_choice() {
        let (_db, client) = hero().await;

        let body = client.get("/book/1").await.text();

        assert!(body.contains("+1 clé"));
        assert!(body.contains("-3 pièces d&#39;or"));
    }

    /// The class name lives in the stylesheet; the element must not.
    #[tokio::test]
    async fn a_choice_costing_nothing_shows_nothing() {
        let (_db, client) = hero().await;

        let body = client.get("/book/2").await.text();

        assert!(!body.contains(r#"<span class="choice-cost">"#));
    }
}

/// Taking one, and paying for it.
mod following_a_choice {
    use super::*;

    /// Following a choice spends things, so it is a POST and not a link.
    #[tokio::test]
    async fn a_choice_is_a_form_when_there_is_a_hero() {
        let (_db, client) = hero().await;

        let body = client.get("/book/1").await.text();

        assert!(body.contains(r#"action="/book/1/choice/0""#));
    }

    #[tokio::test]
    async fn a_choice_is_a_plain_link_without_a_hero() {
        let (_db, client) = client().await;

        let body = client.get("/book/1").await.text();

        assert!(body.contains(r#"href="/book/2""#));
        assert!(!body.contains("/choice/0"));
    }

    #[tokio::test]
    async fn taking_it_pays_and_moves_on() {
        let (db, client) = hero().await;
        let before = stored(&db).await["gold"].as_int().expect("a purse");

        let response = client.post("/book/1/choice/0").await;

        assert_eq!(response.location(), "/book/2");
        assert_eq!(
            stored(&db).await["gold"],
            Field::Int(before - 3)
        );
    }

    #[tokio::test]
    async fn taking_it_picks_up_what_it_gives() {
        let (db, client) = hero().await;
        client.post("/book/1/choice/0").await;

        let items = stored(&db).await["items"]
            .as_list()
            .expect("a hero carries a bag")
            .to_vec();
        assert!(items.iter().any(|item| {
            item.as_dict()
                .is_some_and(|line| line["element"] == Field::Str("key".to_string()))
        }));
    }

    #[tokio::test]
    async fn the_redirect_keeps_a_reload_from_paying_twice() {
        let (db, client) = hero().await;
        client.post("/book/1/choice/0").await;
        let paid_once = stored(&db).await["gold"].clone();

        for _ in 0..3 {
            client.get("/book/2").await;
        }

        assert_eq!(stored(&db).await["gold"], paid_once);
    }

    #[tokio::test]
    async fn an_unknown_page_takes_nothing() {
        let (_db, client) = hero().await;

        assert_eq!(client.post("/book/999/choice/0").await.status, 404);
    }

    #[tokio::test]
    async fn an_index_naming_no_choice_takes_nothing() {
        let (_db, client) = hero().await;

        assert_eq!(client.post("/book/1/choice/7").await.status, 404);
    }

    #[tokio::test]
    async fn a_reader_without_a_hero_is_still_moved_on() {
        let (_db, client) = fighting_client().await;

        let response = client.post("/book/1/choice/0").await;

        assert_eq!(response.location(), "/book/2");
    }
}

/// The ones only the reader can rule on.
mod waiting_changes {
    use super::*;

    #[tokio::test]
    async fn a_conditional_choice_leaves_something_waiting() {
        let (db, client) = hero().await;
        client.post("/book/1/choice/1").await;

        assert_eq!(waiting(&db).await.len(), 1);
    }

    #[tokio::test]
    async fn it_is_not_applied_on_its_own() {
        let (db, client) = hero().await;
        let before = stored(&db).await["force"].clone();
        client.post("/book/1/choice/1").await;

        assert_eq!(stored(&db).await["force"], before);
    }

    #[tokio::test]
    async fn the_page_shows_it_with_its_condition() {
        let (_db, client) = hero().await;
        client.post("/book/1/choice/1").await;
        let body = client.get("/book/4").await.text();

        assert!(body.contains("À vous de voir"));
        assert!(body.contains("+1 point de Force"));
        assert!(body.contains("porterez"));
    }

    #[tokio::test]
    async fn applying_it_moves_the_hero() {
        let (db, client) = hero().await;
        let before = stored(&db).await["force"].as_int().expect("a Force");
        client.post("/book/1/choice/1").await;

        client
            .post_form("/game/pending/0/apply", &[("page", "4")])
            .await;

        assert_eq!(stored(&db).await["force"], Field::Int(before + 1));
        assert!(waiting(&db).await.is_empty());
    }

    #[tokio::test]
    async fn dismissing_it_leaves_the_hero_alone() {
        let (db, client) = hero().await;
        let before = stored(&db).await["force"].clone();
        client.post("/book/1/choice/1").await;

        client
            .post_form("/game/pending/0/dismiss", &[("page", "4")])
            .await;

        assert_eq!(stored(&db).await["force"], before);
        assert!(waiting(&db).await.is_empty());
    }

    #[tokio::test]
    async fn dismissing_everything_clears_the_lot() {
        let (db, client) = hero().await;
        client.post("/book/1/choice/1").await;

        client
            .post_form("/game/pending/dismiss", &[("page", "4")])
            .await;

        assert!(waiting(&db).await.is_empty());
    }

    #[tokio::test]
    async fn the_answer_goes_back_to_the_page_it_was_posted_from() {
        let (_db, client) = hero().await;
        client.post("/book/1/choice/1").await;

        let response = client
            .post_form("/game/pending/0/apply", &[("page", "4")])
            .await;

        assert_eq!(response.location(), "/book/4");
    }

    #[tokio::test]
    async fn a_form_naming_no_page_goes_to_the_sheet() {
        let (_db, client) = hero().await;
        client.post("/book/1/choice/1").await;

        let response = client.post("/game/pending/0/apply").await;

        assert!(response.location().ends_with("/game"));
    }

    #[tokio::test]
    async fn a_form_naming_nonsense_goes_to_the_sheet() {
        let (_db, client) = hero().await;
        client.post("/book/1/choice/1").await;

        let response = client
            .post_form("/game/pending/0/apply", &[("page", "ailleurs")])
            .await;

        assert!(response.location().ends_with("/game"));
    }

    #[tokio::test]
    async fn ruling_without_a_hero_is_harmless() {
        let (_db, client) = fighting_client().await;

        assert_eq!(client.post("/game/pending/0/apply").await.status, 302);
        assert_eq!(client.post("/game/pending/0/dismiss").await.status, 302);
        assert_eq!(client.post("/game/pending/dismiss").await.status, 302);
    }

    #[tokio::test]
    async fn a_page_with_nothing_waiting_shows_no_panel() {
        let (_db, client) = hero().await;

        assert!(!client
            .get("/book/1")
            .await
            .text()
            .contains("À vous de voir"));
    }
}

/// Getting from the story to the sheet, at any moment.
mod the_sheet_link {
    use super::*;

    #[tokio::test]
    async fn the_banner_links_to_the_sheet() {
        let (_db, client) = hero().await;

        assert!(client.get("/book/1").await.text().contains(r#"href="/game""#));
    }

    /// A sword, a bag and four rations: six things.
    #[tokio::test]
    async fn the_link_counts_what_is_carried() {
        let (_db, client) = hero().await;

        let link = part_of(&client, "/book/1", r#"(?s)class="bag-link".*?</a>"#).await;

        assert!(tightened(&link).contains("6 objets"));
    }

    #[tokio::test]
    async fn one_thing_is_not_written_plural() {
        let (db, client) = hero().await;
        carry(
            &db,
            vec![row! { "element" => "sword", "label" => "épée", "count" => 1_i64 }],
        )
        .await;

        let link = part_of(&client, "/book/1", r#"(?s)class="bag-link".*?</a>"#).await;

        assert!(tightened(&link).contains("1 objet)"));
    }

    #[tokio::test]
    async fn an_empty_bag_says_so_rather_than_counting() {
        let (db, client) = hero().await;
        carry(&db, Vec::new()).await;

        assert!(client.get("/book/1").await.text().contains("sac vide"));
    }

    /// It grew past what a line on every page can hold.
    #[tokio::test]
    async fn the_banner_does_not_list_every_item() {
        let (_db, client) = hero().await;

        let banner = part_of(&client, "/book/1", r#"(?s)<p class="holdings">.*?</p>"#).await;

        assert!(!banner.contains("ration"));
    }

    /// Reachable at any moment, hero or not, page or fight.
    #[tokio::test]
    async fn every_page_carries_the_link_in_its_header() {
        let (_db, client) = hero().await;

        for path in ["/", "/book/1", "/book/2", "/game", "/heroes"] {
            assert!(
                client
                    .get(path)
                    .await
                    .text()
                    .contains(r#"class="site-sheet" href="/game""#),
                "on {path}"
            );
        }
    }

    #[tokio::test]
    async fn a_reader_without_a_hero_gets_no_banner_link() {
        let (_db, client) = client().await;

        assert!(!client
            .get("/book/1")
            .await
            .text()
            .contains(r#"class="bag-link""#));
    }
}

/// The bag and the purse on the character sheet.
///
/// The three tests of this group were shadowed in the Python file and never
/// ran; two of them are ported as they stand.
mod the_bag_and_the_purse {
    use super::*;

    #[tokio::test]
    async fn the_purse_shows_its_throws() {
        let (db, client) = hero().await;

        let body = client.get("/game").await.text();

        assert!(body.contains("2D6 deux fois"));
        assert!(body.contains(&format!("<strong>{}</strong>", stored(&db).await["gold"])));
    }

    #[tokio::test]
    async fn the_bag_is_listed() {
        let (_db, client) = hero().await;

        let body = client.get("/game").await.text();

        assert!(body.contains("Équipement"));
        assert!(body.contains("épée"));
        assert!(body.contains("×4"));
    }
}

/// Everything about the hero, on one page.
mod the_sheet {
    use super::*;

    #[tokio::test]
    async fn it_shows_vie_out_of_its_maximum() {
        let (db, client) = hero().await;
        let game = stored(&db).await;
        let body = client.get("/game").await.text();

        assert!(body.contains(&format!("<strong>{}</strong>", game["vie_actuelle"])));
        assert!(body.contains(&format!("/{}", game["vie_max"])));
    }

    #[tokio::test]
    async fn it_shows_force_and_gold() {
        let (db, client) = hero().await;
        let game = stored(&db).await;
        let body = client.get("/game").await.text();

        assert!(body.contains(&format!("<strong>{}</strong>", game["force"])));
        assert!(body.contains(&format!("<strong>{}</strong>", game["gold"])));
    }

    #[tokio::test]
    async fn it_shows_the_throw_behind_each_one() {
        let (_db, client) = hero().await;

        let body = client.get("/game").await.text();

        assert!(Regex::new(r"\d \+ \d \(2D6\)\s*\+ 6 = \d+")
            .expect("the pattern is written here")
            .is_match(&body));
        assert!(Regex::new(r"\d \+ \d \(2D6\)\s*\+ 18 = \d+")
            .expect("the pattern is written here")
            .is_match(&body));
        assert!(body.contains("2D6 deux fois"));
    }

    /// Set rather than rolled: a hero whose Vie is odd has no exact half.
    #[tokio::test]
    async fn the_vie_gauge_matches_the_vie() {
        let (db, client) = hero().await;
        let mut game = stored(&db).await;
        game.insert("vie_max".to_string(), Field::Int(24));
        game.insert("vie_actuelle".to_string(), Field::Int(12));
        db.table("games").seed(vec![game]).await;

        let gauge = part_of(&client, "/game", r#"(?s)<div class="gauge".*?</div>"#).await;

        assert!(gauge.contains("width: 50%"));
    }

    #[tokio::test]
    async fn a_full_gauge_is_full() {
        let (_db, client) = hero().await;

        let gauge = part_of(&client, "/game", r#"(?s)<div class="gauge".*?</div>"#).await;

        assert!(gauge.contains("width: 100%"));
    }

    #[tokio::test]
    async fn it_lists_the_bag() {
        let (_db, client) = hero().await;

        let body = client.get("/game").await.text();

        assert!(body.contains("Équipement"));
        assert!(body.contains("épée"));
        assert!(body.contains("×4"));
    }

    /// Literal template text, so the apostrophe reaches the page as it is.
    #[tokio::test]
    async fn an_empty_bag_says_so() {
        let (db, client) = hero().await;
        carry(&db, Vec::new()).await;

        assert!(client
            .get("/game")
            .await
            .text()
            .contains("Vous n'avez plus rien"));
    }

    #[tokio::test]
    async fn it_offers_the_way_back_to_the_story() {
        let (_db, client) = hero().await;
        client.get("/book/2").await;

        assert!(client.get("/game").await.text().contains(r#"href="/book/2""#));
    }

    #[tokio::test]
    async fn the_way_back_is_the_last_page_read() {
        let (_db, client) = hero().await;
        client.get("/book/2").await;
        client.get("/book/1").await;

        assert!(client.get("/game").await.text().contains(r#"href="/book/1""#));
    }

    /// Otherwise the way back would point at the sheet.
    #[tokio::test]
    async fn the_sheet_itself_is_not_a_page_read() {
        let (db, client) = hero().await;
        client.get("/book/2").await;
        client.get("/game").await;

        let pages: Vec<String> = db
            .table("page_views")
            .docs()
            .await
            .iter()
            .map(|view| view["page"].to_string())
            .collect();
        assert_eq!(pages.last().map(String::as_str), Some("2"));
    }

    #[tokio::test]
    async fn nothing_read_yet_offers_the_opening_page() {
        let (_db, client) = hero().await;

        assert!(client
            .get("/game")
            .await
            .text()
            .contains("Commencer l'aventure"));
    }

    #[tokio::test]
    async fn an_open_fight_is_shown() {
        let (_db, client) = hero().await;
        client.get("/combat/1").await;

        let body = client.get("/game").await.text();

        assert!(body.contains("Combat en cours"));
        assert!(body.contains("Reprendre le combat"));
    }

    #[tokio::test]
    async fn it_links_to_the_fallen() {
        let (_db, client) = hero().await;

        assert!(client.get("/game").await.text().contains(r#"href="/heroes""#));
    }
}
