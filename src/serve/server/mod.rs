// Changed from oxhttp 0.3.3 for Statac: the client, TLS, content decoding and request-body reading are removed; an `on_fault` hook hands connection faults to the owner, and requests sent together on one connection are each answered.
//! An HTTP/1.1 server on plain threads: the server of oxhttp 0.3.3,
//! https://github.com/oxigraph/oxhttp, which is offered under MIT or
//! Apache-2.0, less its client, its TLS, its content decoding and the
//! reading of a request's body. It says nothing of its own: a connection
//! it can't take or answer is handed to its owner to tell.

mod io;
pub(super) mod model;
mod utils;

use io::{BUFFER_CAPACITY, decode_request_headers, encode_response, skip_request_body};
use model::header::{CONNECTION, CONTENT_TYPE, EXPECT};
use model::request::Builder as RequestBuilder;
use model::{Body, Request, Response, StatusCode, Version};
use std::io::{BufRead, BufReader, BufWriter, Error, ErrorKind, Result, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::Builder as ThreadBuilder;
use std::time::Duration;

type OnRequest = dyn Fn(&Request<()>) -> Response<Body> + Send + Sync + 'static;
type OnFault = dyn Fn(&Error) + Send + Sync + 'static;

/// An HTTP server.
///
/// It uses a very simple threading mechanism: a new thread is started on each connection and closed when the client connection is closed.
/// To avoid crashes it is possible to set an upper bound to the number of concurrent connections using the [`Server::with_max_concurrent_connections`] function.
pub(super) struct Server {
    on_request: Arc<OnRequest>,
    on_fault: Arc<OnFault>,
    socket_addrs: Vec<SocketAddr>,
    timeout: Option<Duration>,
    max_num_thread: Option<usize>,
}

impl Server {
    /// Builds the server using the given `on_request` method that builds a `Response` from a given `Request`.
    ///
    /// `on_fault` is handed what kept a connection from being taken or
    /// answered: the first of each run of such faults, since one that
    /// lasts recurs with every connection.
    #[inline]
    pub(super) fn new(
        on_request: impl Fn(&Request<()>) -> Response<Body> + Send + Sync + 'static,
        on_fault: impl Fn(&Error) + Send + Sync + 'static,
    ) -> Self {
        Self {
            on_request: Arc::new(on_request),
            on_fault: Arc::new(on_fault),
            socket_addrs: Vec::new(),
            timeout: None,
            max_num_thread: None,
        }
    }

    /// Ask the server to listen to a given socket when spawned.
    pub(super) fn bind(mut self, addr: impl Into<SocketAddr>) -> Self {
        let addr = addr.into();
        if !self.socket_addrs.contains(&addr) {
            self.socket_addrs.push(addr);
        }
        self
    }

    /// Sets the global timeout value (applies to both read and write).
    #[inline]
    pub(super) fn with_global_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the number maximum number of threads this server can spawn.
    #[inline]
    pub(super) fn with_max_concurrent_connections(mut self, max_num_thread: usize) -> Self {
        self.max_num_thread = Some(max_num_thread);
        self
    }

    /// Spawns the server by listening to the given addresses.
    ///
    /// Note that this is not blocking: the listening threads go on until the process ends.
    pub(super) fn spawn(self) -> Result<()> {
        let timeout = self.timeout;
        let thread_limit = self.max_num_thread.map(Semaphore::new);
        let faults = Faults {
            on_fault: self.on_fault,
            failing: Arc::new(AtomicBool::new(false)),
        };
        for listener_addr in self.socket_addrs {
            let listener = TcpListener::bind(listener_addr)?;
            let thread_name = format!("{listener_addr}: listener thread of the server");
            let thread_limit = thread_limit.clone();
            let on_request = Arc::clone(&self.on_request);
            let faults = faults.clone();
            ThreadBuilder::new().name(thread_name).spawn(move || {
                for stream in listener.incoming() {
                    let stream = match stream {
                        Ok(stream) => stream,
                        // Given up by the client before it was taken.
                        Err(error) if error.kind() == ErrorKind::ConnectionAborted => continue,
                        Err(error) => {
                            faults.fault(&error);
                            // A fault that lasts, such as no file handles
                            // left, would otherwise turn this loop at full
                            // speed.
                            std::thread::sleep(Duration::from_millis(100));
                            continue;
                        }
                    };
                    let _ = stream.set_nodelay(true);
                    let thread_name = match stream.peer_addr() {
                        Ok(peer_addr) => format!("{peer_addr}: responding thread of the server"),
                        Err(_) => "responding thread of the server".to_owned(),
                    };
                    let thread_guard = thread_limit.as_ref().map(|s| s.lock());
                    let on_request = Arc::clone(&on_request);
                    let spawned = ThreadBuilder::new().name(thread_name).spawn(move || {
                        // A failure here is nearly always the browser
                        // closing the connection before its answer.
                        let _ = accept_request(stream, &*on_request, timeout);
                        drop(thread_guard);
                    });
                    match spawned {
                        Ok(_) => faults.clear(),
                        Err(error) => faults.fault(&error),
                    }
                }
            })?;
        }
        Ok(())
    }
}

/// Where faults go, each run of them told once at its first, whichever
/// listener meets it.
#[derive(Clone)]
struct Faults {
    on_fault: Arc<OnFault>,
    failing: Arc<AtomicBool>,
}

impl Faults {
    fn fault(&self, error: &Error) {
        if !self.failing.swap(true, Ordering::Relaxed) {
            (self.on_fault)(error);
        }
    }

    /// Ends a run of faults, so the next fault is told.
    fn clear(&self) {
        self.failing.store(false, Ordering::Relaxed);
    }
}

fn accept_request(
    stream: TcpStream,
    on_request: &dyn Fn(&Request<()>) -> Response<Body>,
    timeout: Option<Duration>,
) -> Result<()> {
    stream.set_read_timeout(timeout)?;
    stream.set_write_timeout(timeout)?;
    // One reader for the whole connection: what it reads past a request
    // is the start of the next, sent before the first was answered. It
    // shares the socket with the writer rather than holding a copy, which
    // would take a second file handle where none may be left.
    let mut reader = BufReader::with_capacity(BUFFER_CAPACITY, &stream);
    let mut connection_state = ConnectionState::KeepAlive;
    while connection_state == ConnectionState::KeepAlive {
        let (mut response, new_connection_state) = match decode_request_headers(&mut reader) {
            Ok(request) => {
                // Handles Expect header
                if let Some(expect) = request.headers_ref().unwrap().get(EXPECT).cloned() {
                    if request.version_ref().is_none_or(|v| *v >= Version::HTTP_11)
                        && expect.as_bytes().eq_ignore_ascii_case(b"100-continue")
                    {
                        (&stream).write_all(b"HTTP/1.1 100 Continue\r\n\r\n")?;
                        read_body_and_build_response(request, &mut reader, on_request)
                    } else {
                        (
                            build_text_response(
                                StatusCode::EXPECTATION_FAILED,
                                format!(
                                    "Expect header value '{}' is not supported.",
                                    String::from_utf8_lossy(expect.as_ref())
                                ),
                            ),
                            ConnectionState::Close,
                        )
                    }
                } else {
                    read_body_and_build_response(request, &mut reader, on_request)
                }
            }
            Err(error) => {
                if error.kind() == ErrorKind::ConnectionAborted {
                    return Ok(()); // The client is disconnected. Let's ignore this error and do not try to write an answer that won't be received.
                } else {
                    (build_error(error), ConnectionState::Close)
                }
            }
        };
        connection_state = new_connection_state;

        encode_response(
            &mut response,
            BufWriter::with_capacity(BUFFER_CAPACITY, &stream),
        )?
        .into_inner()
        .map_err(|e| e.into_error())?;
    }
    Ok(())
}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
enum ConnectionState {
    Close,
    KeepAlive,
}

fn read_body_and_build_response(
    request: RequestBuilder,
    reader: &mut impl BufRead,
    on_request: &dyn Fn(&Request<()>) -> Response<Body>,
) -> (Response<Body>, ConnectionState) {
    match skip_request_body(request, reader) {
        Ok(request) => {
            let response = on_request(&request);
            let connection_state = request
                .headers()
                .get(CONNECTION)
                .and_then(|v| {
                    v.as_bytes()
                        .eq_ignore_ascii_case(b"close")
                        .then_some(ConnectionState::Close)
                })
                .unwrap_or_else(|| {
                    if request.version() <= Version::HTTP_10 {
                        ConnectionState::Close
                    } else {
                        ConnectionState::KeepAlive
                    }
                });
            (response, connection_state)
        }
        Err(error) => (build_error(error), ConnectionState::Close),
    }
}

fn build_error(error: Error) -> Response<Body> {
    build_text_response(
        match error.kind() {
            ErrorKind::TimedOut => StatusCode::REQUEST_TIMEOUT,
            ErrorKind::InvalidData => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        },
        error.to_string(),
    )
}

fn build_text_response(status: StatusCode, text: String) -> Response<Body> {
    Response::builder()
        .status(status)
        .header(CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(Body::from(text))
        .unwrap()
}

/// Dumb semaphore allowing to overflow capacity
#[derive(Clone)]
struct Semaphore {
    inner: Arc<InnerSemaphore>,
}

struct InnerSemaphore {
    count: Mutex<usize>,
    capacity: usize,
    condvar: Condvar,
}

impl Semaphore {
    fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(InnerSemaphore {
                count: Mutex::new(0),
                capacity,
                condvar: Condvar::new(),
            }),
        }
    }

    fn lock(&self) -> SemaphoreGuard {
        let data = &self.inner;
        *data
            .condvar
            .wait_while(data.count.lock().unwrap(), |count| *count >= data.capacity)
            .unwrap() += 1;
        SemaphoreGuard {
            inner: Arc::clone(&self.inner),
        }
    }
}

struct SemaphoreGuard {
    inner: Arc<InnerSemaphore>,
}

impl Drop for SemaphoreGuard {
    fn drop(&mut self) {
        let data = &self.inner;
        *data.count.lock().unwrap() -= 1;
        data.condvar.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use std::net::Ipv4Addr;

    #[test]
    fn test_regular_http_operations() -> Result<()> {
        test_server(
            [
                "GET / HTTP/1.1\nhost: localhost:9999\n\n",
                "POST /foo HTTP/1.1\nhost: localhost:9999\nexpect: 100-continue\nconnection:close\ncontent-length:4\n\nabcd",
            ],
            [
                "HTTP/1.1 200 OK\r\ncontent-length: 4\r\n\r\nhome",
                "HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 404 Not Found\r\ncontent-length: 0\r\n\r\n",
            ],
        )
    }

    #[test]
    fn test_bad_request() -> Result<()> {
        test_server(
            ["GET / HTTP/1.1\nhost: localhost:9999\nfoo\n\n"],
            [
                "HTTP/1.1 400 Bad Request\r\ncontent-type: text/plain; charset=utf-8\r\ncontent-length: 19\r\n\r\ninvalid header name",
            ],
        )
    }

    #[test]
    fn test_bad_expect() -> Result<()> {
        test_server(
            ["GET / HTTP/1.1\nhost: localhost:9999\nexpect: bad\n\n"],
            [
                "HTTP/1.1 417 Expectation Failed\r\ncontent-type: text/plain; charset=utf-8\r\ncontent-length: 43\r\n\r\nExpect header value 'bad' is not supported.",
            ],
        )
    }

    fn free_port() -> Result<u16> {
        Ok(TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?
            .local_addr()?
            .port())
    }

    fn test_server(
        requests: impl IntoIterator<Item = &'static str>,
        responses: impl IntoIterator<Item = &'static str>,
    ) -> Result<()> {
        let mut stream = home_server()?;
        for (request, response) in requests.into_iter().zip(responses) {
            stream.write_all(request.as_bytes())?;
            let mut output = vec![b'\0'; response.len()];
            stream.read_exact(&mut output)?;
            assert_eq!(String::from_utf8(output).unwrap(), response);
        }
        Ok(())
    }

    /// A connection to a server answering `/` with "home" and anything
    /// else with nothing found.
    fn home_server() -> Result<TcpStream> {
        let server_port = free_port()?;
        Server::new(
            |request| {
                if request.uri().path() == "/" {
                    Response::builder().body(Body::from("home")).unwrap()
                } else {
                    Response::builder()
                        .status(StatusCode::NOT_FOUND)
                        .body(Body::empty())
                        .unwrap()
                }
            },
            |_| {},
        )
        .bind((Ipv4Addr::LOCALHOST, server_port))
        .with_global_timeout(Duration::from_secs(1))
        .spawn()?;
        TcpStream::connect((Ipv4Addr::LOCALHOST, server_port))
    }

    #[test]
    fn requests_sent_together_are_each_answered() -> Result<()> {
        let mut stream = home_server()?;
        stream.write_all(
            b"POST /foo HTTP/1.1\nhost: localhost:9999\ncontent-length: 4\n\nabcd\
              GET / HTTP/1.1\nhost: localhost:9999\n\n\
              GET / HTTP/1.1\nhost: localhost:9999\nconnection: close\n\n",
        )?;
        let mut output = String::new();
        stream.read_to_string(&mut output)?;
        assert_eq!(
            output,
            "HTTP/1.1 404 Not Found\r\ncontent-length: 0\r\n\r\n\
             HTTP/1.1 200 OK\r\ncontent-length: 4\r\n\r\nhome\
             HTTP/1.1 200 OK\r\ncontent-length: 4\r\n\r\nhome"
        );
        Ok(())
    }

    #[test]
    fn a_run_of_faults_is_told_once() {
        let told = Arc::new(Mutex::new(Vec::new()));
        let faults = Faults {
            on_fault: Arc::new({
                let told = told.clone();
                move |error: &Error| told.lock().unwrap().push(error.to_string())
            }),
            failing: Arc::new(AtomicBool::new(false)),
        };
        let other = faults.clone();
        faults.fault(&Error::other("first"));
        other.fault(&Error::other("second"));
        faults.fault(&Error::other("third"));
        other.clear();
        faults.fault(&Error::other("fourth"));
        assert_eq!(*told.lock().unwrap(), ["first", "fourth"]);
    }

    #[test]
    fn test_thread_limit() -> Result<()> {
        let server_port = free_port()?;
        let request = b"GET / HTTP/1.1\nhost: localhost:9999\n\n";
        let response = b"HTTP/1.1 200 OK\r\ncontent-length: 4\r\n\r\nhome";
        Server::new(
            |_| Response::builder().body(Body::from("home")).unwrap(),
            |_| {},
        )
        .bind((Ipv4Addr::LOCALHOST, server_port))
        .with_global_timeout(Duration::from_secs(1))
        .with_max_concurrent_connections(2)
        .spawn()?;
        let streams = (0..128)
            .map(|_| {
                let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, server_port))?;
                stream.write_all(request)?;
                Ok(stream)
            })
            .collect::<Result<Vec<_>>>()?;
        for mut stream in streams {
            let mut output = vec![b'\0'; response.len()];
            stream.read_exact(&mut output)?;
            assert_eq!(output, response);
        }
        Ok(())
    }
}
