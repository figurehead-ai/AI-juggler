//! A plain HTTP/1.1 `GET` to a tool's own server on this machine
//! (127.0.0.1 only), for the adapters that ask one (D-106: Kimi Code's
//! `kimi web`). No dependency: a request line, a few headers, one answer
//! read to its end within a deadline and a size cap.
//!
//! What is sent is only what the caller names: the path, `Host`
//! (`127.0.0.1:<port>`, the loopback name the tool's DNS-rebinding check
//! allows), `Accept`, `Connection: close` and, when given, one bearer
//! credential. No `Origin`, no cookie, no user agent. The credential is
//! written to the socket and nowhere else: never logged, never in an
//! error (errors name the step and the status only).

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

/// The largest answer read (a usage summary is well under 4 KB).
const MAX_BODY: usize = 256 * 1024;

/// An answer: its status and its body.
#[derive(Debug)]
pub struct Answer {
    /// The HTTP status.
    pub status: u16,
    /// The body, its chunked framing removed.
    pub body: Vec<u8>,
}

/// `GET path` from `127.0.0.1:port`, with `bearer` as its credential when
/// given, answered within `timeout`.
///
/// # Errors
///
/// In plain words, naming the step (connect, send, read, answer) and never
/// the credential.
pub fn get(
    port: u16,
    path: &str,
    bearer: Option<&str>,
    timeout: Duration,
) -> Result<Answer, String> {
    if port == 0 || !path.starts_with('/') || path.contains(['\r', '\n', ' ']) {
        return Err("not a request to a local server".into());
    }
    if bearer.is_some_and(|token| token.contains(['\r', '\n'])) {
        return Err("the credential isn't one line".into());
    }
    let deadline = Instant::now() + timeout;
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let mut stream = TcpStream::connect_timeout(&address, timeout)
        .map_err(|error| format!("couldn't connect to the local server: {}", error.kind()))?;
    let left = || {
        deadline
            .saturating_duration_since(Instant::now())
            .max(Duration::from_millis(1))
    };
    stream
        .set_write_timeout(Some(left()))
        .map_err(|_| "couldn't send the request".to_owned())?;
    let mut request = format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAccept: application/json\r\nConnection: close\r\n"
    );
    if let Some(token) = bearer {
        request.push_str("Authorization: Bearer ");
        request.push_str(token);
        request.push_str("\r\n");
    }
    request.push_str("\r\n");
    let sent = stream.write_all(request.as_bytes());
    // The request held the credential: gone before anything else happens.
    drop(request);
    sent.map_err(|_| "couldn't send the request".to_owned())?;
    let mut raw = Vec::new();
    let mut buffer = [0_u8; 8192];
    loop {
        if Instant::now() >= deadline {
            return Err("the local server didn't answer in time".into());
        }
        stream
            .set_read_timeout(Some(left()))
            .map_err(|_| "couldn't read the answer".to_owned())?;
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                raw.extend_from_slice(&buffer[..read]);
                if raw.len() > MAX_BODY {
                    return Err("the local server's answer is too long".into());
                }
                if complete(&raw) {
                    break;
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                return Err("the local server didn't answer in time".into());
            }
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
            Err(_) => return Err("couldn't read the answer".into()),
        }
    }
    parse(&raw).ok_or_else(|| "the local server's answer isn't HTTP".into())
}

/// Where the head ends, and its lines (lower-cased names).
fn head(raw: &[u8]) -> Option<(usize, Vec<String>)> {
    let end = raw.windows(4).position(|window| window == b"\r\n\r\n")? + 4;
    let text = std::str::from_utf8(&raw[..end]).ok()?;
    Some((end, text.lines().map(str::to_owned).collect()))
}

/// A header's value, by its name in lower case.
fn header<'a>(lines: &'a [String], name: &str) -> Option<&'a str> {
    lines.iter().skip(1).find_map(|line| {
        let (key, value) = line.split_once(':')?;
        key.trim()
            .eq_ignore_ascii_case(name)
            .then_some(value.trim())
    })
}

/// Whether `raw` holds the whole answer: its length is known and read,
/// or its last chunk is in.
fn complete(raw: &[u8]) -> bool {
    let Some((end, lines)) = head(raw) else {
        return false;
    };
    if let Some(length) = header(&lines, "content-length").and_then(|v| v.parse::<usize>().ok()) {
        return raw.len() >= end + length;
    }
    header(&lines, "transfer-encoding").is_some_and(|v| v.eq_ignore_ascii_case("chunked"))
        && raw[end..].ends_with(b"0\r\n\r\n")
}

/// The status and body of a whole answer.
fn parse(raw: &[u8]) -> Option<Answer> {
    let (end, lines) = head(raw)?;
    let mut status_line = lines.first()?.split_whitespace();
    if !status_line.next()?.starts_with("HTTP/1.") {
        return None;
    }
    let status = status_line.next()?.parse::<u16>().ok()?;
    let rest = &raw[end..];
    let body =
        if header(&lines, "transfer-encoding").is_some_and(|v| v.eq_ignore_ascii_case("chunked")) {
            dechunk(rest)?
        } else if let Some(length) =
            header(&lines, "content-length").and_then(|v| v.parse::<usize>().ok())
        {
            rest.get(..length)?.to_vec()
        } else {
            rest.to_vec()
        };
    Some(Answer { status, body })
}

/// A chunked body joined.
fn dechunk(mut rest: &[u8]) -> Option<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        let line_end = rest.windows(2).position(|window| window == b"\r\n")?;
        let size_text = std::str::from_utf8(&rest[..line_end]).ok()?;
        let size = usize::from_str_radix(size_text.split(';').next()?.trim(), 16).ok()?;
        rest = &rest[line_end + 2..];
        if size == 0 {
            return Some(body);
        }
        body.extend_from_slice(rest.get(..size)?);
        rest = rest.get(size + 2..)?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    /// One answer from a listener on 127.0.0.1, and the request it got.
    fn serve(answer: &'static [u8]) -> (u16, std::thread::JoinHandle<Vec<u8>>) {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let handle = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut raw = Vec::new();
            let mut buffer = [0_u8; 1024];
            while !raw.windows(4).any(|w| w == b"\r\n\r\n") {
                let read = stream.read(&mut buffer).unwrap();
                raw.extend_from_slice(&buffer[..read]);
            }
            stream.write_all(answer).unwrap();
            raw
        });
        (port, handle)
    }

    #[test]
    fn a_get_sends_only_what_it_names_and_reads_either_framing() {
        let (port, server) = serve(b"HTTP/1.1 200 OK\r\nContent-Length: 11\r\n\r\n{\"code\":0}\n");
        let answer = get(port, "/api/v1/healthz", None, Duration::from_secs(5)).unwrap();
        assert_eq!(
            (answer.status, answer.body.as_slice()),
            (200, &b"{\"code\":0}\n"[..])
        );
        let request = String::from_utf8(server.join().unwrap()).unwrap();
        assert!(request.starts_with("GET /api/v1/healthz HTTP/1.1\r\n"));
        assert!(request.contains(&format!("Host: 127.0.0.1:{port}\r\n")));
        assert!(!request.to_ascii_lowercase().contains("origin"));
        assert!(!request.contains("Authorization"));

        let (port, server) = serve(
            b"HTTP/1.1 401 Unauthorized\r\nTransfer-Encoding: chunked\r\n\r\n4\r\n{\"a\"\r\n3\r\n:1}\r\n0\r\n\r\n",
        );
        let answer = get(port, "/x", Some("t0ken"), Duration::from_secs(5)).unwrap();
        assert_eq!(
            (answer.status, answer.body.as_slice()),
            (401, &b"{\"a\":1}"[..])
        );
        let request = String::from_utf8(server.join().unwrap()).unwrap();
        assert!(request.contains("Authorization: Bearer t0ken\r\n"));
    }

    #[test]
    fn a_refused_or_silent_server_and_bad_input_say_why_without_the_credential() {
        let unused = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = unused.local_addr().unwrap().port();
        drop(unused);
        let error = get(port, "/x", Some("secret-xyz"), Duration::from_secs(2)).unwrap_err();
        assert!(error.starts_with("couldn't connect"), "{error}");
        assert!(!error.contains("secret-xyz"));

        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let hold = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            std::thread::sleep(Duration::from_millis(600));
            drop(stream);
        });
        let error = get(port, "/x", Some("secret-xyz"), Duration::from_millis(200)).unwrap_err();
        assert!(error.contains("in time"), "{error}");
        hold.join().unwrap();

        assert!(get(0, "/x", None, Duration::from_secs(1)).is_err());
        assert!(get(1, "x", None, Duration::from_secs(1)).is_err());
        assert!(get(1, "/x", Some("a\r\nb"), Duration::from_secs(1)).is_err());
    }
}
