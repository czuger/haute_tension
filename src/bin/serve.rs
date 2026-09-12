//! The development server.

use haute_tension::application::factory::{create_app, AppConfig};

/// The address the development server listens on.
const ADDRESS: &str = "127.0.0.1:5001";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = create_app(AppConfig::default())?;
    let listener = tokio::net::TcpListener::bind(ADDRESS).await?;
    println!("Haute Tension — http://{ADDRESS}");
    axum::serve(listener, app).await?;
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
