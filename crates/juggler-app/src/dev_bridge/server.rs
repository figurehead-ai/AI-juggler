//! The bridge's HTTP side: admission (rate, `Host`, `Origin`, token,
//! route, allowlist — in that order), the command call, and the SSE
//! event stream. One HTTP/1.1 connection per task, bounded.

use std::convert::Infallible;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use bytes::Bytes;
use http_body_util::{BodyExt as _, Full, Limited};
use hyper::body::{Frame, Incoming};
use hyper::header::{self, HeaderMap, HeaderValue};
use hyper::{Method, Request, Response, StatusCode};
use tauri::Listener as _;
use tokio::sync::{Semaphore, broadcast, mpsc};

use super::routes::{self, Reply};
use super::{Config, FORWARDED_EVENTS};

/// The largest request body (command arguments).
pub(super) const MAX_BODY_BYTES: usize = 64 * 1024;
/// hyper's read buffer, which bounds the request head.
const MAX_HEAD_BYTES: usize = 16 * 1024;
/// Open connections at once; more are closed on accept.
const MAX_CONNECTIONS: usize = 32;
/// Open event streams at once.
const MAX_STREAMS: usize = 4;
/// A request head must arrive within this.
const HEAD_TIMEOUT: Duration = Duration::from_secs(5);
/// An SSE comment this often keeps an idle stream open.
const KEEPALIVE: Duration = Duration::from_secs(15);
/// The rate limit: a bucket of this many requests…
const BURST: f64 = 200.0;
/// …refilled at this many a second. A screen's first load is a few dozen
/// queries; a loop hammering the backend is not.
const PER_SECOND: f64 = 50.0;
/// Command names are `snake_case` identifiers.
const MAX_NAME_LEN: usize = 64;

/// The message a refused write shows, in the UI and in the body.
pub const READ_ONLY_MESSAGE: &str = "Read-only test bridge — not available";

type Body = http_body_util::combinators::UnsyncBoxBody<Bytes, Infallible>;

/// What a request asks for, once admitted.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Route {
    /// A CORS preflight.
    Preflight,
    /// The list of commands the bridge serves.
    Commands,
    /// One command call.
    Invoke(&'static routes::ReadRoute),
    /// The SSE stream of refresh events.
    Events,
}

/// Why a request was turned away.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Rejection {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: &'static str,
}

const fn reject(status: StatusCode, code: &'static str, message: &'static str) -> Rejection {
    Rejection {
        status,
        code,
        message,
    }
}

/// A token bucket for the whole bridge (one client: the page).
#[derive(Debug)]
pub(super) struct RateLimit {
    bucket: Mutex<(f64, Instant)>,
}

impl RateLimit {
    pub(super) fn new() -> Self {
        Self {
            bucket: Mutex::new((BURST, Instant::now())),
        }
    }

    /// Takes one request from the bucket, if there is one.
    pub(super) fn take(&self) -> bool {
        let Ok(mut bucket) = self.bucket.lock() else {
            return false;
        };
        let now = Instant::now();
        let refill = now.duration_since(bucket.1).as_secs_f64() * PER_SECOND;
        *bucket = ((bucket.0 + refill).min(BURST), now);
        if bucket.0 >= 1.0 {
            bucket.0 -= 1.0;
            true
        } else {
            false
        }
    }
}

/// Decides whether a request may go on, and to what. The order matters:
/// a request from a page that isn't ours learns nothing, and one without
/// the token can't tell which routes exist.
pub(super) fn admit(
    method: &Method,
    path: &str,
    headers: &HeaderMap,
    config: &Config,
    rate: &RateLimit,
) -> Result<Route, Rejection> {
    if !rate.take() {
        return Err(reject(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "Too many requests to the test bridge — slow down.",
        ));
    }
    // DNS rebinding: a page on another name that resolves here still
    // sends its own name as Host.
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let port = config.port;
    if host != format!("127.0.0.1:{port}") && host != format!("localhost:{port}") {
        return Err(reject(StatusCode::FORBIDDEN, "bad_host", "Wrong Host."));
    }
    if let Some(origin) = headers.get(header::ORIGIN)
        && origin.as_bytes() != config.origin.as_bytes()
    {
        return Err(reject(StatusCode::FORBIDDEN, "bad_origin", "Wrong Origin."));
    }
    if method == Method::OPTIONS {
        // A preflight carries no token by design, and gets nothing but
        // the CORS answer — for the allowed origin only.
        return if headers.contains_key(header::ORIGIN) {
            Ok(Route::Preflight)
        } else {
            Err(reject(StatusCode::FORBIDDEN, "bad_origin", "Wrong Origin."))
        };
    }
    let presented = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.as_bytes().strip_prefix(b"Bearer "))
        .unwrap_or_default();
    if !config.token.matches(presented) {
        return Err(reject(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Missing or wrong token.",
        ));
    }
    match (method, path) {
        (&Method::GET, "/v1/commands") => Ok(Route::Commands),
        (&Method::GET, "/v1/events") => Ok(Route::Events),
        (&Method::POST, _) if path.starts_with("/v1/invoke/") => {
            let name = &path["/v1/invoke/".len()..];
            let well_formed = !name.is_empty()
                && name.len() <= MAX_NAME_LEN
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_');
            // Unknown and write commands look the same from here.
            match routes::find(name) {
                Some(route) if well_formed => Ok(Route::Invoke(route)),
                _ => Err(reject(
                    StatusCode::FORBIDDEN,
                    "read_only_bridge",
                    READ_ONLY_MESSAGE,
                )),
            }
        }
        _ => Err(reject(StatusCode::NOT_FOUND, "not_found", "No such route.")),
    }
}

/// The parts of a running bridge every connection shares.
struct Shared<R: tauri::Runtime> {
    app: tauri::AppHandle<R>,
    config: Arc<Config>,
    rate: RateLimit,
    events: broadcast::Sender<Bytes>,
    streams: Arc<Semaphore>,
}

/// Starts serving `listener` (already bound to 127.0.0.1) on the app's
/// async runtime, and starts forwarding the refresh events.
///
/// # Errors
///
/// The listener can't be made non-blocking.
pub(super) fn spawn<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    listener: std::net::TcpListener,
    config: Arc<Config>,
) -> Result<(), String> {
    listener
        .set_nonblocking(true)
        .map_err(|error| error.to_string())?;
    let (events, _) = broadcast::channel(64);
    for name in FORWARDED_EVENTS {
        let sender = events.clone();
        app.listen_any(*name, move |event| {
            // No subscriber is fine: nobody is watching yet.
            let _ = sender.send(sse_frame(name, event.payload()));
        });
    }
    let shared = Arc::new(Shared {
        app,
        config,
        rate: RateLimit::new(),
        events,
        streams: Arc::new(Semaphore::new(MAX_STREAMS)),
    });
    tauri::async_runtime::spawn(serve(listener, shared));
    Ok(())
}

async fn serve<R: tauri::Runtime>(listener: std::net::TcpListener, shared: Arc<Shared<R>>) {
    let listener = match tokio::net::TcpListener::from_std(listener) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("ai-juggler: dev bridge stopped: {error}");
            return;
        }
    };
    let connections = Arc::new(Semaphore::new(MAX_CONNECTIONS));
    loop {
        let Ok((stream, peer)) = listener.accept().await else {
            // Out of descriptors and the like: back off, don't spin.
            tokio::time::sleep(Duration::from_millis(100)).await;
            continue;
        };
        // Bound to 127.0.0.1, so always loopback; checked all the same.
        if !peer.ip().is_loopback() {
            continue;
        }
        let Ok(permit) = Arc::clone(&connections).try_acquire_owned() else {
            continue; // dropping the stream closes it
        };
        let shared = Arc::clone(&shared);
        tokio::spawn(async move {
            let service = hyper::service::service_fn(move |request| {
                let shared = Arc::clone(&shared);
                async move { Ok::<_, Infallible>(handle(request, &shared).await) }
            });
            let _ = hyper::server::conn::http1::Builder::new()
                .timer(hyper_util::rt::TokioTimer::new())
                .header_read_timeout(HEAD_TIMEOUT)
                .max_buf_size(MAX_HEAD_BYTES)
                .serve_connection(hyper_util::rt::TokioIo::new(stream), service)
                .await;
            drop(permit);
        });
    }
}

async fn handle<R: tauri::Runtime>(
    request: Request<Incoming>,
    shared: &Shared<R>,
) -> Response<Body> {
    let (parts, body) = request.into_parts();
    let admitted = admit(
        &parts.method,
        parts.uri.path(),
        &parts.headers,
        &shared.config,
        &shared.rate,
    );
    // Audit: what and the outcome, never arguments or results.
    let what = match &admitted {
        Ok(Route::Invoke(route)) => route.ipc,
        Ok(Route::Commands) => "commands",
        Ok(Route::Events) => "events",
        Ok(Route::Preflight) => "preflight",
        Err(rejection) => rejection.code,
    };
    let response = match admitted {
        Err(rejection) => error_response(rejection.status, rejection.code, rejection.message),
        Ok(Route::Preflight) => preflight(),
        Ok(Route::Commands) => {
            let names: Vec<&str> = routes::READ_ROUTES.iter().map(|route| route.ipc).collect();
            json_response(
                StatusCode::OK,
                serde_json::to_vec(&names).unwrap_or_default(),
            )
        }
        Ok(Route::Events) => events(shared),
        Ok(Route::Invoke(route)) => invoke(shared, route, &parts.headers, body).await,
    };
    eprintln!(
        "ai-juggler: dev bridge {} {what} -> {}",
        parts.method,
        response.status().as_u16()
    );
    finish(response, &parts.headers, &shared.config)
}

async fn invoke<R: tauri::Runtime>(
    shared: &Shared<R>,
    route: &'static routes::ReadRoute,
    headers: &HeaderMap,
    body: Incoming,
) -> Response<Body> {
    let declared = headers
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<usize>().ok());
    if declared.is_some_and(|length| length > MAX_BODY_BYTES) {
        return too_large();
    }
    let Ok(collected) = Limited::new(body, MAX_BODY_BYTES).collect().await else {
        return too_large();
    };
    let bytes = collected.to_bytes();
    let args = if bytes.is_empty() {
        serde_json::Value::Object(serde_json::Map::new())
    } else {
        match serde_json::from_slice::<serde_json::Value>(&bytes) {
            Ok(value @ serde_json::Value::Object(_)) => value,
            _ => {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    "bad_arguments",
                    "The arguments must be a JSON object.",
                );
            }
        }
    };
    let Reply { status, body } = routes::dispatch(&shared.app, route.ipc, args)
        .await
        .unwrap_or_else(Reply::internal);
    json_response(status, body)
}

fn events<R: tauri::Runtime>(shared: &Shared<R>) -> Response<Body> {
    let Ok(permit) = Arc::clone(&shared.streams).try_acquire_owned() else {
        return error_response(
            StatusCode::TOO_MANY_REQUESTS,
            "too_many_streams",
            "Too many event streams open.",
        );
    };
    let mut events = shared.events.subscribe();
    let (sender, receiver) = mpsc::channel::<Bytes>(16);
    tokio::spawn(async move {
        let _permit = permit;
        if sender
            .send(Bytes::from_static(b": open\n\n"))
            .await
            .is_err()
        {
            return;
        }
        loop {
            let frame = match tokio::time::timeout(KEEPALIVE, events.recv()).await {
                Ok(Ok(frame)) => frame,
                // A slow page misses ticks; the next one carries the state.
                Ok(Err(broadcast::error::RecvError::Lagged(_))) => continue,
                Ok(Err(broadcast::error::RecvError::Closed)) => return,
                Err(_) => Bytes::from_static(b": keepalive\n\n"),
            };
            // The page went away: stop.
            if sender.send(frame).await.is_err() {
                return;
            }
        }
    });
    let mut response = Response::new(SseBody(receiver).boxed_unsync());
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/event-stream"),
    );
    response
}

/// One SSE frame; a payload's line breaks become `data:` lines.
pub(super) fn sse_frame(name: &str, payload: &str) -> Bytes {
    let mut frame = format!("event: {name}\n");
    for line in payload.split('\n') {
        frame.push_str("data: ");
        frame.push_str(line.trim_end_matches('\r'));
        frame.push('\n');
    }
    frame.push('\n');
    Bytes::from(frame)
}

/// An SSE response body fed by the stream's task.
struct SseBody(mpsc::Receiver<Bytes>);

impl http_body::Body for SseBody {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Infallible>>> {
        self.0
            .poll_recv(cx)
            .map(|next| next.map(|bytes| Ok(Frame::data(bytes))))
    }
}

fn full(body: Vec<u8>) -> Body {
    Full::new(Bytes::from(body)).boxed_unsync()
}

fn json_response(status: StatusCode, body: Vec<u8>) -> Response<Body> {
    let mut response = Response::new(full(body));
    *response.status_mut() = status;
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/json"),
    );
    response
}

/// The UI's error shape (`ErrorWire`): `code`, `retryable`, `user_message`.
fn error_response(status: StatusCode, code: &str, message: &str) -> Response<Body> {
    let body = serde_json::json!({
        "code": code,
        "retryable": status == StatusCode::TOO_MANY_REQUESTS,
        "user_message": message,
    });
    json_response(status, body.to_string().into_bytes())
}

fn too_large() -> Response<Body> {
    error_response(
        StatusCode::PAYLOAD_TOO_LARGE,
        "too_large",
        "The request is too large.",
    )
}

fn preflight() -> Response<Body> {
    let mut response = Response::new(full(Vec::new()));
    *response.status_mut() = StatusCode::NO_CONTENT;
    let headers = response.headers_mut();
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("authorization, content-type"),
    );
    headers.insert(
        header::ACCESS_CONTROL_MAX_AGE,
        HeaderValue::from_static("600"),
    );
    response
}

/// Headers on every answer; CORS only for the allowed origin.
fn finish(mut response: Response<Body>, request: &HeaderMap, config: &Config) -> Response<Body> {
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::VARY, HeaderValue::from_static("Origin"));
    if let Some(origin) = request.get(header::ORIGIN)
        && origin.as_bytes() == config.origin.as_bytes()
    {
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, origin.clone());
    }
    response
}
