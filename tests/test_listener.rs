//! The socket the proxy connects to: what binding it costs, and what it refuses.

use std::os::unix::fs::PermissionsExt;

use haute_tension::application::listener::{bind_socket, SOCKET_MODE};
use tokio::net::UnixStream;

/// A socket path of this test's own, in a directory that goes away with it.
struct Where(tempfile::TempDir);

impl Where {
    fn new() -> Self {
        Self(tempfile::tempdir().expect("a temporary directory is always made"))
    }

    fn socket(&self) -> std::path::PathBuf {
        self.0.path().join("serve.sock")
    }
}

mod binding {
    use super::*;

    #[tokio::test]
    async fn the_socket_is_made_and_answers() {
        let place = Where::new();

        let listener = bind_socket(&place.socket()).await.expect("nothing is there");

        assert!(place.socket().exists());
        assert!(UnixStream::connect(place.socket()).await.is_ok());
        drop(listener);
    }

    /// nginx's workers are another user, and in a container not even a user of
    /// this system, so the mode cannot name one.
    #[tokio::test]
    async fn anyone_who_reaches_it_may_connect() {
        let place = Where::new();

        let _listener = bind_socket(&place.socket()).await.expect("nothing is there");

        let mode = std::fs::metadata(place.socket())
            .expect("the socket was made")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, SOCKET_MODE);
    }

    /// What a crash or a kill leaves behind: the file, with nobody listening.
    #[tokio::test]
    async fn a_socket_nobody_answers_on_is_replaced() {
        let place = Where::new();
        let first = bind_socket(&place.socket()).await.expect("nothing is there");
        drop(first);

        let _second = bind_socket(&place.socket())
            .await
            .expect("the stale socket is this run's to take");

        assert!(UnixStream::connect(place.socket()).await.is_ok());
    }

    #[tokio::test]
    async fn a_socket_that_still_answers_is_left_alone() {
        let place = Where::new();
        let _serving = bind_socket(&place.socket()).await.expect("nothing is there");

        let trouble = bind_socket(&place.socket())
            .await
            .expect_err("one socket is one server");

        assert_eq!(trouble.kind(), std::io::ErrorKind::AddrInUse);
        assert!(trouble.to_string().contains("already being served"));
        // The first server is still the one listening.
        assert!(UnixStream::connect(place.socket()).await.is_ok());
    }

    /// The directory is shared with the proxy on a deployed run, so it belongs to
    /// whoever set the two up and the application does not make one.
    #[tokio::test]
    async fn a_missing_directory_is_a_failure() {
        let place = Where::new();
        let path = place.0.path().join("nowhere").join("serve.sock");

        let trouble = bind_socket(&path).await.expect_err("the directory is missing");

        assert_eq!(trouble.kind(), std::io::ErrorKind::NotFound);
    }
}
