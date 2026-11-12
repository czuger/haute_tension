//! The server: on a UNIX socket when a proxy is in front of it, on a port
//! otherwise.

use haute_tension::application::factory::{create_app, AppConfig};
use haute_tension::application::listener::bind_socket;
use haute_tension::core::config::listen_socket;
use haute_tension::core::logs::general_log::event;
use haute_tension::vars;

/// The address a development run listens on, and nothing but a development run:
/// it is for a reader at this keyboard, not for the network.
const ADDRESS: &str = "127.0.0.1:5001";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_app(AppConfig::default())?;

    // A deployed run is reached through its socket, so there is no port to bind
    // and no port to leave open; `LISTEN_SOCKET` is what says which run this is.
    match listen_socket() {
        Some(path) => {
            let listener = bind_socket(&path).await?;
            let socket = path.display().to_string();
            event("Server listening", vars! { "socket" => socket.clone() });
            println!("Haute Tension — unix:{socket}");
            axum::serve(listener, app).await?;
        }
        None => {
            let listener = tokio::net::TcpListener::bind(ADDRESS).await?;
            event("Server listening", vars! { "address" => ADDRESS });
            println!("Haute Tension — http://{ADDRESS}");
            axum::serve(listener, app).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::ADDRESS;

    /// The development server is for a reader at the keyboard, and for nobody
    /// else on the network.
    #[test]
    fn it_serves_on_localhost_only() {
        assert_eq!(ADDRESS, "127.0.0.1:5001");
    }
}
