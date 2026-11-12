//! What the server listens on: a UNIX socket for a proxied run, a port for a
//! development one.
//!
//! The socket is how a proxy on the same machine reaches the application without
//! a port being published: nginx connects to the file, and nothing else on the
//! network can reach the server at all.

use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use tokio::net::{UnixListener, UnixStream};

/// The mode the socket is left in: anyone who can reach the directory it sits in
/// may connect.
///
/// The proxy's workers do not run as the user the application does — nginx's run
/// as `nginx`, and in a container their ids are not even this system's — so a
/// mode naming an owner or a group would refuse the one process meant to connect.
/// What guards the socket is the directory holding it, which only this user and
/// root may write.
pub const SOCKET_MODE: u32 = 0o666;

/// Bind the UNIX socket the proxy connects to.
///
/// A socket file outlives the process that made it, so one is usually already
/// there — after a crash, a kill, a machine that went down. It is removed, but
/// only once nothing answers on it: a socket that still answers belongs to a
/// server that is running, and taking it would leave two of them fighting over
/// one address.
///
/// # Arguments
///
/// * `path` - Where the socket goes. Its directory must exist: the application
///   does not make one, because on a deployed run that directory is shared with
///   the proxy and belongs to whoever set the two up.
///
/// # Errors
///
/// If a server is already answering on the socket, if the directory holding it
/// is missing, or if the socket cannot be created or left readable.
pub async fn bind_socket(path: &Path) -> io::Result<UnixListener> {
    if path.exists() {
        if UnixStream::connect(path).await.is_ok() {
            return Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                format!("{} is already being served", path.display()),
            ));
        }
        std::fs::remove_file(path)?;
    }
    let listener = UnixListener::bind(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(SOCKET_MODE))?;
    Ok(listener)
}
