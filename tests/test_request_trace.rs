//! Every request written into the general log, and the bodies it will not copy.
//!
//! Every test takes the log lock: the capture belongs to the process, so a test
//! reading it must be the only one writing into it.

// The environment and the log belong to the process, so the lock that
// serialises the tests reading them is held for the length of each one.
#![allow(clippy::await_holding_lock)]

#[path = "common/mod.rs"]
mod common;

use axum::body::Body;
use axum::http::StatusCode;
use common::{client, fighting_client, hero, log_lock};
use haute_tension::application::logs::request_trace::{what_goes_out, STREAMED};
use haute_tension::application::models::game::SESSION_KEY;
use haute_tension::core::logs::general_log::{start_capture, take_capture, Record};
use haute_tension::core::models::hybrid_document::Field;

/// Every logged line starting with `opening`.
fn lines(records: &[Record], opening: &str) -> Vec<String> {
    records
        .iter()
        .map(|record| record.message.clone())
        .filter(|message| message.starts_with(opening))
        .collect()
}

/// The whole log as one text.
fn text_of(records: &[Record]) -> String {
    records
        .iter()
        .map(Record::text)
        .collect::<Vec<String>>()
        .join("\n")
}

/// Two lines per request, and nothing outside the trace.
mod every_request_is_traced {
    use super::*;

    #[tokio::test]
    async fn a_page_leaves_a_request_and_an_answer() {
        let _lock = log_lock();
        let (_db, client) = client().await;
        start_capture();

        client.get("/book/1").await;

        let records = take_capture();
        assert!(!lines(&records, "Request").is_empty());
        assert!(!lines(&records, "Answer").is_empty());
    }

    #[tokio::test]
    async fn the_request_names_what_was_asked_for() {
        let _lock = log_lock();
        let (_db, client) = client().await;
        start_capture();

        client.get("/book/1").await;

        let request = lines(&take_capture(), "Request")[0].clone();
        assert!(request.contains("path='/book/1'"));
        assert!(request.contains("method='GET'"));
        assert!(request.contains("endpoint='web.read_page'"));
    }

    #[tokio::test]
    async fn the_answer_names_its_status_and_time() {
        let _lock = log_lock();
        let (_db, client) = client().await;
        start_capture();

        client.get("/book/1").await;

        let answer = lines(&take_capture(), "Answer")[0].clone();
        assert!(answer.contains("status=200"));
        assert!(answer.contains(" ms'"));
    }

    #[tokio::test]
    async fn an_address_matching_no_route_is_traced_too() {
        let _lock = log_lock();
        let (_db, client) = client().await;
        start_capture();

        client.get("/nowhere-at-all").await;

        let records = take_capture();
        assert!(lines(&records, "Request")[0].contains("path='/nowhere-at-all'"));
        assert!(lines(&records, "Answer")[0].contains("status=404"));
    }

    #[tokio::test]
    async fn the_query_string_is_written() {
        let _lock = log_lock();
        let (_db, client) = client().await;
        start_capture();

        client.get("/book/1?from=here").await;

        assert!(lines(&take_capture(), "Request")[0].contains(r#"query={"from": "here"}"#));
    }

    #[tokio::test]
    async fn the_route_arguments_are_written() {
        let _lock = log_lock();
        let (_db, client) = client().await;
        start_capture();

        client.get("/book/1").await;

        assert!(lines(&take_capture(), "Request")[0].contains(r#"route={"number": 1}"#));
    }
}

/// What is copied, what is described, and what is left alone.
mod bodies {
    use super::*;

    #[tokio::test]
    async fn a_json_answer_is_written_in_full() {
        let _lock = log_lock();
        let (_db, client) = client().await;
        start_capture();

        client.get("/data/1").await;

        assert!(lines(&take_capture(), "Answer")[0].contains("Le début de l'aventure."));
    }

    #[tokio::test]
    async fn a_page_is_described_rather_than_copied() {
        let _lock = log_lock();
        let (_db, client) = client().await;
        start_capture();

        client.get("/book/1").await;

        let answer = lines(&take_capture(), "Answer")[0].clone();
        assert!(answer.contains("bytes of text/html"));
        assert!(!answer.contains("<!doctype html>"));
    }

    /// From 400 up the body carries the sentence explaining it.
    #[tokio::test]
    async fn a_refusal_is_read_whatever_its_shape() {
        let _lock = log_lock();
        let (_db, client) = client().await;
        start_capture();

        client.get("/book/999").await;

        assert!(lines(&take_capture(), "Answer")[0].contains("Not Found"));
    }

    #[tokio::test]
    async fn a_posted_form_is_written() {
        let _lock = log_lock();
        let (_db, client) = fighting_client().await;
        start_capture();

        client.post_form("/game/new", &[("chosen", "yes")]).await;

        assert!(lines(&take_capture(), "Request")[0].contains(r#"payload={"chosen": "yes"}"#));
    }

    #[tokio::test]
    async fn a_request_carrying_nothing_says_so() {
        let _lock = log_lock();
        let (_db, client) = client().await;
        start_capture();

        client.get("/book/1").await;

        assert!(lines(&take_capture(), "Request")[0].contains("payload=None"));
    }

    #[tokio::test]
    async fn a_body_that_is_not_a_form_is_described() {
        let _lock = log_lock();
        let (_db, client) = client().await;
        start_capture();

        client
            .post_raw("/book/1", vec![0, 1, 2], "application/octet-stream")
            .await;

        assert!(lines(&take_capture(), "Request")[0]
            .contains("3 bytes, application/octet-stream"));
    }

    #[tokio::test]
    async fn a_json_request_is_parsed() {
        let _lock = log_lock();
        let (_db, client) = client().await;
        start_capture();

        client
            .post_json("/book/1", serde_json::json!({"page": "22"}))
            .await;

        assert!(lines(&take_capture(), "Request")[0].contains(r#"payload={"page": "22"}"#));
    }
}

/// Where a redirect sends is the whole of what it says.
mod redirects {
    use super::*;

    #[tokio::test]
    async fn a_redirect_names_its_destination() {
        let _lock = log_lock();
        let (_db, client) = hero().await;
        start_capture();

        client.post("/combat/1/assault").await;

        let answer = lines(&take_capture(), "Answer")[0].clone();
        assert!(answer.contains("destination="));
        assert!(answer.contains("/combat/1"));
    }

    #[tokio::test]
    async fn an_ordinary_answer_carries_no_destination() {
        let _lock = log_lock();
        let (_db, client) = client().await;
        start_capture();

        client.get("/book/1").await;

        assert!(!lines(&take_capture(), "Answer")[0].contains("destination="));
    }
}

/// The play-through the session cookie names.
mod the_game_id {
    use super::*;

    #[tokio::test]
    async fn a_reader_without_a_game_shows_none() {
        let _lock = log_lock();
        let (_db, client) = client().await;
        start_capture();

        client.get("/book/1").await;

        assert!(lines(&take_capture(), "Request")[0].contains("game=<absent>"));
    }

    #[tokio::test]
    async fn a_reader_with_one_is_followed_by_its_id() {
        let _lock = log_lock();
        let (db, client) = hero().await;
        let game_id = db.table("games").one().await["id"].clone();
        start_capture();

        client.get("/book/1").await;

        assert!(lines(&take_capture(), "Request")[0].contains(&format!("game='{game_id}'")));
        assert_eq!(client.session().get_id(SESSION_KEY), game_id.as_int());
    }
}

/// What a request died of.
mod failures {
    use super::*;

    use axum::response::IntoResponse;
    use axum::routing::get;
    use axum::Router;
    use haute_tension::application::errors::{refusal, AppError};

    /// A handler whose page cannot be built, which is what a broken template is.
    async fn the_templates_are_gone() -> axum::response::Response {
        refusal(
            &AppError::Render(askama::Error::Custom(
                "the templates are gone".to_string().into(),
            )),
            false,
            "",
        )
        .into_response()
    }

    #[tokio::test]
    async fn a_raising_route_is_logged_with_its_trouble() {
        let _lock = log_lock();
        let (_db, client) = client().await;
        let broken = Router::new().route("/book/{number}", get(the_templates_are_gone));
        start_capture();

        let response = tower::ServiceExt::oneshot(
            traced(broken, &client).await,
            axum::http::Request::builder()
                .uri("/book/1")
                .body(Body::empty())
                .expect("the request is built"),
        )
        .await
        .expect("the router answers");

        assert_eq!(response.status(), 500);
        let records = take_capture();
        assert!(!lines(&records, "Request failed").is_empty());
        assert!(text_of(&records).contains("500"));
    }

    /// A refusal left as an answer with a status of its own is not a failure.
    #[tokio::test]
    async fn a_handled_refusal_is_not_a_failure() {
        let _lock = log_lock();
        let (_db, client) = client().await;
        start_capture();

        client.get("/book/999").await;

        assert!(lines(&take_capture(), "Request failed").is_empty());
    }

    /// The trace, wrapped around a router of the test's own.
    async fn traced(router: Router, client: &common::TestClient) -> Router {
        router.layer(axum::middleware::from_fn_with_state(
            client.state(),
            haute_tension::application::logs::request_trace::trace_the_request,
        ))
    }
}

/// The two answers the trace describes instead of reading.
mod bodies_not_read {
    use super::*;

    /// Reading it to log it would consume it.
    #[tokio::test]
    async fn a_streamed_answer_is_left_untouched() {
        let stream = futures_lite::stream::iter(vec![
            Ok::<_, std::io::Error>(bytes::Bytes::from_static(b"a")),
            Ok(bytes::Bytes::from_static(b"b")),
        ]);
        let body = Body::from_stream(stream);

        assert!(size_is_unknown(&body));
    }

    /// A body whose length is known is read, and described.
    #[test]
    fn a_page_is_described_by_its_length() {
        let page = "<!doctype html><html></html>";

        assert_eq!(
            what_goes_out(StatusCode::OK, page, page.len()),
            Field::Str(format!("<{} bytes of text/html>", page.len()))
        );
    }

    #[test]
    fn a_streamed_answer_says_so_rather_than_being_read() {
        assert_eq!(STREAMED, "<streamed, left untouched>");
    }

    /// Whether a body's length is unknown, which is what a streamed one is.
    fn size_is_unknown(body: &Body) -> bool {
        use axum::body::HttpBody as _;
        body.size_hint().exact().is_none()
    }
}

/// How long a request has been in the server's hands.
mod timing {
    use super::*;

    #[tokio::test]
    async fn a_marked_arrival_reads_in_milliseconds() {
        let _lock = log_lock();
        let (_db, client) = client().await;
        start_capture();

        client.get("/book/1").await;

        assert!(lines(&take_capture(), "Answer")[0].contains(" ms'"));
    }
}
