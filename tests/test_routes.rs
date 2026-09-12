//! Tests for the story data route.

#[path = "common/mod.rs"]
mod common;

use common::{client, BOOK};
use haute_tension::core::models::hybrid_document::Field;

/// Serving one page as JSON, and noting that it was asked for.
mod the_data_route {
    use super::*;

    #[tokio::test]
    async fn a_page_is_returned_with_its_choices() {
        let (_db, client) = client().await;

        let response = client.get("/data/1").await;

        assert_eq!(response.status, 200);
        let body = response.json();
        let gotos: Vec<&str> = body["choices"]
            .as_array()
            .expect("a page carries its choices")
            .iter()
            .filter_map(|choice| choice["goto"].as_str())
            .collect();
        assert_eq!(gotos, ["2"]);
    }

    #[tokio::test]
    async fn the_page_text_is_returned() {
        let (_db, client) = client().await;

        let response = client.get("/data/1").await;

        assert_eq!(response.status, 200);
        assert_eq!(
            response.json()["text"],
            serde_json::json!(["Le début de l'aventure.", "Deuxième ligne."])
        );
    }

    #[tokio::test]
    async fn a_requested_page_is_recorded() {
        let (db, client) = client().await;
        client.get("/data/1").await;

        let views = db.table("page_views").docs().await;

        assert_eq!(views.len(), 1);
        assert_eq!(views[0]["book"], Field::Str(BOOK.to_string()));
        assert_eq!(views[0]["page"], Field::Str("1".to_string()));
    }

    #[tokio::test]
    async fn an_unknown_page_is_recorded_too() {
        let (db, client) = client().await;
        client.get("/data/404").await;

        let pages: Vec<String> = db
            .table("page_views")
            .docs()
            .await
            .iter()
            .map(|view| view["page"].to_string())
            .collect();
        assert_eq!(pages, ["404"]);
    }

    #[tokio::test]
    async fn an_unknown_page_is_reported() {
        let (_db, client) = client().await;

        let response = client.get("/data/404").await;

        assert_eq!(response.status, 200);
        assert_eq!(response.mimetype(), "application/json");
        assert_eq!(
            response.json(),
            serde_json::json!({
                "success": false,
                "message": "le numéro 404 n'a pas été trouvé",
            })
        );
    }

    #[tokio::test]
    async fn the_error_keeps_accented_characters() {
        let (_db, client) = client().await;

        assert!(client.get("/data/404").await.text().contains("numéro"));
    }
}
