//! The memorial, read in the browser.
//!
//! Ported from `TestTheMemorialPage` in `tests/test_fallen_heroes.py`.

#[path = "common/mod.rs"]
mod common;

use common::{at, fighting_client, hero, FakeDb};
use haute_tension::core::models::hybrid_document::Field;

/// Lay the one stored hero to rest, in place.
async fn bury(db: &FakeDb, page: Option<&str>, cause: Option<&str>) {
    let mut game = db.table("games").one().await;
    game.insert("vie_actuelle".to_string(), Field::Int(0));
    game.insert(
        "died_at".to_string(),
        Field::Instant(at(2026, 9, 10, 12, 0)),
    );
    if let Some(page) = page {
        game.insert("died_on_page".to_string(), Field::Str(page.to_string()));
    }
    if let Some(cause) = cause {
        game.insert("died_of".to_string(), Field::Str(cause.to_string()));
    }
    db.table("games").seed(vec![game]).await;
}

#[tokio::test]
async fn it_says_when_nobody_has_fallen() {
    let (_db, client) = hero().await;

    let body = client.get("/heroes").await.text();

    assert!(body.contains("Personne n'est encore tombé"));
}

#[tokio::test]
async fn a_fallen_hero_is_shown() {
    let (db, client) = hero().await;
    bury(&db, Some("22"), Some("collecteur d'impots")).await;

    let body = client.get("/heroes").await.text();

    assert!(body.contains("Paragraphe 22"));
    assert!(body.contains("collecteur d&#39;impots"));
}

#[tokio::test]
async fn what_he_carried_is_shown() {
    let (db, client) = hero().await;
    bury(&db, None, None).await;

    let body = client.get("/heroes").await.text();

    assert!(body.contains("épée"));
    assert!(body.contains("ration"));
}

#[tokio::test]
async fn it_is_reachable_from_every_page() {
    let (_db, client) = hero().await;

    for path in ["/", "/book/1", "/game", "/heroes"] {
        assert!(
            client.get(path).await.text().contains(r#"href="/heroes""#),
            "on {path}"
        );
    }
}

#[tokio::test]
async fn a_reader_without_a_hero_may_still_read_it() {
    let (_db, client) = fighting_client().await;

    let response = client.get("/heroes").await;

    assert_eq!(response.status, 200);
    assert!(response.text().contains("Créer un héros"));
}

#[tokio::test]
async fn the_dead_hero_sheet_says_so() {
    let (db, client) = hero().await;
    bury(&db, Some("22"), Some("un troll")).await;

    let body = client.get("/game").await.text();

    assert!(body.contains("Ce héros est mort"));
    assert!(body.contains("au paragraphe 22"));
    assert!(body.contains("Repartir de zéro"));
    assert!(!body.contains("Abandonner et créer"));
}
