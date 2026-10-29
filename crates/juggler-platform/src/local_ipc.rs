//! A local connection only this user can make (SECURITY-GUIDELINES C7,
//! ADR-0020): never a TCP or HTTP listener.
//!
//! On Unix it is a socket file:
//! - in a `0700` folder of ours (made if missing; an existing one must
//!   already be ours and closed),
//! - made `0600`,
//! - every connection's peer user checked (`getpeereid`, through tokio's
//!   `peer_cred`); anyone else is dropped unanswered,
//! - a stale endpoint removed only when `lstat` shows it is a socket we
//!   own; anything else at the path is refused and left alone, never
//!   followed.
//!
//! On Windows the same promise is a named pipe (`tokio::net::windows::named_pipe`,
//! local clients only) whose server checks each client's user before it
//! answers. Reading a pipe client's user needs Win32 calls this workspace
//! doesn't make yet, and a pipe that answers without that check would
//! trust any user of the computer, so on Windows nothing is opened:
//! [`SUPPORTED`] is `false`, [`connect`] says "not available", and the
//! listener can't be made (it needs a [`UserId`], which Windows has none
//! of yet).

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

use crate::user::UserId;

/// Whether local connections work on this system.
pub const SUPPORTED: bool = cfg!(unix);

/// The longest endpoint path, in bytes: a Unix socket's path must fit
/// `sun_path` (104 bytes on macOS, 108 on Linux, with its terminating
/// NUL); this stays well inside both.
pub const MAX_ENDPOINT_PATH: usize = 100;

/// Whether `endpoint` is short enough to be one.
#[must_use]
pub fn fits(endpoint: &Path) -> bool {
    endpoint.as_os_str().len() < MAX_ENDPOINT_PATH
}

/// Why an endpoint couldn't be served.
#[derive(Debug)]
pub enum BindError {
    /// The folder, or something already at the endpoint, belongs to
    /// someone else, is open to others, or isn't a socket: refused, and
    /// left as it was.
    NotOurs(PathBuf),
    /// The system's error, and where.
    Io(PathBuf, std::io::Error),
}

impl std::fmt::Display for BindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotOurs(path) => write!(f, "{} isn't ours", path.display()),
            Self::Io(path, error) => write!(f, "{}: {error}", path.display()),
        }
    }
}

impl std::error::Error for BindError {}

impl From<BindError> for std::io::Error {
    fn from(error: BindError) -> Self {
        match error {
            BindError::NotOurs(_) => {
                Self::new(std::io::ErrorKind::PermissionDenied, error.to_string())
            }
            BindError::Io(_, source) => source,
        }
    }
}

/// An endpoint bound for `me`, not yet served (binding needs no async
/// runtime; serving does).
#[derive(Debug)]
pub struct Bound {
    listener: imp::StdListener,
    me: UserId,
}

/// Binds `endpoint` for `me`: its folder made closed (or found ours and
/// closed), a stale endpoint of ours removed, bound, then made `me`'s
/// alone.
///
/// # Errors
///
/// [`BindError::NotOurs`] when the folder or what is at the endpoint
/// isn't ours (nothing is removed); [`BindError::Io`] when it can't be
/// made.
pub fn bind(endpoint: &Path, me: UserId) -> Result<Bound, BindError> {
    imp::bind(endpoint, me).map(|listener| Bound { listener, me })
}

impl Bound {
    /// Serves it on the current tokio runtime.
    ///
    /// # Errors
    ///
    /// When there is no runtime, or it won't take the endpoint.
    pub fn into_async(self) -> std::io::Result<Listener> {
        Ok(Listener {
            inner: imp::into_async(self.listener)?,
            me: self.me,
        })
    }
}

/// A served endpoint: answers only `me`.
#[derive(Debug)]
pub struct Listener {
    inner: imp::AsyncListener,
    me: UserId,
}

impl Listener {
    /// The next connection: `Some` when it comes from `me`, `None` when
    /// it came from anyone else (or from someone the system can't name),
    /// which is dropped unanswered.
    ///
    /// # Errors
    ///
    /// The system's error accepting it.
    pub async fn accept(&self) -> std::io::Result<Option<Stream>> {
        let (stream, peer) = imp::accept(&self.inner).await?;
        Ok(crate::user::peer_allowed(peer, self.me).then_some(Stream(stream)))
    }
}

/// One served connection, from `me`.
#[derive(Debug)]
pub struct Stream(imp::AsyncStream);

impl AsyncRead for Stream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

impl AsyncWrite for Stream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}

/// A connection made to an endpoint (blocking).
#[derive(Debug)]
pub struct Connection(imp::StdStream);

/// Connects to `endpoint`. Callers check [`endpoint_is_ours`] first when
/// what they send matters.
///
/// # Errors
///
/// The system's error (`ConnectionRefused`: nothing is listening), or
/// `Unsupported` where local connections aren't available (Windows).
pub fn connect(endpoint: &Path) -> std::io::Result<Connection> {
    imp::connect(endpoint).map(Connection)
}

impl Connection {
    /// A second handle on the same connection (one to read, one to write).
    ///
    /// # Errors
    ///
    /// The system's error.
    pub fn try_clone(&self) -> std::io::Result<Self> {
        imp::try_clone(&self.0).map(Self)
    }

    /// How long a read may wait.
    ///
    /// # Errors
    ///
    /// The system's error.
    pub fn set_read_timeout(&self, wait: Option<std::time::Duration>) -> std::io::Result<()> {
        imp::set_read_timeout(&self.0, wait)
    }

    /// How long a write may wait.
    ///
    /// # Errors
    ///
    /// The system's error.
    pub fn set_write_timeout(&self, wait: Option<std::time::Duration>) -> std::io::Result<()> {
        imp::set_write_timeout(&self.0, wait)
    }

    /// Says nothing more will be written (the other side reads to the end).
    ///
    /// # Errors
    ///
    /// The system's error.
    pub fn finish_writing(&self) -> std::io::Result<()> {
        imp::finish_writing(&self.0)
    }
}

impl Read for Connection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(buf)
    }
}

impl Write for Connection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

/// Whether `endpoint` is a live endpoint `me` made (`lstat`: a link is
/// never followed).
#[must_use]
pub fn endpoint_is_ours(endpoint: &Path, me: UserId) -> bool {
    imp::endpoint_is_ours(endpoint, me)
}

/// Removes `endpoint` when it is one of `me`'s (a stale one from before);
/// anything else is left alone.
pub fn remove_if_ours(endpoint: &Path, me: UserId) {
    if endpoint_is_ours(endpoint, me) {
        let _ = std::fs::remove_file(endpoint);
    }
}

/// Makes an endpoint another program made for us `me`'s alone: it must be
/// an endpoint (never a link) `me` owns.
///
/// # Errors
///
/// When it isn't one of `me`'s, or can't be closed.
pub fn claim(endpoint: &Path, me: UserId) -> std::io::Result<()> {
    imp::claim(endpoint, me)
}

#[cfg(unix)]
mod imp {
    use super::BindError;
    use crate::user::UserId;
    use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
    use std::path::Path;

    pub type StdListener = std::os::unix::net::UnixListener;
    pub type AsyncListener = tokio::net::UnixListener;
    pub type AsyncStream = tokio::net::UnixStream;
    pub type StdStream = std::os::unix::net::UnixStream;

    pub fn endpoint_is_ours(endpoint: &Path, me: UserId) -> bool {
        std::fs::symlink_metadata(endpoint)
            .is_ok_and(|meta| meta.file_type().is_socket() && meta.uid() == me.uid())
    }

    fn io(path: &Path) -> impl FnOnce(std::io::Error) -> BindError + '_ {
        move |error| BindError::Io(path.to_path_buf(), error)
    }

    pub fn bind(endpoint: &Path, me: UserId) -> Result<StdListener, BindError> {
        let dir = endpoint
            .parent()
            .ok_or_else(|| BindError::NotOurs(endpoint.to_path_buf()))?;
        if !crate::private_fs::make_closed_dir(dir, me).map_err(io(dir))? {
            return Err(BindError::NotOurs(dir.to_path_buf()));
        }
        match std::fs::symlink_metadata(endpoint) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io(endpoint)(error)),
            Ok(_) if endpoint_is_ours(endpoint, me) => {
                std::fs::remove_file(endpoint).map_err(io(endpoint))?;
            }
            Ok(_) => return Err(BindError::NotOurs(endpoint.to_path_buf())),
        }
        let listener = StdListener::bind(endpoint).map_err(io(endpoint))?;
        std::fs::set_permissions(endpoint, std::fs::Permissions::from_mode(0o600))
            .map_err(io(endpoint))?;
        listener.set_nonblocking(true).map_err(io(endpoint))?;
        Ok(listener)
    }

    pub fn into_async(listener: StdListener) -> std::io::Result<AsyncListener> {
        AsyncListener::from_std(listener)
    }

    pub async fn accept(
        listener: &AsyncListener,
    ) -> std::io::Result<(AsyncStream, Option<UserId>)> {
        let (stream, _) = listener.accept().await?;
        let peer = stream
            .peer_cred()
            .ok()
            .map(|cred| UserId::from_uid(cred.uid()));
        Ok((stream, peer))
    }

    pub fn connect(endpoint: &Path) -> std::io::Result<StdStream> {
        StdStream::connect(endpoint)
    }

    pub fn try_clone(stream: &StdStream) -> std::io::Result<StdStream> {
        stream.try_clone()
    }

    pub fn set_read_timeout(
        stream: &StdStream,
        wait: Option<std::time::Duration>,
    ) -> std::io::Result<()> {
        stream.set_read_timeout(wait)
    }

    pub fn set_write_timeout(
        stream: &StdStream,
        wait: Option<std::time::Duration>,
    ) -> std::io::Result<()> {
        stream.set_write_timeout(wait)
    }

    pub fn finish_writing(stream: &StdStream) -> std::io::Result<()> {
        stream.shutdown(std::net::Shutdown::Write)
    }

    pub fn claim(endpoint: &Path, me: UserId) -> std::io::Result<()> {
        if !endpoint_is_ours(endpoint, me) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("{} isn't our endpoint", endpoint.display()),
            ));
        }
        std::fs::set_permissions(endpoint, std::fs::Permissions::from_mode(0o600))
    }
}

#[cfg(not(unix))]
mod imp {
    //! No local connections here yet (see the module's note): every value
    //! that would hold one is empty, so no code path can use one.
    use super::BindError;
    use crate::Never;
    use crate::user::UserId;
    use std::io::{Read, Write};
    use std::path::Path;
    use std::pin::Pin;
    use std::task::{Context, Poll};
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

    pub type StdListener = Never;
    pub type AsyncListener = Never;

    #[derive(Debug)]
    pub struct AsyncStream(Never);
    #[derive(Debug)]
    pub struct StdStream(Never);

    impl AsyncRead for AsyncStream {
        fn poll_read(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _buf: &mut ReadBuf<'_>,
        ) -> Poll<std::io::Result<()>> {
            match self.0 {}
        }
    }

    impl AsyncWrite for AsyncStream {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            _buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            match self.0 {}
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            match self.0 {}
        }

        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            match self.0 {}
        }
    }

    impl Read for StdStream {
        fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
            match self.0 {}
        }
    }

    impl Write for StdStream {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            match self.0 {}
        }

        fn flush(&mut self) -> std::io::Result<()> {
            match self.0 {}
        }
    }

    pub fn endpoint_is_ours(_endpoint: &Path, me: UserId) -> bool {
        me.unreachable()
    }

    pub fn bind(_endpoint: &Path, me: UserId) -> Result<StdListener, BindError> {
        me.unreachable()
    }

    pub fn into_async(listener: StdListener) -> std::io::Result<AsyncListener> {
        match listener {}
    }

    pub async fn accept(
        listener: &AsyncListener,
    ) -> std::io::Result<(AsyncStream, Option<UserId>)> {
        match *listener {}
    }

    pub fn connect(_endpoint: &Path) -> std::io::Result<StdStream> {
        Err(crate::unsupported("a local connection only you can make"))
    }

    pub fn try_clone(stream: &StdStream) -> std::io::Result<StdStream> {
        match stream.0 {}
    }

    pub fn set_read_timeout(
        stream: &StdStream,
        _wait: Option<std::time::Duration>,
    ) -> std::io::Result<()> {
        match stream.0 {}
    }

    pub fn set_write_timeout(
        stream: &StdStream,
        _wait: Option<std::time::Duration>,
    ) -> std::io::Result<()> {
        match stream.0 {}
    }

    pub fn finish_writing(stream: &StdStream) -> std::io::Result<()> {
        match stream.0 {}
    }

    pub fn claim(_endpoint: &Path, me: UserId) -> std::io::Result<()> {
        me.unreachable()
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::{BindError, bind, claim, connect, endpoint_is_ours, remove_if_ours};
    use crate::user::{UserId, current};
    use std::io::{Read, Write};
    use std::os::unix::fs::PermissionsExt;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    /// A short base folder (a socket path must stay under ~100 bytes).
    fn base() -> tempfile::TempDir {
        tempfile::Builder::new()
            .prefix("aij-ipc")
            .tempdir_in("/tmp")
            .unwrap()
    }

    fn mode(path: &std::path::Path) -> u32 {
        std::fs::symlink_metadata(path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777
    }

    #[tokio::test]
    async fn this_user_is_answered_over_a_0600_endpoint_in_a_0700_folder() {
        let root = base();
        let endpoint = root.path().join("d").join("s.sock");
        let me = current().unwrap();
        let listener = bind(&endpoint, me).unwrap().into_async().unwrap();
        assert_eq!(mode(endpoint.parent().unwrap()), 0o700);
        assert_eq!(mode(&endpoint), 0o600);
        assert!(endpoint_is_ours(&endpoint, me));
        let client = std::thread::spawn({
            let endpoint = endpoint.clone();
            move || {
                let mut connection = connect(&endpoint).unwrap();
                connection.write_all(b"hi\n").unwrap();
                connection.finish_writing().unwrap();
                let mut answer = String::new();
                connection.read_to_string(&mut answer).unwrap();
                answer
            }
        });
        let mut stream = listener.accept().await.unwrap().unwrap();
        let mut asked = String::new();
        stream.read_to_string(&mut asked).await.unwrap();
        assert_eq!(asked, "hi\n");
        stream.write_all(b"ok\n").await.unwrap();
        stream.shutdown().await.unwrap();
        drop(stream);
        assert_eq!(client.join().unwrap(), "ok\n");
    }

    #[test]
    fn a_stale_endpoint_of_ours_goes_and_anything_else_stays() {
        let root = base();
        let dir = root.path().join("d");
        let endpoint = dir.join("s.sock");
        let me = current().unwrap();
        drop(bind(&endpoint, me).unwrap());
        assert!(endpoint.exists(), "a crashed run's endpoint");
        drop(bind(&endpoint, me).unwrap());
        // A plain file, or a link, is never removed or followed.
        std::fs::remove_file(&endpoint).unwrap();
        std::fs::write(&endpoint, "keep me").unwrap();
        assert!(matches!(bind(&endpoint, me), Err(BindError::NotOurs(_))));
        remove_if_ours(&endpoint, me);
        assert_eq!(std::fs::read_to_string(&endpoint).unwrap(), "keep me");
        assert!(claim(&endpoint, me).is_err(), "not an endpoint");
    }

    #[test]
    fn someone_elses_or_an_open_folder_is_refused_and_left_as_it_was() {
        let root = base();
        let endpoint = root.path().join("d").join("s.sock");
        let me = current().unwrap();
        drop(bind(&endpoint, me).unwrap());
        let stranger = UserId::from_uid(me.uid().wrapping_add(1));
        assert!(matches!(
            bind(&endpoint, stranger),
            Err(BindError::NotOurs(_))
        ));
        assert!(!endpoint_is_ours(&endpoint, stranger));
        remove_if_ours(&endpoint, stranger);
        assert!(
            endpoint.exists(),
            "someone else's endpoint is never removed"
        );
        assert!(claim(&endpoint, stranger).is_err());
        claim(&endpoint, me).unwrap();

        let open = root.path().join("open");
        std::fs::create_dir(&open).unwrap();
        std::fs::set_permissions(&open, std::fs::Permissions::from_mode(0o777)).unwrap();
        assert!(matches!(
            bind(&open.join("s.sock"), me),
            Err(BindError::NotOurs(_))
        ));
        assert_eq!(mode(&open), 0o777, "never chmod'ed into shape");
    }

    #[test]
    fn nothing_listening_is_refused() {
        let root = base();
        let error = connect(&root.path().join("none.sock")).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    }
}
